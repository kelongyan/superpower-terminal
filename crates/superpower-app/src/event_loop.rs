use std::sync::Arc;
use std::time::{Duration, Instant};
use superpower_core::{
    cell_bounds, line_bounds, word_bounds, Color, MouseTrackingMode, Selection, SelectionPos,
    TerminalHandler,
};
use superpower_pty::{PtyEvent, PtySession};
use superpower_renderer::{Renderer, RendererOptions};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, Ime, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes};

/// 应用状态
struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    terminal: Option<TerminalHandler>,
    pty: Option<PtySession>,
    shift_pressed: bool,
    ctrl_pressed: bool,
    alt_pressed: bool,
    /// 鼠标选择进行中
    selecting: bool,
    /// 选择起始位置（行列）
    selection_start_cell: Option<(usize, usize)>,
    /// 选择锚点的完整边界，用于双击/三击拖拽扩展
    selection_anchor: Option<(SelectionPos, SelectionPos)>,
    /// 当前拖拽选择模式
    selection_drag_mode: SelectionDragMode,
    /// 最近一次鼠标所在单元格
    pointer_cell: Option<(usize, usize)>,
    /// 最近一次左键点击的时间
    last_click_time: Option<Instant>,
    /// 最近一次左键点击的单元格
    last_click_cell: Option<(usize, usize)>,
    /// 连续点击计数
    click_count: u8,
    /// 鼠标报告时最后一次上报的单元格
    last_reported_cell: Option<(usize, usize)>,
    /// 当前按下的鼠标按钮，用于拖拽上报
    pressed_mouse_button: Option<MouseButton>,
    /// shell 是否已经退出
    shell_exited: bool,
    /// shell 最近一次退出码
    shell_exit_code: Option<i32>,
    /// 单元格尺寸（从 renderer 缓存）
    cell_width: f32,
    cell_height: f32,
    /// 布局 padding（物理像素）
    padding_x: f32,
    padding_y: f32,
}

/// 选择拖拽模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectionDragMode {
    Char,
    Word,
    Line,
}

/// 鼠标上报事件种类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MouseReportKind {
    Press(MouseButton),
    Release(MouseButton),
    Motion(Option<MouseButton>),
    WheelUp,
    WheelDown,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            renderer: None,
            terminal: None,
            pty: None,
            shift_pressed: false,
            ctrl_pressed: false,
            alt_pressed: false,
            selecting: false,
            selection_start_cell: None,
            selection_anchor: None,
            selection_drag_mode: SelectionDragMode::Char,
            pointer_cell: None,
            last_click_time: None,
            last_click_cell: None,
            click_count: 0,
            last_reported_cell: None,
            pressed_mouse_button: None,
            shell_exited: false,
            shell_exit_code: None,
            cell_width: 0.0,
            cell_height: 0.0,
            padding_x: 0.0,
            padding_y: 0.0,
        }
    }

    /// 将像素坐标转换为终端行列
    fn pixel_to_cell(&self, x: f64, y: f64) -> Option<(usize, usize)> {
        if self.cell_width <= 0.0 || self.cell_height <= 0.0 {
            return None;
        }
        let local_x = (x as f32 - self.padding_x).max(0.0);
        let local_y = (y as f32 - self.padding_y).max(0.0);
        let col = (local_x / self.cell_width) as usize;
        let row = (local_y / self.cell_height) as usize;
        Some((row, col))
    }

    /// 根据连续点击次数推导当前选择模式
    fn drag_mode_from_clicks(click_count: u8) -> SelectionDragMode {
        match click_count {
            2 => SelectionDragMode::Word,
            3 => SelectionDragMode::Line,
            _ => SelectionDragMode::Char,
        }
    }

    /// 计算某个单元格在当前拖拽模式下的语义边界
    fn semantic_bounds(
        terminal: &TerminalHandler,
        row: usize,
        col: usize,
        mode: SelectionDragMode,
    ) -> Option<(SelectionPos, SelectionPos)> {
        match mode {
            SelectionDragMode::Char => cell_bounds(&terminal.terminal.grid, row, col),
            SelectionDragMode::Word => word_bounds(&terminal.terminal.grid, row, col),
            SelectionDragMode::Line => line_bounds(&terminal.terminal.grid, row),
        }
    }

    /// 从锚点与当前鼠标位置重建选区
    fn build_selection_for_drag(
        terminal: &TerminalHandler,
        anchor: (SelectionPos, SelectionPos),
        mode: SelectionDragMode,
        row: usize,
        col: usize,
    ) -> Option<Selection> {
        let (anchor_start, anchor_end) = anchor;
        let (current_start, current_end) = Self::semantic_bounds(terminal, row, col, mode)?;

        // 当向前反向拖拽时，需要保留锚点的完整尾部；向后拖拽时则保留锚点起点。
        Some(if current_start < anchor_start {
            Selection::new(current_start, anchor_end)
        } else {
            Selection::new(anchor_start, current_end)
        })
    }

    /// 当前是否应将鼠标事件交给终端程序，而不是本地选择逻辑
    fn should_report_mouse(&self) -> bool {
        self.terminal.as_ref().is_some_and(|terminal| {
            terminal.terminal.mouse_tracking_mode() != MouseTrackingMode::Disabled
        }) && !self.shift_pressed
    }

    /// 统一向 PTY 写入输入数据，并在写入前重置视口
    fn write_input(&mut self, bytes: &[u8]) {
        if self.shell_exited || bytes.is_empty() {
            return;
        }

        if let Some(terminal) = &mut self.terminal {
            terminal.terminal.grid.reset_display_offset();
            terminal.terminal.damage.mark_full_redraw();
        }

        if let Some(pty) = &mut self.pty {
            if let Err(err) = pty.write(bytes) {
                tracing::warn!("Failed to write input to PTY: {}", err);
            }
        }
    }

    /// 处理 shell 退出后的状态与提示
    fn handle_shell_exit(&mut self, code: i32) {
        if self.shell_exited {
            return;
        }

        self.shell_exited = true;
        self.shell_exit_code = Some(code);
        self.selecting = false;
        self.selection_start_cell = None;
        self.selection_anchor = None;
        self.pressed_mouse_button = None;
        self.last_reported_cell = None;

        if let Some(window) = &self.window {
            window.set_title(&format!("SuperPower Terminal - Shell exited ({})", code));
        }

        if let Some(terminal) = &mut self.terminal {
            let message = format!("\r\n[SuperPower] shell exited with code {}\r\n", code);
            terminal.process(message.as_bytes());
            terminal.terminal.damage.mark_full_redraw();
        }
    }

    /// 将当前终端光标同步给 IME，避免候选框位置漂移
    fn update_ime_cursor_area(&self) {
        let (Some(window), Some(terminal)) = (&self.window, &self.terminal) else {
            return;
        };

        let x = self.padding_x + terminal.terminal.cursor.col as f32 * self.cell_width;
        let y = self.padding_y + terminal.terminal.cursor.row as f32 * self.cell_height;
        window.set_ime_cursor_area(
            winit::dpi::PhysicalPosition::new(x.round() as i32, y.round() as i32),
            winit::dpi::PhysicalSize::new(
                self.cell_width.ceil().max(1.0) as u32,
                self.cell_height.ceil().max(1.0) as u32,
            ),
        );
    }

    /// 上报鼠标事件给终端程序
    fn report_mouse(&mut self, kind: MouseReportKind, row: usize, col: usize) {
        let Some((mode, sgr)) = self.terminal.as_ref().map(|terminal| {
            (
                terminal.terminal.mouse_tracking_mode(),
                terminal.terminal.mouse_sgr_mode(),
            )
        }) else {
            return;
        };
        let encoded = encode_mouse_report(
            kind,
            row,
            col,
            sgr,
            self.shift_pressed,
            self.alt_pressed,
            self.ctrl_pressed,
        );

        if let Some(bytes) = encoded {
            self.write_input(&bytes);
        }

        match kind {
            MouseReportKind::Motion(_) => self.last_reported_cell = Some((row, col)),
            MouseReportKind::Press(button) => {
                self.pressed_mouse_button = Some(button);
                self.last_reported_cell = Some((row, col));
            }
            MouseReportKind::Release(_) => {
                self.pressed_mouse_button = None;
                self.last_reported_cell = Some((row, col));
            }
            MouseReportKind::WheelUp | MouseReportKind::WheelDown => {
                self.last_reported_cell = Some((row, col));
            }
        }

        if mode == MouseTrackingMode::Disabled {
            self.last_reported_cell = None;
        }
    }

    /// 复制选区文本到剪贴板
    fn copy_selection(&mut self) {
        let terminal = match &self.terminal {
            Some(t) => t,
            None => return,
        };

        let selection = match &terminal.terminal.selection {
            Some(s) => s,
            None => return,
        };

        let text = selection.text(&terminal.terminal.grid);

        if text.is_empty() {
            return;
        }

        match arboard::Clipboard::new() {
            Ok(mut clipboard) => {
                if let Err(e) = clipboard.set_text(&text) {
                    tracing::warn!("Failed to copy to clipboard: {}", e);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to access clipboard: {}", e);
            }
        }
    }

    /// 粘贴剪贴板文本
    fn paste_clipboard(&mut self) {
        let text = match arboard::Clipboard::new() {
            Ok(mut clipboard) => clipboard.get_text().unwrap_or_default(),
            Err(_) => return,
        };

        if text.is_empty() {
            return;
        }

        let payload = if let Some(terminal) = &self.terminal {
            if terminal.terminal.bracketed_paste_mode() {
                encode_bracketed_paste(&text)
            } else {
                text.into_bytes()
            }
        } else {
            text.into_bytes()
        };

        self.write_input(&payload);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let config = crate::config::Config::load_from_file(&crate::config::Config::config_path());
        let attrs = WindowAttributes::default()
            .with_title("SuperPower Terminal")
            .with_inner_size(winit::dpi::LogicalSize::new(
                config.window.width,
                config.window.height,
            ));
        // 配置颜色解析失败时回退到终端内置默认值，避免启动时中断。
        let default_foreground = crate::config::Config::parse_color(&config.colors.foreground)
            .unwrap_or_else(|| {
                tracing::warn!(
                    "Invalid foreground color '{}', using default",
                    config.colors.foreground
                );
                Color::DEFAULT_FG
            });
        let default_background = crate::config::Config::parse_color(&config.colors.background)
            .unwrap_or_else(|| {
                tracing::warn!(
                    "Invalid background color '{}', using default",
                    config.colors.background
                );
                Color::DEFAULT_BG
            });

        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("Failed to create window"),
        );
        window.set_ime_allowed(true);

        let renderer = pollster::block_on(Renderer::new(
            Arc::clone(&window),
            RendererOptions {
                font_family: config.font.family.clone(),
                font_size: config.font.size,
                default_foreground,
                default_background,
                padding_x: config.window.padding_x,
                padding_y: config.window.padding_y,
            },
        ));
        let (rows, cols) = renderer.terminal_size();
        self.cell_width = renderer.cell_width();
        self.cell_height = renderer.cell_height();
        self.padding_x = renderer.padding_x();
        self.padding_y = renderer.padding_y();
        let terminal = TerminalHandler::with_theme(
            rows,
            cols,
            config.scrollback.limit,
            default_foreground,
            default_background,
        );
        let pty = PtySession::new(
            cols as u16,
            rows as u16,
            &config.shell.program,
            &config.shell.args,
        )
        .expect("Failed to create PTY session");

        self.window = Some(window);
        self.renderer = Some(renderer);
        self.terminal = Some(terminal);
        self.pty = Some(pty);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::Resized(physical_size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(physical_size);
                }
                if let (Some(renderer), Some(terminal), Some(pty)) =
                    (&self.renderer, &mut self.terminal, &mut self.pty)
                {
                    let (new_rows, new_cols) = renderer.terminal_size();
                    terminal.resize(new_rows, new_cols);
                    let _ = pty.resize(new_cols as u16, new_rows as u16);
                    // 更新缓存的单元格尺寸
                    self.cell_width = renderer.cell_width();
                    self.cell_height = renderer.cell_height();
                    self.padding_x = renderer.padding_x();
                    self.padding_y = renderer.padding_y();
                }
            }

            WindowEvent::ScaleFactorChanged {
                scale_factor,
                inner_size_writer: _,
            } => {
                tracing::info!("DPI scale factor changed to {}", scale_factor);
                if let Some(renderer) = &mut self.renderer {
                    renderer.update_font_metrics(scale_factor);
                    self.cell_width = renderer.cell_width();
                    self.cell_height = renderer.cell_height();
                    self.padding_x = renderer.padding_x();
                    self.padding_y = renderer.padding_y();
                }
                if let (Some(renderer), Some(terminal), Some(pty)) =
                    (&self.renderer, &mut self.terminal, &mut self.pty)
                {
                    let (new_rows, new_cols) = renderer.terminal_size();
                    terminal.resize(new_rows, new_cols);
                    let _ = pty.resize(new_cols as u16, new_rows as u16);
                }
            }

            WindowEvent::RedrawRequested => {
                let mut exit_code = None;
                if let (Some(pty), Some(terminal)) = (&mut self.pty, &mut self.terminal) {
                    while let Ok(event) = pty.rx.try_recv() {
                        match event {
                            PtyEvent::Data(data) => {
                                terminal.process(&data);
                            }
                            PtyEvent::Exit(code) => {
                                tracing::info!("Shell exited");
                                exit_code = Some(code);
                            }
                        }
                    }
                }

                if let Some(code) = exit_code {
                    self.handle_shell_exit(code);
                }

                if let (Some(renderer), Some(terminal)) = (&mut self.renderer, &mut self.terminal) {
                    if renderer.needs_render(terminal) {
                        match renderer.render(terminal) {
                            Ok(_) => {}
                            Err(wgpu::SurfaceError::Lost) => {}
                            Err(wgpu::SurfaceError::OutOfMemory) => {
                                event_loop.exit();
                            }
                            Err(e) => {
                                tracing::error!("Render error: {:?}", e);
                            }
                        }
                    }
                }

                if let Some(terminal) = &mut self.terminal {
                    terminal.terminal.damage.clear();
                }

                self.update_ime_cursor_area();

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                // 先处理复制/粘贴快捷键
                if self.ctrl_pressed && self.shift_pressed {
                    use winit::keyboard::{KeyCode, PhysicalKey};
                    if let PhysicalKey::Code(keycode) = event.physical_key {
                        if keycode == KeyCode::KeyC {
                            self.copy_selection();
                            return;
                        }
                        if keycode == KeyCode::KeyV {
                            self.paste_clipboard();
                            return;
                        }
                    }
                }

                // 其他键盘输入
                if let Some(terminal) = &mut self.terminal {
                    let payload = handle_key_input(
                        event,
                        terminal,
                        self.shift_pressed,
                        self.ctrl_pressed,
                        self.alt_pressed,
                    );
                    if let Some(bytes) = payload {
                        self.write_input(&bytes);
                    }
                }
            }

            WindowEvent::ModifiersChanged(modifiers) => {
                self.shift_pressed = modifiers.state().shift_key();
                self.ctrl_pressed = modifiers.state().control_key();
                self.alt_pressed = modifiers.state().alt_key();
            }

            WindowEvent::Ime(Ime::Commit(text)) if !text.is_empty() => {
                self.write_input(text.as_bytes());
            }

            WindowEvent::MouseWheel { delta, .. } => {
                if self.should_report_mouse() {
                    if let Some((row, col)) = self.pointer_cell {
                        let steps = match delta {
                            winit::event::MouseScrollDelta::LineDelta(_, y) => y.round() as isize,
                            winit::event::MouseScrollDelta::PixelDelta(pos) => {
                                (pos.y / 20.0).round() as isize
                            }
                        };
                        if steps > 0 {
                            for _ in 0..steps {
                                self.report_mouse(MouseReportKind::WheelUp, row, col);
                            }
                        } else if steps < 0 {
                            for _ in 0..(-steps) {
                                self.report_mouse(MouseReportKind::WheelDown, row, col);
                            }
                        }
                    }
                } else if let Some(terminal) = &mut self.terminal {
                    let lines = match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, y) => (-y * 3.0) as isize,
                        winit::event::MouseScrollDelta::PixelDelta(pos) => (-pos.y / 20.0) as isize,
                    };
                    if lines > 0 {
                        terminal.terminal.grid.scroll_display_up(lines as usize);
                        terminal.terminal.damage.mark_full_redraw();
                    } else if lines < 0 {
                        terminal
                            .terminal
                            .grid
                            .scroll_display_down((-lines) as usize);
                        terminal.terminal.damage.mark_full_redraw();
                    }
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if self.should_report_mouse() {
                    if let Some((row, col)) = self.pointer_cell {
                        match (button, state) {
                            (
                                MouseButton::Left | MouseButton::Middle | MouseButton::Right,
                                ElementState::Pressed,
                            ) => {
                                self.report_mouse(MouseReportKind::Press(button), row, col);
                            }
                            (
                                MouseButton::Left | MouseButton::Middle | MouseButton::Right,
                                ElementState::Released,
                            ) => {
                                let release_button = self.pressed_mouse_button.unwrap_or(button);
                                self.report_mouse(
                                    MouseReportKind::Release(release_button),
                                    row,
                                    col,
                                );
                            }
                            _ => {}
                        }
                    }
                    return;
                }

                match (button, state) {
                    (MouseButton::Left, ElementState::Pressed) => {
                        let Some((row, col)) = self.pointer_cell else {
                            return;
                        };

                        let now = Instant::now();
                        let double_click_timeout = Duration::from_millis(450);
                        if self.last_click_cell == Some((row, col))
                            && self.last_click_time.is_some_and(|last| {
                                now.duration_since(last) <= double_click_timeout
                            })
                        {
                            self.click_count = if self.click_count >= 3 {
                                1
                            } else {
                                self.click_count + 1
                            };
                        } else {
                            self.click_count = 1;
                        }
                        self.last_click_time = Some(now);
                        self.last_click_cell = Some((row, col));
                        self.selection_drag_mode = Self::drag_mode_from_clicks(self.click_count);
                        self.selecting = true;
                        self.selection_start_cell = Some((row, col));

                        if let Some(terminal) = &mut self.terminal {
                            self.selection_anchor =
                                Self::semantic_bounds(terminal, row, col, self.selection_drag_mode);

                            if self.selection_drag_mode == SelectionDragMode::Char {
                                terminal.terminal.selection = None;
                                terminal.terminal.damage.mark_full_redraw();
                            } else if let Some(anchor) = self.selection_anchor {
                                if let Some(selection) = Self::build_selection_for_drag(
                                    terminal,
                                    anchor,
                                    self.selection_drag_mode,
                                    row,
                                    col,
                                ) {
                                    terminal.terminal.selection = Some(selection);
                                    terminal.terminal.damage.mark_full_redraw();
                                }
                            }
                        }
                    }
                    (MouseButton::Left, ElementState::Released) => {
                        self.selecting = false;
                        self.selection_start_cell = None;
                        self.selection_anchor = None;
                    }
                    (MouseButton::Right, ElementState::Released) => {
                        // 右键粘贴
                        self.paste_clipboard();
                    }
                    (MouseButton::Middle, ElementState::Released) => {
                        // 中键粘贴
                        self.paste_clipboard();
                    }
                    _ => {}
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.pointer_cell = self.pixel_to_cell(position.x, position.y);
                if self.should_report_mouse() {
                    let Some((row, col)) = self.pointer_cell else {
                        return;
                    };
                    let Some(terminal) = &self.terminal else {
                        return;
                    };

                    let mode = terminal.terminal.mouse_tracking_mode();
                    let should_report_motion = match mode {
                        MouseTrackingMode::Disabled | MouseTrackingMode::Normal => false,
                        MouseTrackingMode::ButtonEvent => self.pressed_mouse_button.is_some(),
                        MouseTrackingMode::AnyEvent => true,
                    };

                    if should_report_motion && self.last_reported_cell != Some((row, col)) {
                        self.report_mouse(
                            MouseReportKind::Motion(self.pressed_mouse_button),
                            row,
                            col,
                        );
                    }
                    return;
                }

                if !self.selecting {
                    return;
                }

                let Some((row, col)) = self.pointer_cell else {
                    return;
                };

                let Some(terminal) = &mut self.terminal else {
                    return;
                };

                let grid_rows = terminal.terminal.grid.rows();
                let grid_cols = terminal.terminal.grid.cols();
                let row = row.min(grid_rows - 1);
                let col = col.min(grid_cols - 1);

                if self.selection_start_cell.is_none() {
                    self.selection_start_cell = Some((row, col));
                }

                if let Some(anchor) = self.selection_anchor {
                    if let Some(selection) = Self::build_selection_for_drag(
                        terminal,
                        anchor,
                        self.selection_drag_mode,
                        row,
                        col,
                    ) {
                        terminal.terminal.selection = Some(selection);
                        terminal.terminal.damage.mark_full_redraw();
                    }
                }
            }

            _ => {}
        }
    }
}

/// 处理键盘输入
fn handle_key_input(
    event: winit::event::KeyEvent,
    terminal: &mut TerminalHandler,
    shift_pressed: bool,
    ctrl_pressed: bool,
    alt_pressed: bool,
) -> Option<Vec<u8>> {
    use winit::keyboard::{KeyCode, PhysicalKey};

    let PhysicalKey::Code(keycode) = event.physical_key else {
        return None;
    };

    // Shift+PageUp/PageDown/Home/End 滚动
    if shift_pressed {
        match keycode {
            KeyCode::PageUp => {
                terminal
                    .terminal
                    .grid
                    .scroll_display_up(terminal.terminal.grid.rows());
                terminal.terminal.damage.mark_full_redraw();
                return None;
            }
            KeyCode::PageDown => {
                terminal
                    .terminal
                    .grid
                    .scroll_display_down(terminal.terminal.grid.rows());
                terminal.terminal.damage.mark_full_redraw();
                return None;
            }
            KeyCode::Home => {
                let max = terminal.terminal.grid.scrollback_len();
                terminal.terminal.grid.scroll_display_up(max);
                terminal.terminal.damage.mark_full_redraw();
                return None;
            }
            KeyCode::End => {
                terminal.terminal.grid.reset_display_offset();
                terminal.terminal.damage.mark_full_redraw();
                return None;
            }
            _ => {}
        }
    }

    let application_cursor_keys = terminal.terminal.application_cursor_keys();
    let mut prefix_alt_for_special_keys = alt_pressed;

    let bytes: Vec<u8> = match keycode {
        KeyCode::Enter => vec![0x0D],
        KeyCode::Backspace => vec![0x08],
        KeyCode::Tab if shift_pressed => vec![0x1B, b'[', b'Z'],
        KeyCode::Tab => vec![0x09],
        KeyCode::Escape => vec![0x1B],
        KeyCode::ArrowUp if application_cursor_keys => vec![0x1B, b'O', b'A'],
        KeyCode::ArrowDown if application_cursor_keys => vec![0x1B, b'O', b'B'],
        KeyCode::ArrowRight if application_cursor_keys => vec![0x1B, b'O', b'C'],
        KeyCode::ArrowLeft if application_cursor_keys => vec![0x1B, b'O', b'D'],
        KeyCode::ArrowUp => vec![0x1B, b'[', b'A'],
        KeyCode::ArrowDown => vec![0x1B, b'[', b'B'],
        KeyCode::ArrowRight => vec![0x1B, b'[', b'C'],
        KeyCode::ArrowLeft => vec![0x1B, b'[', b'D'],
        KeyCode::Home if application_cursor_keys => vec![0x1B, b'O', b'H'],
        KeyCode::End if application_cursor_keys => vec![0x1B, b'O', b'F'],
        KeyCode::Home => vec![0x1B, b'[', b'H'],
        KeyCode::End => vec![0x1B, b'[', b'F'],
        KeyCode::Delete => vec![0x1B, b'[', b'3', b'~'],
        KeyCode::PageUp => vec![0x1B, b'[', b'5', b'~'],
        KeyCode::PageDown => vec![0x1B, b'[', b'6', b'~'],
        KeyCode::Insert => vec![0x1B, b'[', b'2', b'~'],
        KeyCode::F1 => vec![0x1B, b'O', b'P'],
        KeyCode::F2 => vec![0x1B, b'O', b'Q'],
        KeyCode::F3 => vec![0x1B, b'O', b'R'],
        KeyCode::F4 => vec![0x1B, b'O', b'S'],
        KeyCode::F5 => vec![0x1B, b'[', b'1', b'5', b'~'],
        KeyCode::F6 => vec![0x1B, b'[', b'1', b'7', b'~'],
        KeyCode::F7 => vec![0x1B, b'[', b'1', b'8', b'~'],
        KeyCode::F8 => vec![0x1B, b'[', b'1', b'9', b'~'],
        KeyCode::F9 => vec![0x1B, b'[', b'2', b'0', b'~'],
        KeyCode::F10 => vec![0x1B, b'[', b'2', b'1', b'~'],
        KeyCode::F11 => vec![0x1B, b'[', b'2', b'3', b'~'],
        KeyCode::F12 => vec![0x1B, b'[', b'2', b'4', b'~'],
        KeyCode::Numpad0 if terminal.terminal.keypad_application_mode() => vec![0x1B, b'O', b'p'],
        KeyCode::Numpad1 if terminal.terminal.keypad_application_mode() => vec![0x1B, b'O', b'q'],
        KeyCode::Numpad2 if terminal.terminal.keypad_application_mode() => vec![0x1B, b'O', b'r'],
        KeyCode::Numpad3 if terminal.terminal.keypad_application_mode() => vec![0x1B, b'O', b's'],
        KeyCode::Numpad4 if terminal.terminal.keypad_application_mode() => vec![0x1B, b'O', b't'],
        KeyCode::Numpad5 if terminal.terminal.keypad_application_mode() => vec![0x1B, b'O', b'u'],
        KeyCode::Numpad6 if terminal.terminal.keypad_application_mode() => vec![0x1B, b'O', b'v'],
        KeyCode::Numpad7 if terminal.terminal.keypad_application_mode() => vec![0x1B, b'O', b'w'],
        KeyCode::Numpad8 if terminal.terminal.keypad_application_mode() => vec![0x1B, b'O', b'x'],
        KeyCode::Numpad9 if terminal.terminal.keypad_application_mode() => vec![0x1B, b'O', b'y'],
        KeyCode::NumpadAdd if terminal.terminal.keypad_application_mode() => vec![0x1B, b'O', b'k'],
        KeyCode::NumpadSubtract if terminal.terminal.keypad_application_mode() => {
            vec![0x1B, b'O', b'm']
        }
        KeyCode::NumpadMultiply if terminal.terminal.keypad_application_mode() => {
            vec![0x1B, b'O', b'j']
        }
        KeyCode::NumpadDivide if terminal.terminal.keypad_application_mode() => {
            vec![0x1B, b'O', b'o']
        }
        KeyCode::NumpadEnter if terminal.terminal.keypad_application_mode() => {
            vec![0x1B, b'O', b'M']
        }
        KeyCode::NumpadDecimal if terminal.terminal.keypad_application_mode() => {
            vec![0x1B, b'O', b'n']
        }
        _ => {
            if let Some(bytes) = encode_text_input(&event, keycode, ctrl_pressed, alt_pressed) {
                prefix_alt_for_special_keys = false;
                bytes
            } else {
                return None;
            }
        }
    };

    let bytes = if prefix_alt_for_special_keys {
        let mut prefixed = Vec::with_capacity(bytes.len() + 1);
        prefixed.push(0x1B);
        prefixed.extend_from_slice(&bytes);
        prefixed
    } else {
        bytes
    };

    Some(bytes)
}

/// 将普通文本键、Ctrl 组合键和 Alt 组合键编码为终端输入序列
fn encode_text_input(
    event: &winit::event::KeyEvent,
    keycode: winit::keyboard::KeyCode,
    ctrl_pressed: bool,
    alt_pressed: bool,
) -> Option<Vec<u8>> {
    use winit::keyboard::KeyCode;

    // Windows 的 AltGr 往往表现为 Ctrl+Alt，同时仍携带可打印文本。
    // 这种情况下应优先把它当作普通文本，而不是 Ctrl 控制序列。
    if ctrl_pressed && alt_pressed {
        if let Some(text) = event.text.as_ref() {
            if !text.is_empty() {
                return Some(text.as_bytes().to_vec());
            }
        }
    }

    if ctrl_pressed {
        let control = match keycode {
            KeyCode::KeyA => 0x01,
            KeyCode::KeyB => 0x02,
            KeyCode::KeyC => 0x03,
            KeyCode::KeyD => 0x04,
            KeyCode::KeyE => 0x05,
            KeyCode::KeyF => 0x06,
            KeyCode::KeyG => 0x07,
            KeyCode::KeyH => 0x08,
            KeyCode::KeyI => 0x09,
            KeyCode::KeyJ => 0x0A,
            KeyCode::KeyK => 0x0B,
            KeyCode::KeyL => 0x0C,
            KeyCode::KeyM => 0x0D,
            KeyCode::KeyN => 0x0E,
            KeyCode::KeyO => 0x0F,
            KeyCode::KeyP => 0x10,
            KeyCode::KeyQ => 0x11,
            KeyCode::KeyR => 0x12,
            KeyCode::KeyS => 0x13,
            KeyCode::KeyT => 0x14,
            KeyCode::KeyU => 0x15,
            KeyCode::KeyV => 0x16,
            KeyCode::KeyW => 0x17,
            KeyCode::KeyX => 0x18,
            KeyCode::KeyY => 0x19,
            KeyCode::KeyZ => 0x1A,
            KeyCode::BracketLeft => 0x1B,
            KeyCode::Backslash => 0x1C,
            KeyCode::BracketRight => 0x1D,
            KeyCode::Digit6 => 0x1E,
            KeyCode::Minus | KeyCode::Slash => 0x1F,
            KeyCode::Space => 0x00,
            _ => return None,
        };
        return Some(vec![control]);
    }

    let text = event.text.as_ref()?;
    if text.is_empty() {
        return None;
    }

    if alt_pressed {
        let mut bytes = Vec::with_capacity(text.len() + 1);
        bytes.push(0x1B);
        bytes.extend_from_slice(text.as_bytes());
        return Some(bytes);
    }

    Some(text.as_bytes().to_vec())
}

/// 将普通粘贴文本包装为 bracketed paste 协议
fn encode_bracketed_paste(text: &str) -> Vec<u8> {
    let mut payload = Vec::with_capacity(text.len() + 12);
    payload.extend_from_slice(b"\x1b[200~");
    payload.extend_from_slice(text.as_bytes());
    payload.extend_from_slice(b"\x1b[201~");
    payload
}

/// 将鼠标事件编码为终端协议
fn encode_mouse_report(
    kind: MouseReportKind,
    row: usize,
    col: usize,
    sgr_mode: bool,
    shift_pressed: bool,
    alt_pressed: bool,
    ctrl_pressed: bool,
) -> Option<Vec<u8>> {
    let modifiers = (if shift_pressed { 4 } else { 0 })
        + (if alt_pressed { 8 } else { 0 })
        + (if ctrl_pressed { 16 } else { 0 });
    let base = match kind {
        MouseReportKind::Press(button) => button_code(button)?,
        MouseReportKind::Release(button) => button_code(button)?,
        MouseReportKind::Motion(button) => {
            let base_button = match button {
                Some(button) => button_code(button)?,
                None => 3,
            };
            base_button + 32
        }
        MouseReportKind::WheelUp => 64,
        MouseReportKind::WheelDown => 65,
    } + modifiers;

    let x = col + 1;
    let y = row + 1;

    if sgr_mode {
        let final_char = match kind {
            MouseReportKind::Release(_) => 'm',
            _ => 'M',
        };
        return Some(format!("\x1b[<{};{};{}{}", base, x, y, final_char).into_bytes());
    }

    // 传统 X10 编码坐标范围较小，超出时直接截断到可表示范围。
    let encoded_x = (x.min(223) + 32) as u8;
    let encoded_y = (y.min(223) + 32) as u8;
    let encoded_button = match kind {
        MouseReportKind::Release(_) => 3 + modifiers,
        _ => base,
    } + 32;

    Some(vec![
        0x1B,
        b'[',
        b'M',
        encoded_button as u8,
        encoded_x,
        encoded_y,
    ])
}

/// 将 winit 的鼠标按钮映射到终端鼠标协议按钮码
fn button_code(button: MouseButton) -> Option<usize> {
    match button {
        MouseButton::Left => Some(0),
        MouseButton::Middle => Some(1),
        MouseButton::Right => Some(2),
        _ => None,
    }
}

/// 主入口
pub fn run() {
    let event_loop = winit::event_loop::EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let mut app = App::new();
    event_loop.run_app(&mut app).expect("Event loop error");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证 bracketed paste 会正确包裹控制序列
    #[test]
    fn bracketed_paste_wraps_payload() {
        let encoded = encode_bracketed_paste("hello");
        assert_eq!(encoded, b"\x1b[200~hello\x1b[201~");
    }

    /// 验证 SGR 鼠标编码格式正确
    #[test]
    fn encode_sgr_mouse_press() {
        let encoded = encode_mouse_report(
            MouseReportKind::Press(MouseButton::Left),
            4,
            9,
            true,
            false,
            false,
            false,
        )
        .unwrap();
        assert_eq!(encoded, b"\x1b[<0;10;5M");
    }

    /// 验证传统鼠标编码至少会产出完整 ESC 序列
    #[test]
    fn encode_legacy_mouse_release() {
        let encoded = encode_mouse_report(
            MouseReportKind::Release(MouseButton::Left),
            0,
            0,
            false,
            false,
            false,
            false,
        )
        .unwrap();
        assert_eq!(encoded.len(), 6);
        assert_eq!(&encoded[..3], b"\x1b[M");
    }
}

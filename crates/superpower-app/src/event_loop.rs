use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use superpower_core::{
    cell_bounds, char_width, line_bounds, word_bounds, MouseTrackingMode, Selection, SelectionPos,
    TerminalHandler,
};
use superpower_pty::{PtyEvent, PtySession};
use superpower_renderer::{Renderer, RendererOptions};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, Ime, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes};

use crate::config::Config;
use crate::shortcuts::{ShortcutAction, ShortcutManager};
use crate::ui::{
    build_ui_model, AppTheme, StatusView, TabView, ThemePreset, UiAction, UiBuildState, UiModel,
};

/// 配置文件检查间隔（秒）
const CONFIG_CHECK_INTERVAL: Duration = Duration::from_secs(2);

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

/// 当前键盘焦点所在区域
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusArea {
    Terminal,
    Chrome,
}

/// 单个终端标签页状态
struct TerminalTab {
    title: String,
    terminal: TerminalHandler,
    pty: PtySession,
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
}

impl TerminalTab {
    /// 创建一个新的终端标签页
    fn new(
        title: String,
        rows: usize,
        cols: usize,
        config: &Config,
        theme: &AppTheme,
    ) -> Result<Self, String> {
        let mut terminal = TerminalHandler::with_theme(
            rows,
            cols,
            config.scrollback.limit,
            theme.terminal_foreground,
            theme.terminal_background,
        );
        // 在 shell prompt 到来前先写入一段欢迎信息，让首屏更像完整产品而不是空白终端。
        terminal.process(startup_banner(shell_label(config.shell.program.as_str())).as_bytes());
        terminal.terminal.damage.mark_full_redraw();
        let pty = PtySession::new(
            cols as u16,
            rows as u16,
            &config.shell.program,
            &config.shell.args,
        )?;

        Ok(Self {
            title,
            terminal,
            pty,
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
        })
    }

    /// 返回用于 UI 展示的标题
    fn view(&self, is_active: bool) -> TabView {
        TabView {
            title: self.title.clone(),
            is_active,
            is_exited: self.shell_exited,
        }
    }
}

/// 应用状态
struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    config: Config,
    theme: AppTheme,
    tabs: Vec<TerminalTab>,
    active_tab: usize,
    settings_open: bool,
    focus_area: FocusArea,
    ui_model: UiModel,
    ui_dirty: bool,
    shift_pressed: bool,
    ctrl_pressed: bool,
    alt_pressed: bool,
    /// 当前窗口内最后一次鼠标位置
    last_cursor_position: Option<(f64, f64)>,
    /// 单元格尺寸（从 renderer 缓存）
    cell_width: f32,
    cell_height: f32,
    /// 布局 padding（物理像素）
    padding_x: f32,
    padding_y: f32,
    /// 快捷键管理器
    shortcut_manager: ShortcutManager,
    /// 初始字号（用于重置）
    initial_font_size: f32,
    /// 配置文件路径
    config_path: std::path::PathBuf,
    /// 上次配置文件修改时间
    last_config_mtime: Option<std::time::SystemTime>,
    /// 上次检查配置文件的时间
    last_config_check: Instant,
}

impl App {
    /// 创建应用实例，并预先加载配置
    fn new() -> Self {
        let config_path = crate::config::Config::config_path();
        let config = crate::config::Config::load_from_file(&config_path);
        let theme = AppTheme::from_preset(ThemePreset::GraphiteDark);
        let shortcut_manager = if config.shortcuts.is_empty() {
            ShortcutManager::default()
        } else {
            ShortcutManager::from_config(&config.shortcuts)
        };
        let initial_font_size = config.font.size;
        let last_config_mtime = std::fs::metadata(&config_path)
            .and_then(|m| m.modified())
            .ok();
        let ui_model = build_ui_model(&UiBuildState {
            window_width: config.window.width as f32,
            window_height: config.window.height as f32,
            theme,
            settings_open: false,
            tabs: Vec::new(),
            active_tab: 0,
            font_size: config.font.size,
            status: StatusView {
                shell_label: shell_label(config.shell.program.as_str()),
                terminal_cols: 0,
                terminal_rows: 0,
                is_scrolled: false,
                theme_name: theme.name().to_string(),
                exit_code: None,
            },
        });

        Self {
            window: None,
            renderer: None,
            config,
            theme,
            tabs: Vec::new(),
            active_tab: 0,
            settings_open: false,
            focus_area: FocusArea::Terminal,
            ui_model,
            ui_dirty: true,
            shift_pressed: false,
            ctrl_pressed: false,
            alt_pressed: false,
            last_cursor_position: None,
            cell_width: 0.0,
            cell_height: 0.0,
            padding_x: 0.0,
            padding_y: 0.0,
            shortcut_manager,
            initial_font_size,
            config_path,
            last_config_mtime,
            last_config_check: Instant::now(),
        }
    }

    /// 获取当前活动标签页
    fn active_tab(&self) -> Option<&TerminalTab> {
        self.tabs.get(self.active_tab)
    }

    /// 获取当前活动标签页的可变引用
    fn active_tab_mut(&mut self) -> Option<&mut TerminalTab> {
        self.tabs.get_mut(self.active_tab)
    }

    /// 主动请求窗口重绘，避免首帧或状态切换后停留在系统默认白屏
    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// 同步 renderer 缓存出来的像素度量
    fn update_cached_metrics(&mut self) {
        if let Some(renderer) = &self.renderer {
            self.cell_width = renderer.cell_width();
            self.cell_height = renderer.cell_height();
            self.padding_x = renderer.padding_x();
            self.padding_y = renderer.padding_y();
        }
    }

    /// 构建一份当前所有标签页的 UI 摘要
    fn tab_views(&self) -> Vec<TabView> {
        self.tabs
            .iter()
            .enumerate()
            .map(|(index, tab)| tab.view(index == self.active_tab))
            .collect()
    }

    /// 构建当前状态栏信息
    fn status_view(&self, rows: usize, cols: usize) -> StatusView {
        let exit_code = self.active_tab().and_then(|tab| tab.shell_exit_code);
        let is_scrolled = self
            .active_tab()
            .is_some_and(|tab| tab.terminal.terminal.grid.is_scrolled());

        StatusView {
            shell_label: shell_label(self.config.shell.program.as_str()),
            terminal_cols: cols,
            terminal_rows: rows,
            is_scrolled,
            theme_name: self.theme.name().to_string(),
            exit_code,
        }
    }

    /// 统一重建 UI 布局，并在必要时同步终端行列尺寸
    fn refresh_ui_model(&mut self) {
        let Some(window) = &self.window else {
            return;
        };
        let Some(font_size) = self.renderer.as_ref().map(|renderer| renderer.font_size()) else {
            return;
        };

        let size = window.inner_size();
        let current_rows = self
            .active_tab()
            .map(|tab| tab.terminal.terminal.grid.rows())
            .unwrap_or(0);
        let current_cols = self
            .active_tab()
            .map(|tab| tab.terminal.terminal.grid.cols())
            .unwrap_or(0);
        let initial_model = build_ui_model(&UiBuildState {
            window_width: size.width as f32,
            window_height: size.height as f32,
            theme: self.theme,
            settings_open: self.settings_open,
            tabs: self.tab_views(),
            active_tab: self.active_tab,
            font_size,
            status: self.status_view(current_rows, current_cols),
        });
        let (rows, cols) = {
            let renderer = self
                .renderer
                .as_mut()
                .expect("renderer must exist before refreshing ui");
            renderer.set_terminal_viewport(initial_model.layout.terminal_viewport);
            let (rows, cols) = renderer.terminal_size();

            for tab in &mut self.tabs {
                if tab.terminal.terminal.grid.rows() != rows
                    || tab.terminal.terminal.grid.cols() != cols
                {
                    tab.terminal.resize(rows, cols);
                    let _ = tab.pty.resize(cols as u16, rows as u16);
                    tab.terminal.terminal.damage.mark_full_redraw();
                }
            }

            (rows, cols)
        };

        let ui_model = build_ui_model(&UiBuildState {
            window_width: size.width as f32,
            window_height: size.height as f32,
            theme: self.theme,
            settings_open: self.settings_open,
            tabs: self.tab_views(),
            active_tab: self.active_tab,
            font_size,
            status: self.status_view(rows, cols),
        });

        if let Some(renderer) = &mut self.renderer {
            renderer.set_terminal_viewport(ui_model.layout.terminal_viewport);
        }
        self.ui_model = ui_model;
        self.update_cached_metrics();
        self.ui_dirty = true;
        self.update_window_title();
        self.request_redraw();
    }

    /// 创建一个新的终端标签页并切换过去
    fn create_tab(&mut self) {
        let Some(renderer) = &self.renderer else {
            return;
        };
        let (rows, cols) = renderer.terminal_size();
        let title = shell_label(self.config.shell.program.as_str());

        match TerminalTab::new(title, rows, cols, &self.config, &self.theme) {
            Ok(tab) => {
                self.tabs.push(tab);
                self.active_tab = self.tabs.len().saturating_sub(1);
                self.focus_area = FocusArea::Terminal;
                self.refresh_ui_model();
            }
            Err(err) => {
                tracing::error!("Failed to create tab: {}", err);
            }
        }
    }

    /// 关闭指定标签页，并确保窗口内至少保留一个终端会话
    fn close_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }

        if let Some(tab) = self.tabs.get_mut(index) {
            let _ = tab.pty.kill();
        }

        if self.tabs.len() == 1 {
            self.tabs.clear();
            self.active_tab = 0;
            self.create_tab();
            return;
        }

        self.tabs.remove(index);
        if self.active_tab > index {
            self.active_tab -= 1;
        } else if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len().saturating_sub(1);
        }
        self.refresh_ui_model();
    }

    /// 处理快捷键动作
    fn handle_shortcut_action(&mut self, action: ShortcutAction) {
        match action {
            ShortcutAction::Copy => self.copy_selection(),
            ShortcutAction::Paste => self.paste_clipboard(),
            ShortcutAction::NewTab => self.create_tab(),
            ShortcutAction::CloseTab => self.close_tab(self.active_tab),
            ShortcutAction::NextTab => self.switch_to_next_tab(),
            ShortcutAction::PreviousTab => self.switch_to_previous_tab(),
            ShortcutAction::IncreaseFontSize => self.increase_font_size(),
            ShortcutAction::DecreaseFontSize => self.decrease_font_size(),
            ShortcutAction::ResetFontSize => self.reset_font_size(),
            ShortcutAction::ToggleSettings => self.toggle_settings(),
            ShortcutAction::SwitchTheme => self.switch_theme(),
        }
    }

    /// 切换到下一个标签页
    fn switch_to_next_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        self.active_tab = (self.active_tab + 1) % self.tabs.len();
        self.refresh_ui_model();
        self.request_redraw();
    }

    /// 切换到上一个标签页
    fn switch_to_previous_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        self.active_tab = if self.active_tab == 0 {
            self.tabs.len() - 1
        } else {
            self.active_tab - 1
        };
        self.refresh_ui_model();
        self.request_redraw();
    }

    /// 增加字号
    fn increase_font_size(&mut self) {
        if let Some(renderer) = &mut self.renderer {
            let current_size = renderer.font_size();
            let new_size = (current_size + 1.0).min(48.0);
            renderer.set_font_size(new_size);
            self.update_cached_metrics();
            self.refresh_ui_model();
            self.request_redraw();
        }
    }

    /// 减小字号
    fn decrease_font_size(&mut self) {
        if let Some(renderer) = &mut self.renderer {
            let current_size = renderer.font_size();
            let new_size = (current_size - 1.0).max(8.0);
            renderer.set_font_size(new_size);
            self.update_cached_metrics();
            self.refresh_ui_model();
            self.request_redraw();
        }
    }

    /// 重置字号到初始值
    fn reset_font_size(&mut self) {
        if let Some(renderer) = &mut self.renderer {
            renderer.set_font_size(self.initial_font_size);
            self.update_cached_metrics();
            self.refresh_ui_model();
            self.request_redraw();
        }
    }

    /// 切换主题
    fn switch_theme(&mut self) {
        self.theme = AppTheme::from_preset(self.theme.preset.next());
        let fg = self.theme.terminal_foreground;
        let bg = self.theme.terminal_background;
        if let Some(tab) = self.active_tab_mut() {
            tab.terminal.terminal.update_theme(fg, bg);
            tab.terminal.terminal.damage.mark_full_redraw();
        }
        self.refresh_ui_model();
        self.request_redraw();
    }

    /// 切换设置面板
    fn toggle_settings(&mut self) {
        self.settings_open = !self.settings_open;
        self.refresh_ui_model();
        self.request_redraw();
    }

    /// 应用新的主题预设，并同步 terminal 与 renderer 的默认颜色
    fn apply_theme_preset(&mut self, preset: ThemePreset) {
        self.theme = AppTheme::from_preset(preset);

        if let Some(renderer) = &mut self.renderer {
            renderer.set_terminal_palette(
                self.theme.terminal_foreground,
                self.theme.terminal_background,
            );
        }

        for tab in &mut self.tabs {
            tab.terminal.terminal.update_theme(
                self.theme.terminal_foreground,
                self.theme.terminal_background,
            );
        }

        self.refresh_ui_model();
    }

    /// 调整字号，并重新计算终端视口对应的行列数
    fn adjust_font_size(&mut self, delta: f32) {
        if let Some(renderer) = &mut self.renderer {
            let next_size = (renderer.font_size() + delta).clamp(10.0, 24.0);
            renderer.set_font_size(next_size);
            self.refresh_ui_model();
        }
    }

    /// 清空当前活动终端的内容与滚动缓冲区
    fn clear_active_terminal(&mut self) {
        let Some(tab) = self.active_tab_mut() else {
            return;
        };

        tab.terminal.terminal.grid.clear_all();
        tab.terminal.terminal.grid.clear_scrollback();
        tab.terminal.terminal.grid.reset_display_offset();
        tab.terminal.terminal.selection = None;
        tab.terminal.terminal.damage.mark_full_redraw();
        self.refresh_ui_model();
    }

    /// 让当前终端视口回到底部
    fn scroll_active_to_bottom(&mut self) {
        let Some(tab) = self.active_tab_mut() else {
            return;
        };

        tab.terminal.terminal.grid.reset_display_offset();
        tab.terminal.terminal.damage.mark_full_redraw();
        self.refresh_ui_model();
    }

    /// 处理一个 UI 命中动作
    fn handle_ui_action(&mut self, action: UiAction) {
        self.focus_area = FocusArea::Chrome;

        match action {
            UiAction::CreateTab => self.create_tab(),
            UiAction::ToggleSettings => {
                self.settings_open = !self.settings_open;
                self.refresh_ui_model();
            }
            UiAction::CycleTheme => self.apply_theme_preset(self.theme.preset.next()),
            UiAction::SelectTheme(preset) => self.apply_theme_preset(preset),
            UiAction::ActivateTab(index) => {
                if index < self.tabs.len() {
                    self.active_tab = index;
                    self.focus_area = FocusArea::Terminal;
                    self.refresh_ui_model();
                }
            }
            UiAction::CloseTab(index) => self.close_tab(index),
            UiAction::DecreaseFont => self.adjust_font_size(-1.0),
            UiAction::IncreaseFont => self.adjust_font_size(1.0),
            UiAction::CopySelection => self.copy_selection(),
            UiAction::PasteClipboard => self.paste_clipboard(),
            UiAction::ClearTerminal => self.clear_active_terminal(),
            UiAction::ScrollToBottom => self.scroll_active_to_bottom(),
        }
    }

    /// 当前鼠标是否位于终端面板区域
    fn cursor_in_terminal_panel(&self) -> bool {
        self.last_cursor_position
            .is_some_and(|(x, y)| self.ui_model.layout.terminal_panel.contains(x, y))
    }

    /// 将像素坐标转换为终端行列，超出终端边界时会自动夹紧
    fn pixel_to_cell(&self, x: f64, y: f64) -> Option<(usize, usize)> {
        if self.cell_width <= 0.0 || self.cell_height <= 0.0 {
            return None;
        }

        let viewport = self.ui_model.layout.terminal_viewport;
        let local_x = (x as f32 - viewport.x - self.padding_x).max(0.0);
        let local_y = (y as f32 - viewport.y - self.padding_y).max(0.0);
        let mut row = (local_y / self.cell_height) as usize;
        let mut col = (local_x / self.cell_width) as usize;

        if let Some(tab) = self.active_tab() {
            let max_row = tab.terminal.terminal.grid.rows().saturating_sub(1);
            let max_col = tab.terminal.terminal.grid.cols().saturating_sub(1);
            row = row.min(max_row);
            col = col.min(max_col);
        }

        Some((row, col))
    }

    /// 拖拽选择时，根据鼠标位置自动滚动 scrollback
    fn maybe_auto_scroll_selection(&mut self, y: f64) -> bool {
        let viewport = self.ui_model.layout.terminal_viewport;
        let threshold = (self.cell_height * 0.75).max(8.0) as f64;
        let Some(tab) = self.active_tab_mut() else {
            return false;
        };
        let top_edge = viewport.y as f64;
        let bottom_edge = viewport.bottom() as f64;
        let mut scrolled = false;

        if y < top_edge + threshold {
            tab.terminal.terminal.grid.scroll_display_up(1);
            tab.terminal.terminal.damage.mark_full_redraw();
            scrolled = true;
        } else if y > bottom_edge - threshold {
            tab.terminal.terminal.grid.scroll_display_down(1);
            tab.terminal.terminal.damage.mark_full_redraw();
            scrolled = true;
        }

        if scrolled {
            self.refresh_ui_model();
        }

        scrolled
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

        Some(if current_start < anchor_start {
            Selection::new(current_start, anchor_end)
        } else {
            Selection::new(anchor_start, current_end)
        })
    }

    /// 当前是否应将鼠标事件交给终端程序，而不是本地选择逻辑
    fn should_report_mouse(&self) -> bool {
        self.focus_area == FocusArea::Terminal
            && self.active_tab().is_some_and(|tab| {
                tab.terminal.terminal.mouse_tracking_mode() != MouseTrackingMode::Disabled
            })
            && !self.shift_pressed
    }

    /// 统一向活动 PTY 写入输入数据，并在写入前重置视口
    fn write_input(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }

        let Some(tab) = self.active_tab_mut() else {
            return;
        };
        if tab.shell_exited {
            return;
        }

        tab.terminal.terminal.grid.reset_display_offset();
        tab.terminal.terminal.damage.mark_full_redraw();
        
        if let Err(err) = tab.pty.write(bytes) {
            tracing::error!("Failed to write input to PTY: {}", err);
            // 检查 PTY 是否仍然存活
            if !tab.pty.is_alive() {
                tracing::error!("PTY process is dead, marking shell as exited");
                let index = self.active_tab;
                self.handle_shell_exit(index, -1);
            }
        }
    }

    /// 直接向指定标签页 PTY 写入原始响应数据，不改变本地视口或重绘状态
    fn write_pty_raw(&mut self, index: usize, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }

        let Some(tab) = self.tabs.get_mut(index) else {
            return;
        };
        if tab.shell_exited {
            return;
        }

        if let Err(err) = tab.pty.write(bytes) {
            tracing::error!("Failed to write raw PTY bytes: {}", err);
            // 检查 PTY 是否仍然存活
            if !tab.pty.is_alive() {
                tracing::error!("PTY process is dead, marking shell as exited");
                self.handle_shell_exit(index, -1);
            }
        }
    }

    /// 处理 shell 退出后的状态与提示
    fn handle_shell_exit(&mut self, index: usize, code: i32) {
        let Some(tab) = self.tabs.get_mut(index) else {
            return;
        };
        if tab.shell_exited {
            return;
        }

        tab.shell_exited = true;
        tab.shell_exit_code = Some(code);
        tab.selecting = false;
        tab.selection_start_cell = None;
        tab.selection_anchor = None;
        tab.pressed_mouse_button = None;
        tab.last_reported_cell = None;

        let message = format!("\r\n[SuperPower] shell exited with code {}\r\n", code);
        tab.terminal.process(message.as_bytes());
        tab.terminal.terminal.damage.mark_full_redraw();
        self.refresh_ui_model();
    }

    /// 根据终端 OSC 标题同步标签页标题
    fn sync_tab_title(&mut self, index: usize) -> bool {
        let Some(tab) = self.tabs.get_mut(index) else {
            return false;
        };

        let terminal_title = tab.terminal.terminal.title.trim();
        if terminal_title.is_empty() || terminal_title == tab.title {
            return false;
        }

        tab.title = terminal_title.to_string();
        true
    }

    /// 把活动标签页标题同步到窗口标题栏
    fn update_window_title(&self) {
        let Some(window) = &self.window else {
            return;
        };

        let title = self
            .active_tab()
            .map(|tab| format!("SuperPower Terminal - {}", tab.title))
            .unwrap_or_else(|| "SuperPower Terminal".to_string());
        window.set_title(title.as_str());
    }

    /// 检查并重新加载配置文件（如果已修改）
    fn check_and_reload_config(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_config_check) < CONFIG_CHECK_INTERVAL {
            return;
        }
        self.last_config_check = now;

        let Ok(metadata) = std::fs::metadata(&self.config_path) else {
            return;
        };
        let Ok(mtime) = metadata.modified() else {
            return;
        };

        if let Some(last_mtime) = self.last_config_mtime {
            if mtime <= last_mtime {
                return;
            }
        }

        tracing::info!("Config file changed, reloading...");
        let new_config = Config::load_from_file(&self.config_path);
        
        // 更新快捷键
        self.shortcut_manager = if new_config.shortcuts.is_empty() {
            ShortcutManager::default()
        } else {
            ShortcutManager::from_config(&new_config.shortcuts)
        };

        // 更新字号（如果改变）
        if (new_config.font.size - self.config.font.size).abs() > 0.1 {
            if let Some(renderer) = &mut self.renderer {
                renderer.set_font_size(new_config.font.size);
                self.update_cached_metrics();
                self.refresh_ui_model();
            }
        }

        self.config = new_config;
        self.last_config_mtime = Some(mtime);
        self.request_redraw();
        
        tracing::info!("Config reloaded successfully");
    }

    /// 将当前终端光标同步给 IME，避免候选框位置漂移
    fn update_ime_cursor_area(&self) {
        let (Some(window), Some(tab)) = (&self.window, &self.active_tab()) else {
            return;
        };

        let viewport = self.ui_model.layout.terminal_viewport;
        let preedit_col_offset = tab
            .terminal
            .terminal
            .ime_preedit()
            .map(|preedit| {
                let cursor_chars = preedit.cursor_range.map(|(start, _)| start).unwrap_or(0);
                preedit_visual_offset(preedit.text.as_str(), cursor_chars)
            })
            .unwrap_or(0);

        let x = viewport.x
            + self.padding_x
            + (tab.terminal.terminal.cursor.col + preedit_col_offset) as f32 * self.cell_width;
        let y = viewport.y
            + self.padding_y
            + tab.terminal.terminal.cursor.row as f32 * self.cell_height;
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
        let Some((mode, sgr)) = self.active_tab().map(|tab| {
            (
                tab.terminal.terminal.mouse_tracking_mode(),
                tab.terminal.terminal.mouse_sgr_mode(),
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
            self.write_pty_raw(self.active_tab, &bytes);
        }

        if let Some(tab) = self.active_tab_mut() {
            match kind {
                MouseReportKind::Motion(_) => tab.last_reported_cell = Some((row, col)),
                MouseReportKind::Press(button) => {
                    tab.pressed_mouse_button = Some(button);
                    tab.last_reported_cell = Some((row, col));
                }
                MouseReportKind::Release(_) => {
                    tab.pressed_mouse_button = None;
                    tab.last_reported_cell = Some((row, col));
                }
                MouseReportKind::WheelUp | MouseReportKind::WheelDown => {
                    tab.last_reported_cell = Some((row, col));
                }
            }

            if mode == MouseTrackingMode::Disabled {
                tab.last_reported_cell = None;
            }
        }
    }

    /// 复制活动标签页中的选区文本到剪贴板
    fn copy_selection(&mut self) {
        let Some(tab) = self.active_tab() else {
            return;
        };
        let Some(selection) = &tab.terminal.terminal.selection else {
            return;
        };

        let text = selection.text(&tab.terminal.terminal.grid);
        if text.is_empty() {
            return;
        }

        match arboard::Clipboard::new() {
            Ok(mut clipboard) => {
                if let Err(err) = clipboard.set_text(&text) {
                    tracing::warn!("Failed to copy to clipboard: {}", err);
                }
            }
            Err(err) => {
                tracing::warn!("Failed to access clipboard: {}", err);
            }
        }
    }

    /// 从剪贴板读取文本并粘贴到活动终端
    fn paste_clipboard(&mut self) {
        let text = match arboard::Clipboard::new() {
            Ok(mut clipboard) => clipboard.get_text().unwrap_or_default(),
            Err(_) => return,
        };
        if text.is_empty() {
            return;
        }

        let payload = if let Some(tab) = self.active_tab() {
            if tab.terminal.terminal.bracketed_paste_mode() {
                encode_bracketed_paste(&text)
            } else {
                text.into_bytes()
            }
        } else {
            text.into_bytes()
        };

        self.write_input(&payload);
    }

    /// 处理终端区左键按下，支持字符/单词/整行选择
    fn handle_terminal_left_press(&mut self) {
        let pointer_cell = self.active_tab().and_then(|tab| tab.pointer_cell);
        let Some((row, col)) = pointer_cell else {
            return;
        };

        let now = Instant::now();
        let double_click_timeout = Duration::from_millis(450);

        if let Some(tab) = self.active_tab_mut() {
            if tab.last_click_cell == Some((row, col))
                && tab
                    .last_click_time
                    .is_some_and(|last| now.duration_since(last) <= double_click_timeout)
            {
                tab.click_count = if tab.click_count >= 3 {
                    1
                } else {
                    tab.click_count + 1
                };
            } else {
                tab.click_count = 1;
            }

            tab.last_click_time = Some(now);
            tab.last_click_cell = Some((row, col));
            tab.selection_drag_mode = Self::drag_mode_from_clicks(tab.click_count);
            tab.selecting = true;
            tab.selection_start_cell = Some((row, col));

            tab.selection_anchor =
                Self::semantic_bounds(&tab.terminal, row, col, tab.selection_drag_mode);

            if tab.selection_drag_mode == SelectionDragMode::Char {
                tab.terminal.terminal.selection = None;
                tab.terminal.terminal.damage.mark_full_redraw();
            } else if let Some(anchor) = tab.selection_anchor {
                if let Some(selection) = Self::build_selection_for_drag(
                    &tab.terminal,
                    anchor,
                    tab.selection_drag_mode,
                    row,
                    col,
                ) {
                    tab.terminal.terminal.selection = Some(selection);
                    tab.terminal.terminal.damage.mark_full_redraw();
                }
            }
        }
    }

    /// 处理活动标签页的 PTY 输出和退出事件
    fn process_pty_events(&mut self) {
        let mut needs_ui_refresh = false;
        let tab_len = self.tabs.len();

        for index in 0..tab_len {
            let mut exit_code = None;
            let mut pending_output = Vec::new();

            {
                let tab = &mut self.tabs[index];
                while let Ok(event) = tab.pty.rx.try_recv() {
                    match event {
                        PtyEvent::Data(data) => {
                            tab.terminal.process(&data);
                            pending_output.extend_from_slice(
                                tab.terminal.terminal.take_pending_output().as_slice(),
                            );
                        }
                        PtyEvent::Exit(code) => {
                            tracing::info!("Shell exited in tab {}", index);
                            exit_code = Some(code);
                        }
                    }
                }
            }

            if !pending_output.is_empty() {
                self.write_pty_raw(index, &pending_output);
            }

            if let Some(code) = exit_code {
                self.handle_shell_exit(index, code);
                needs_ui_refresh = true;
            }

            if self.sync_tab_title(index) {
                needs_ui_refresh = true;
            }
        }

        if needs_ui_refresh {
            self.refresh_ui_model();
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = WindowAttributes::default()
            .with_title("SuperPower Terminal")
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.config.window.width,
                self.config.window.height,
            ));

        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("Failed to create window"),
        );
        window.set_ime_allowed(true);

        let mut renderer = pollster::block_on(Renderer::new(
            Arc::clone(&window),
            RendererOptions {
                font_family: self.config.font.family.clone(),
                font_size: self.config.font.size,
                default_foreground: self.theme.terminal_foreground,
                default_background: self.theme.terminal_background,
                padding_x: self.config.window.padding_x,
                padding_y: self.config.window.padding_y,
            },
        ));

        renderer.set_terminal_viewport(self.ui_model.layout.terminal_viewport);
        self.window = Some(window);
        self.renderer = Some(renderer);
        self.update_cached_metrics();
        self.refresh_ui_model();
        self.create_tab();
        self.request_redraw();
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
                self.refresh_ui_model();
            }

            WindowEvent::ScaleFactorChanged {
                scale_factor,
                inner_size_writer: _,
            } => {
                tracing::info!("DPI scale factor changed to {}", scale_factor);
                if let Some(renderer) = &mut self.renderer {
                    renderer.update_font_metrics(scale_factor);
                }
                self.refresh_ui_model();
            }

            WindowEvent::RedrawRequested => {
                // 定期检查配置文件是否更新
                self.check_and_reload_config();
                
                self.process_pty_events();

                let active_index = self.active_tab;
                let chrome = self.ui_model.chrome.clone();
                let should_render = self.tabs.get(active_index).is_some_and(|tab| {
                    self.ui_dirty
                        || self
                            .renderer
                            .as_ref()
                            .is_some_and(|renderer| renderer.needs_render(&tab.terminal))
                });

                if should_render {
                    if let (Some(renderer), Some(tab)) =
                        (&mut self.renderer, self.tabs.get(active_index))
                    {
                        match renderer.render(&tab.terminal, &chrome) {
                            Ok(_) => {}
                            Err(wgpu::SurfaceError::Lost) => {}
                            Err(wgpu::SurfaceError::OutOfMemory) => {
                                event_loop.exit();
                            }
                            Err(err) => {
                                tracing::error!("Render error: {:?}", err);
                            }
                        }
                    }

                    if let Some(tab) = self.active_tab_mut() {
                        tab.terminal.terminal.damage.clear();
                    }
                    self.ui_dirty = false;
                }

                self.update_ime_cursor_area();

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                use winit::keyboard::PhysicalKey;

                if let PhysicalKey::Code(keycode) = event.physical_key {
                    let modifiers = winit::keyboard::ModifiersState::from_bits_truncate(
                        (self.ctrl_pressed as u32) << 3
                            | (self.shift_pressed as u32) << 0
                            | (self.alt_pressed as u32) << 1,
                    );

                    if let Some(action) = self.shortcut_manager.find_action(keycode, modifiers) {
                        self.handle_shortcut_action(action);
                        return;
                    }
                }

                if self.focus_area != FocusArea::Terminal {
                    return;
                }

                let shift_pressed = self.shift_pressed;
                let ctrl_pressed = self.ctrl_pressed;
                let alt_pressed = self.alt_pressed;
                if let Some(tab) = self.active_tab_mut() {
                    let payload = handle_key_input(
                        event,
                        &mut tab.terminal,
                        shift_pressed,
                        ctrl_pressed,
                        alt_pressed,
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

            WindowEvent::Ime(Ime::Commit(text))
                if !text.is_empty() && self.focus_area == FocusArea::Terminal =>
            {
                if let Some(tab) = self.active_tab_mut() {
                    tab.terminal.terminal.clear_ime_preedit();
                }
                self.write_input(text.as_bytes());
            }

            WindowEvent::Ime(Ime::Preedit(text, cursor_range))
                if self.focus_area == FocusArea::Terminal =>
            {
                if let Some(tab) = self.active_tab_mut() {
                    if text.is_empty() {
                        tab.terminal.terminal.clear_ime_preedit();
                    } else {
                        tab.terminal.terminal.set_ime_preedit(
                            text.clone(),
                            cursor_range
                                .map(|range| preedit_byte_range_to_char_range(&text, range)),
                        );
                    }
                }
            }

            WindowEvent::Ime(Ime::Disabled) => {
                if let Some(tab) = self.active_tab_mut() {
                    tab.terminal.terminal.clear_ime_preedit();
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                if !self.cursor_in_terminal_panel() {
                    return;
                }

                if self.should_report_mouse() {
                    let pointer_cell = self.active_tab().and_then(|tab| tab.pointer_cell);
                    if let Some((row, col)) = pointer_cell {
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
                    return;
                }

                if let Some(tab) = self.active_tab_mut() {
                    let lines = match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, y) => (-y * 3.0) as isize,
                        winit::event::MouseScrollDelta::PixelDelta(pos) => (-pos.y / 20.0) as isize,
                    };
                    if lines > 0 {
                        tab.terminal.terminal.grid.scroll_display_up(lines as usize);
                        tab.terminal.terminal.damage.mark_full_redraw();
                    } else if lines < 0 {
                        tab.terminal
                            .terminal
                            .grid
                            .scroll_display_down((-lines) as usize);
                        tab.terminal.terminal.damage.mark_full_redraw();
                    }
                    self.refresh_ui_model();
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                let cursor = self.last_cursor_position.unwrap_or((0.0, 0.0));

                if state == ElementState::Pressed && !self.cursor_in_terminal_panel() {
                    self.focus_area = FocusArea::Chrome;
                }

                if state == ElementState::Released {
                    if let Some(action) = self.ui_model.hit_test(cursor.0, cursor.1) {
                        self.handle_ui_action(action);
                        return;
                    }
                }

                if !self.cursor_in_terminal_panel() {
                    return;
                }

                self.focus_area = FocusArea::Terminal;
                let pointer_cell = self.pixel_to_cell(cursor.0, cursor.1);
                if let Some(tab) = self.active_tab_mut() {
                    tab.pointer_cell = pointer_cell;
                }

                if self.should_report_mouse() {
                    let pointer_cell = self.active_tab().and_then(|tab| tab.pointer_cell);
                    if let Some((row, col)) = pointer_cell {
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
                                let release_button = self
                                    .active_tab()
                                    .and_then(|tab| tab.pressed_mouse_button)
                                    .unwrap_or(button);
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
                        self.handle_terminal_left_press();
                    }
                    (MouseButton::Left, ElementState::Released) => {
                        if let Some(tab) = self.active_tab_mut() {
                            tab.selecting = false;
                            tab.selection_start_cell = None;
                            tab.selection_anchor = None;
                        }
                    }
                    (MouseButton::Right, ElementState::Released)
                    | (MouseButton::Middle, ElementState::Released) => {
                        self.paste_clipboard();
                    }
                    _ => {}
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.last_cursor_position = Some((position.x, position.y));
                let cursor_in_terminal = self.cursor_in_terminal_panel();
                let pointer_cell = self.pixel_to_cell(position.x, position.y);

                if let Some(tab) = self.active_tab_mut() {
                    tab.pointer_cell = if cursor_in_terminal || tab.selecting {
                        pointer_cell
                    } else {
                        None
                    };
                }

                if self.should_report_mouse() {
                    let pointer_cell = self.active_tab().and_then(|tab| tab.pointer_cell);
                    let Some((row, col)) = pointer_cell else {
                        return;
                    };
                    let Some(tab) = self.active_tab() else {
                        return;
                    };

                    let mode = tab.terminal.terminal.mouse_tracking_mode();
                    let should_report_motion = match mode {
                        MouseTrackingMode::Disabled | MouseTrackingMode::Normal => false,
                        MouseTrackingMode::ButtonEvent => tab.pressed_mouse_button.is_some(),
                        MouseTrackingMode::AnyEvent => true,
                    };

                    if should_report_motion && tab.last_reported_cell != Some((row, col)) {
                        self.report_mouse(
                            MouseReportKind::Motion(tab.pressed_mouse_button),
                            row,
                            col,
                        );
                    }
                    return;
                }

                let selecting = self.active_tab().is_some_and(|tab| tab.selecting);
                if !selecting {
                    return;
                }

                self.maybe_auto_scroll_selection(position.y);

                let pointer_cell = self.active_tab().and_then(|tab| tab.pointer_cell);
                let Some((row, col)) = pointer_cell else {
                    return;
                };

                if let Some(tab) = self.active_tab_mut() {
                    let grid_rows = tab.terminal.terminal.grid.rows();
                    let grid_cols = tab.terminal.terminal.grid.cols();
                    let row = row.min(grid_rows.saturating_sub(1));
                    let col = col.min(grid_cols.saturating_sub(1));

                    if tab.selection_start_cell.is_none() {
                        tab.selection_start_cell = Some((row, col));
                    }

                    if let Some(anchor) = tab.selection_anchor {
                        if let Some(selection) = Self::build_selection_for_drag(
                            &tab.terminal,
                            anchor,
                            tab.selection_drag_mode,
                            row,
                            col,
                        ) {
                            tab.terminal.terminal.selection = Some(selection);
                            tab.terminal.terminal.damage.mark_full_redraw();
                        }
                    }
                }
            }

            _ => {}
        }
    }
}

/// 获取 shell 展示名，优先保留可读的文件名
fn shell_label(program: &str) -> String {
    Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(program)
        .to_string()
}

/// 构建新标签页首屏欢迎横幅
fn startup_banner(shell_name: String) -> String {
    format!(
        "[SuperPower] {} ready - Ctrl+Shift+T new tab, Ctrl+Shift+C/V copy paste\r\n\r\n",
        shell_name
    )
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

/// 将 IME 提供的字节范围转换为字符索引范围
fn preedit_byte_range_to_char_range(text: &str, range: (usize, usize)) -> (usize, usize) {
    let start = byte_offset_to_char_index(text, range.0);
    let end = byte_offset_to_char_index(text, range.1);
    (start, end)
}

/// 将 UTF-8 字节偏移转换为字符索引
fn byte_offset_to_char_index(text: &str, byte_offset: usize) -> usize {
    text.char_indices()
        .take_while(|(idx, _)| *idx < byte_offset)
        .count()
}

/// 计算 preedit 光标在终端网格中的列偏移
fn preedit_visual_offset(text: &str, cursor_chars: usize) -> usize {
    text.chars().take(cursor_chars).map(char_width).sum()
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

    /// 验证 IME 字节范围会被正确映射到字符索引
    #[test]
    fn preedit_range_maps_to_char_indices() {
        let range = preedit_byte_range_to_char_range("啊b", (3, 4));
        assert_eq!(range, (1, 2));
    }

    /// 验证 preedit 光标列偏移会按字符宽度累计
    #[test]
    fn preedit_visual_offset_counts_wide_chars() {
        assert_eq!(preedit_visual_offset("啊b", 1), 2);
        assert_eq!(preedit_visual_offset("啊b", 2), 3);
    }

    /// 验证 shell 名称会优先使用文件名部分
    #[test]
    fn shell_label_uses_file_name() {
        assert_eq!(shell_label("C:\\Windows\\System32\\cmd.exe"), "cmd.exe");
        assert_eq!(shell_label("powershell"), "powershell");
    }

    /// 验证新标签页欢迎横幅会包含 shell 名称与快捷键提示
    #[test]
    fn startup_banner_contains_shell_and_shortcuts() {
        let banner = startup_banner("cmd.exe".to_string());
        assert!(banner.contains("cmd.exe"));
        assert!(banner.contains("Ctrl+Shift+T"));
    }
}

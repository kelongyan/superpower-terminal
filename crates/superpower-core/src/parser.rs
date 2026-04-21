use crate::cell::{char_width, Cell, CellFlags, Color};
use crate::cursor::{Cursor, CursorShape};
use crate::damage::DamageTracker;
use crate::grid::Grid;
use crate::selection::Selection;

/// ANSI 颜色调色板
const ANSI_COLORS: [Color; 16] = [
    Color::new(0x1E, 0x1E, 0x1E), // 0  Black
    Color::new(0xCD, 0x00, 0x00), // 1  Red
    Color::new(0x00, 0xCD, 0x00), // 2  Green
    Color::new(0xCD, 0xCD, 0x00), // 3  Yellow
    Color::new(0x00, 0x00, 0xCD), // 4  Blue
    Color::new(0xCD, 0x00, 0xCD), // 5  Magenta
    Color::new(0x00, 0xCD, 0xCD), // 6  Cyan
    Color::new(0xFA, 0xEB, 0xD7), // 7  White
    Color::new(0x40, 0x40, 0x40), // 8  Bright Black
    Color::new(0xFF, 0x00, 0x00), // 9  Bright Red
    Color::new(0x00, 0xFF, 0x00), // 10 Bright Green
    Color::new(0xFF, 0xFF, 0x00), // 11 Bright Yellow
    Color::new(0x00, 0x00, 0xFF), // 12 Bright Blue
    Color::new(0xFF, 0x00, 0xFF), // 13 Bright Magenta
    Color::new(0x00, 0xFF, 0xFF), // 14 Bright Cyan
    Color::new(0xFF, 0xFF, 0xFF), // 15 Bright White
];

/// 鼠标跟踪模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MouseTrackingMode {
    /// 不上报鼠标事件
    #[default]
    Disabled,
    /// 基础点击跟踪（X10/1000）
    Normal,
    /// 按钮拖拽跟踪（1002）
    ButtonEvent,
    /// 任意移动跟踪（1003）
    AnyEvent,
}

/// IME 预编辑状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImePreedit {
    /// 当前预编辑文本
    pub text: String,
    /// 预编辑光标范围（按字符索引）
    pub cursor_range: Option<(usize, usize)>,
}

/// 终端 — 组合 Grid + Cursor + 解析器
pub struct Terminal {
    pub grid: Grid,
    pub cursor: Cursor,
    pub damage: DamageTracker,
    /// 保存的光标位置
    saved_cursor: Cursor,
    /// 当前前景色
    foreground: Color,
    /// 当前背景色
    background: Color,
    /// 当前 Cell 标志
    cell_flags: CellFlags,
    /// 默认前景色
    default_foreground: Color,
    /// 默认背景色
    default_background: Color,
    /// 标准终端模式集合
    modes: std::collections::HashSet<u16>,
    /// 私有终端模式集合
    private_modes: std::collections::HashSet<u16>,
    /// 应用光标键模式
    application_cursor_keys: bool,
    /// 鼠标跟踪模式
    mouse_tracking_mode: MouseTrackingMode,
    /// 是否启用 SGR 扩展鼠标编码（1006）
    mouse_sgr_mode: bool,
    /// 是否启用 bracketed paste（2004）
    bracketed_paste_mode: bool,
    /// 是否处于 alternate screen
    alternate_screen: bool,
    /// alternate screen 对应的网格
    alternate_grid: Grid,
    /// alternate screen 对应的光标
    alternate_cursor: Cursor,
    /// alternate screen 对应的保存光标
    alternate_saved_cursor: Cursor,
    /// IME 预编辑状态
    ime_preedit: Option<ImePreedit>,
    /// 窗口标题
    pub title: String,
    /// 当前选区
    pub selection: Option<Selection>,
}

/// 终端处理器 — 持有 Parser，驱动 Terminal 状态更新
pub struct TerminalHandler {
    parser: vte::Parser,
    pub terminal: Terminal,
}

impl TerminalHandler {
    pub fn new(rows: usize, cols: usize, scrollback_limit: usize) -> Self {
        Self {
            parser: vte::Parser::new(),
            terminal: Terminal::new(rows, cols, scrollback_limit),
        }
    }

    /// 使用指定默认主题颜色创建终端处理器
    pub fn with_theme(
        rows: usize,
        cols: usize,
        scrollback_limit: usize,
        default_foreground: Color,
        default_background: Color,
    ) -> Self {
        Self {
            parser: vte::Parser::new(),
            terminal: Terminal::with_theme(
                rows,
                cols,
                scrollback_limit,
                default_foreground,
                default_background,
            ),
        }
    }

    /// 喂入 PTY 输出数据，解析并更新 Terminal
    pub fn process(&mut self, data: &[u8]) {
        for &byte in data {
            self.parser.advance(&mut self.terminal, byte);
        }
    }

    /// 调整终端大小
    pub fn resize(&mut self, new_rows: usize, new_cols: usize) {
        self.terminal.resize(new_rows, new_cols);
    }
}

impl Terminal {
    pub fn new(rows: usize, cols: usize, scrollback_limit: usize) -> Self {
        Self::with_theme(
            rows,
            cols,
            scrollback_limit,
            Color::DEFAULT_FG,
            Color::DEFAULT_BG,
        )
    }

    /// 使用指定默认主题颜色创建终端
    pub fn with_theme(
        rows: usize,
        cols: usize,
        scrollback_limit: usize,
        default_foreground: Color,
        default_background: Color,
    ) -> Self {
        let mut terminal = Self {
            grid: Grid::with_colors(
                rows,
                cols,
                scrollback_limit,
                default_foreground,
                default_background,
            ),
            cursor: Cursor::new(),
            damage: DamageTracker::new(rows),
            saved_cursor: Cursor::new(),
            foreground: default_foreground,
            background: default_background,
            cell_flags: CellFlags::EMPTY,
            default_foreground,
            default_background,
            modes: std::collections::HashSet::new(),
            private_modes: std::collections::HashSet::new(),
            application_cursor_keys: false,
            mouse_tracking_mode: MouseTrackingMode::Disabled,
            mouse_sgr_mode: false,
            bracketed_paste_mode: false,
            alternate_screen: false,
            alternate_grid: Grid::with_colors(
                rows,
                cols,
                0,
                default_foreground,
                default_background,
            ),
            alternate_cursor: Cursor::new(),
            alternate_saved_cursor: Cursor::new(),
            ime_preedit: None,
            title: String::new(),
            selection: None,
        };
        // 首帧强制全量重绘，确保窗口初始化时就有稳定输出。
        terminal.damage.mark_full_redraw();
        terminal
    }

    pub fn resize(&mut self, new_rows: usize, new_cols: usize) {
        self.grid.resize(new_rows, new_cols);
        self.alternate_grid.resize(new_rows, new_cols);
        self.cursor.clamp(new_rows, new_cols);
        self.alternate_cursor.clamp(new_rows, new_cols);
        self.damage.resize(new_rows);
    }

    // === 内部辅助方法 ===

    fn make_cell(&self, character: char) -> Cell {
        Cell {
            character,
            foreground: self.foreground,
            background: self.background,
            flags: self.cell_flags,
        }
    }

    fn mark_dirty(&mut self, row: usize) {
        self.damage.mark_row(row);
    }

    /// 保存光标位置
    fn save_cursor(&mut self) {
        self.saved_cursor = self.cursor.clone();
    }

    /// 恢复光标位置
    fn restore_cursor(&mut self) {
        self.cursor = self.saved_cursor.clone();
        self.mark_dirty(self.cursor.row);
    }

    /// 创建一个新的空 alternate screen
    fn fresh_alternate_grid(&self) -> Grid {
        Grid::with_colors(
            self.grid.rows(),
            self.grid.cols(),
            0,
            self.default_foreground,
            self.default_background,
        )
    }

    /// 进入 alternate screen
    fn enter_alternate_screen(&mut self) {
        if self.alternate_screen {
            return;
        }

        // 每次进入 alternate screen 都从空白状态开始，避免遗留上次的内容。
        self.alternate_grid = self.fresh_alternate_grid();
        self.alternate_cursor = Cursor::new();
        self.alternate_saved_cursor = Cursor::new();

        std::mem::swap(&mut self.grid, &mut self.alternate_grid);
        std::mem::swap(&mut self.cursor, &mut self.alternate_cursor);
        std::mem::swap(&mut self.saved_cursor, &mut self.alternate_saved_cursor);

        self.alternate_screen = true;
        self.ime_preedit = None;
        self.selection = None;
        self.damage.mark_full_redraw();
    }

    /// 离开 alternate screen，回到主屏
    fn leave_alternate_screen(&mut self) {
        if !self.alternate_screen {
            return;
        }

        std::mem::swap(&mut self.grid, &mut self.alternate_grid);
        std::mem::swap(&mut self.cursor, &mut self.alternate_cursor);
        std::mem::swap(&mut self.saved_cursor, &mut self.alternate_saved_cursor);

        self.alternate_screen = false;
        self.ime_preedit = None;
        self.selection = None;
        self.damage.mark_full_redraw();
    }

    /// 设置标准模式
    fn set_mode(&mut self, mode: u16) {
        self.modes.insert(mode);
    }

    /// 清除标准模式
    fn unset_mode(&mut self, mode: u16) {
        self.modes.remove(&mode);
    }

    /// 设置 DEC 私有模式，并同步更新输入/光标相关状态
    fn set_private_mode(&mut self, mode: u16) {
        self.private_modes.insert(mode);

        match mode {
            // DECCKM - 应用光标键模式
            1 => self.application_cursor_keys = true,
            // DECTCEM - 显示光标
            25 => self.cursor.visible = true,
            // DECNKM - 应用小键盘模式
            66 => self.cursor.app_mode = true,
            // Alternate screen（xterm 兼容）
            47 | 1047 => self.enter_alternate_screen(),
            // Save cursor
            1048 => self.save_cursor(),
            // Save cursor + alternate screen
            1049 => {
                self.save_cursor();
                self.enter_alternate_screen();
            }
            // 鼠标点击跟踪
            1000 => self.mouse_tracking_mode = MouseTrackingMode::Normal,
            // 鼠标拖拽跟踪
            1002 => self.mouse_tracking_mode = MouseTrackingMode::ButtonEvent,
            // 鼠标任意移动跟踪
            1003 => self.mouse_tracking_mode = MouseTrackingMode::AnyEvent,
            // SGR 鼠标编码
            1006 => self.mouse_sgr_mode = true,
            // bracketed paste
            2004 => self.bracketed_paste_mode = true,
            _ => {}
        }
    }

    /// 清除 DEC 私有模式，并同步更新输入/光标相关状态
    fn unset_private_mode(&mut self, mode: u16) {
        self.private_modes.remove(&mode);

        match mode {
            // DECCKM - 普通光标键模式
            1 => self.application_cursor_keys = false,
            // DECTCEM - 隐藏光标
            25 => self.cursor.visible = false,
            // DECNKM - 普通小键盘模式
            66 => self.cursor.app_mode = false,
            // Alternate screen
            47 | 1047 => self.leave_alternate_screen(),
            // Restore cursor
            1048 => self.restore_cursor(),
            // Leave alternate screen + restore cursor
            1049 => {
                self.leave_alternate_screen();
                self.restore_cursor();
            }
            // 关闭鼠标跟踪
            1000 | 1002 | 1003 => self.mouse_tracking_mode = MouseTrackingMode::Disabled,
            // 关闭 SGR 鼠标编码
            1006 => self.mouse_sgr_mode = false,
            // 关闭 bracketed paste
            2004 => self.bracketed_paste_mode = false,
            _ => {}
        }
    }

    /// 是否启用应用光标键模式
    pub fn application_cursor_keys(&self) -> bool {
        self.application_cursor_keys
    }

    /// 是否启用应用小键盘模式
    pub fn keypad_application_mode(&self) -> bool {
        self.cursor.app_mode
    }

    /// 当前鼠标跟踪模式
    pub fn mouse_tracking_mode(&self) -> MouseTrackingMode {
        self.mouse_tracking_mode
    }

    /// 是否启用 SGR 鼠标编码
    pub fn mouse_sgr_mode(&self) -> bool {
        self.mouse_sgr_mode
    }

    /// 是否启用 bracketed paste
    pub fn bracketed_paste_mode(&self) -> bool {
        self.bracketed_paste_mode
    }

    /// 是否处于 alternate screen
    pub fn alternate_screen(&self) -> bool {
        self.alternate_screen
    }

    /// 设置 IME 预编辑文本
    pub fn set_ime_preedit(&mut self, text: String, cursor_range: Option<(usize, usize)>) {
        if text.is_empty() {
            self.clear_ime_preedit();
            return;
        }

        self.ime_preedit = Some(ImePreedit { text, cursor_range });
        self.damage.mark_full_redraw();
    }

    /// 清除 IME 预编辑文本
    pub fn clear_ime_preedit(&mut self) {
        if self.ime_preedit.take().is_some() {
            self.damage.mark_full_redraw();
        }
    }

    /// 获取 IME 预编辑状态
    pub fn ime_preedit(&self) -> Option<&ImePreedit> {
        self.ime_preedit.as_ref()
    }

    fn newline(&mut self) {
        let row = self.cursor.row;
        let bottom = self.grid.scroll_bottom();

        if row == bottom {
            self.grid.scroll_up();
            // 新输出时重置视口到底部
            self.grid.reset_display_offset();
            self.damage.mark_full_redraw();
        } else if row < self.grid.rows() - 1 {
            self.cursor.row += 1;
        }
    }

    fn carriage_return(&mut self) {
        self.cursor.col = 0;
    }

    fn tab(&mut self) {
        let col = self.cursor.col;
        // 跳到下一个 8 的倍数位置
        let next_tab = ((col / 8) + 1) * 8;
        self.cursor.col = next_tab.min(self.grid.cols() - 1);
    }

    fn backspace(&mut self) {
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
        }
    }

    fn put_char(&mut self, c: char) {
        let _row = self.cursor.row;
        let col = self.cursor.col;

        if col >= self.grid.cols() {
            // 自动换行
            self.cursor.col = 0;
            self.newline();
        }

        let row = self.cursor.row;
        let col = self.cursor.col;

        // 检查是否是宽字符
        let width = char_width(c);
        if width == 0 {
            // 不可见字符，跳过
            return;
        }

        let cell = self.make_cell(c);
        let mut cell = cell;
        if width == 2 {
            cell.flags |= CellFlags::WIDE;
        }
        self.grid.write_cell(row, col, cell);
        self.mark_dirty(row);

        if width == 2 && col + 1 < self.grid.cols() {
            // 宽字符：标记后续 Cell 为 spacer
            let mut spacer = Cell::blank_with_colors(self.foreground, self.background);
            spacer.flags = CellFlags::WIDE_CHAR_SPACER;
            self.grid.write_cell(row, col + 1, spacer);
            self.cursor.col = col + 2;
        } else {
            self.cursor.col = col + 1;
        }
    }

    /// CSI: 设置图形属性 (SGR)
    fn sgr(&mut self, params: &[u64]) {
        if params.is_empty() {
            // SGR 0: 重置
            self.foreground = self.default_foreground;
            self.background = self.default_background;
            self.cell_flags = CellFlags::EMPTY;
            return;
        }

        let mut i = 0;
        while i < params.len() {
            let p = params[i];
            match p {
                0 => {
                    self.foreground = self.default_foreground;
                    self.background = self.default_background;
                    self.cell_flags = CellFlags::EMPTY;
                }
                1 => self.cell_flags |= CellFlags::BOLD,
                2 => self.cell_flags |= CellFlags::DIM,
                3 => self.cell_flags |= CellFlags::ITALIC,
                4 => self.cell_flags |= CellFlags::UNDERLINE,
                5 => self.cell_flags |= CellFlags::BLINK,
                7 => self.cell_flags |= CellFlags::REVERSE,
                8 => self.cell_flags |= CellFlags::HIDDEN,
                9 => self.cell_flags |= CellFlags::STRIKETHROUGH,
                21 => self.cell_flags |= CellFlags::DOUBLE_UNDERLINE,
                22 => {
                    self.cell_flags = self.cell_flags & !(CellFlags::BOLD | CellFlags::DIM);
                }
                23 => self.cell_flags = self.cell_flags & !CellFlags::ITALIC,
                24 => {
                    self.cell_flags =
                        self.cell_flags & !(CellFlags::UNDERLINE | CellFlags::DOUBLE_UNDERLINE);
                }
                25 => self.cell_flags = self.cell_flags & !CellFlags::BLINK,
                27 => self.cell_flags = self.cell_flags & !CellFlags::REVERSE,
                28 => self.cell_flags = self.cell_flags & !CellFlags::HIDDEN,
                29 => self.cell_flags = self.cell_flags & !CellFlags::STRIKETHROUGH,
                30..=37 => {
                    // 前景色标准 8 色
                    self.foreground = ANSI_COLORS[(p - 30) as usize];
                }
                38 if i + 1 < params.len() => {
                    // 扩展前景色
                    match params[i + 1] {
                        5 if i + 2 < params.len() => {
                            // 256 色
                            let idx = params[i + 2] as usize;
                            self.foreground = color_256(idx);
                            i += 2;
                        }
                        2 if i + 4 < params.len() => {
                            // True color
                            self.foreground = Color::new(
                                params[i + 2] as u8,
                                params[i + 3] as u8,
                                params[i + 4] as u8,
                            );
                            i += 4;
                        }
                        _ => {}
                    }
                }
                39 => self.foreground = self.default_foreground,
                40..=47 => {
                    // 背景色标准 8 色
                    self.background = ANSI_COLORS[(p - 40) as usize];
                }
                48 if i + 1 < params.len() => {
                    // 扩展背景色
                    match params[i + 1] {
                        5 if i + 2 < params.len() => {
                            let idx = params[i + 2] as usize;
                            self.background = color_256(idx);
                            i += 2;
                        }
                        2 if i + 4 < params.len() => {
                            self.background = Color::new(
                                params[i + 2] as u8,
                                params[i + 3] as u8,
                                params[i + 4] as u8,
                            );
                            i += 4;
                        }
                        _ => {}
                    }
                }
                49 => self.background = self.default_background,
                90..=97 => {
                    // 前景色高亮 8 色
                    self.foreground = ANSI_COLORS[(p - 90 + 8) as usize];
                }
                100..=107 => {
                    // 背景色高亮 8 色
                    self.background = ANSI_COLORS[(p - 100 + 8) as usize];
                }
                _ => {}
            }
            i += 1;
        }
    }
}

/// 256 色查找
fn color_256(idx: usize) -> Color {
    if idx < 16 {
        ANSI_COLORS[idx]
    } else if idx < 232 {
        // 6x6x6 色立方体
        let idx = idx - 16;
        let b = idx % 6;
        let g = (idx / 6) % 6;
        let r = idx / 36;
        Color::new(
            if r == 0 { 0 } else { 55 + r as u8 * 40 },
            if g == 0 { 0 } else { 55 + g as u8 * 40 },
            if b == 0 { 0 } else { 55 + b as u8 * 40 },
        )
    } else {
        // 灰度 24 级
        let v = 8 + (idx - 232) as u8 * 10;
        Color::new(v, v, v)
    }
}

/// vte Perform 实现
impl vte::Perform for Terminal {
    fn print(&mut self, c: char) {
        self.put_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            0x08 => self.backspace(),       // BS
            0x09 => self.tab(),             // HT
            0x0A => self.newline(),         // LF
            0x0D => self.carriage_return(), // CR
            _ => {}
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let p: Vec<u64> = params
            .iter()
            .flat_map(|s| s.iter().copied().map(|v| v as u64))
            .collect();

        match action {
            // Cursor Up
            'A' => {
                let n = param_or(&p, 0, 1).max(1) as usize;
                self.cursor.row = self.cursor.row.saturating_sub(n);
                self.mark_dirty(self.cursor.row);
            }
            // Cursor Down
            'B' => {
                let n = param_or(&p, 0, 1).max(1) as usize;
                self.cursor.row = (self.cursor.row + n).min(self.grid.rows() - 1);
                self.mark_dirty(self.cursor.row);
            }
            // Cursor Forward
            'C' => {
                let n = param_or(&p, 0, 1).max(1) as usize;
                self.cursor.col = (self.cursor.col + n).min(self.grid.cols() - 1);
                self.mark_dirty(self.cursor.row);
            }
            // Cursor Back
            'D' => {
                let n = param_or(&p, 0, 1).max(1) as usize;
                self.cursor.col = self.cursor.col.saturating_sub(n);
                self.mark_dirty(self.cursor.row);
            }
            // Cursor Next Line
            'E' => {
                let n = param_or(&p, 0, 1).max(1) as usize;
                self.cursor.row = (self.cursor.row + n).min(self.grid.rows() - 1);
                self.cursor.col = 0;
                self.mark_dirty(self.cursor.row);
            }
            // Cursor Previous Line
            'F' => {
                let n = param_or(&p, 0, 1).max(1) as usize;
                self.cursor.row = self.cursor.row.saturating_sub(n);
                self.cursor.col = 0;
                self.mark_dirty(self.cursor.row);
            }
            // Cursor Position (CUP)
            'H' | 'f' => {
                let row = param_or(&p, 0, 1).max(1) as usize - 1;
                let col = param_or(&p, 1, 1).max(1) as usize - 1;
                self.cursor.row = row.min(self.grid.rows() - 1);
                self.cursor.col = col.min(self.grid.cols() - 1);
                self.mark_dirty(self.cursor.row);
            }
            // Erase in Display (ED)
            'J' => {
                let mode = param_or(&p, 0, 0);
                match mode {
                    0 => self.grid.clear_below(self.cursor.row, self.cursor.col),
                    1 => self.grid.clear_above(self.cursor.row, self.cursor.col),
                    2 => self.grid.clear_all(),
                    3 => {
                        // 清除 scrollback
                        self.grid.clear_scrollback();
                    }
                    _ => {}
                }
                self.damage.mark_full_redraw();
            }
            // Erase in Line (EL)
            'K' => {
                let mode = param_or(&p, 0, 0);
                match mode {
                    0 => self.grid.clear_right(self.cursor.row, self.cursor.col),
                    1 => self.grid.clear_left(self.cursor.row, self.cursor.col),
                    2 => self.grid.clear_row(self.cursor.row),
                    _ => {}
                }
                self.mark_dirty(self.cursor.row);
            }
            // Insert Lines
            'L' => {
                let n = param_or(&p, 0, 1).max(1);
                for _ in 0..n {
                    self.grid.scroll_down();
                }
                self.damage.mark_full_redraw();
            }
            // Delete Lines
            'M' => {
                let n = param_or(&p, 0, 1).max(1);
                for _ in 0..n {
                    self.grid.scroll_up();
                }
                self.damage.mark_full_redraw();
            }
            // Set Scrolling Region (DECSTBM)
            'r' => {
                let top = param_or(&p, 0, 1) as usize;
                let bottom = if p.len() > 1 {
                    param_or(&p, 1, self.grid.rows() as u64) as usize
                } else {
                    self.grid.rows()
                };
                if top < bottom && bottom <= self.grid.rows() {
                    self.grid.set_scroll_region(top - 1, bottom - 1);
                }
                self.cursor.row = 0;
                self.cursor.col = 0;
                self.damage.mark_full_redraw();
            }
            // Save Cursor Position (DECSC)
            's' => {
                self.save_cursor();
            }
            // Restore Cursor Position (DECRC)
            'u' => {
                self.restore_cursor();
            }
            // SGR - Select Graphic Rendition
            'm' => {
                self.sgr(&p);
            }
            // Set Mode
            'h' if intermediates == [b'?'] => {
                for &mode in &p {
                    self.set_private_mode(mode as u16);
                }
            }
            // Reset Mode
            'l' if intermediates == [b'?'] => {
                for &mode in &p {
                    self.unset_private_mode(mode as u16);
                }
            }
            // Set Mode
            'h' => {
                for &mode in &p {
                    self.set_mode(mode as u16);
                }
            }
            // Reset Mode
            'l' => {
                for &mode in &p {
                    self.unset_mode(mode as u16);
                }
            }
            // Cursor Shape: DECSCUSR — CSI Ps SP q
            // vte-rs 将中间键 ' ' (SP) 和 final char 'q' 一起处理
            // 当 intermediates 包含 SP 且 action == 'q' 时匹配
            'q' => {
                if let Some(&shape) = p.first() {
                    self.cursor.shape = match shape {
                        0 | 1 => CursorShape::Block,
                        2 => CursorShape::Underline,
                        3 | 4 => CursorShape::Beam,
                        _ => CursorShape::Block,
                    };
                }
            }
            _ => {
                tracing::trace!("Unhandled CSI: {:?} {}", p, action);
            }
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        // OSC 格式: OSC <command> ; <data> ST
        if params.len() < 2 {
            return;
        }

        let command = std::str::from_utf8(params[0]).unwrap_or("");
        let data = std::str::from_utf8(params[1]).unwrap_or("");

        match command {
            "0" | "2" => {
                // 设置窗口标题
                self.title = data.to_string();
            }
            _ => {
                tracing::trace!("Unhandled OSC: {} {}", command, data);
            }
        }
    }
}

/// 获取参数，若不存在则用默认值
fn param_or(params: &[u64], idx: usize, default: u64) -> u64 {
    params.get(idx).copied().unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_output() {
        let mut handler = TerminalHandler::new(24, 80, 1000);
        handler.process(b"Hello, World!");
        let cell = handler.terminal.grid.cell(0, 0).unwrap();
        assert_eq!(cell.character, 'H');
        let cell = handler.terminal.grid.cell(0, 12).unwrap();
        assert_eq!(cell.character, '!');
    }

    #[test]
    fn test_newline() {
        let mut handler = TerminalHandler::new(24, 80, 1000);
        handler.process(b"Line1\r\nLine2");
        let cell = handler.terminal.grid.cell(1, 0).unwrap();
        assert_eq!(cell.character, 'L');
    }

    #[test]
    fn test_cursor_movement() {
        let mut handler = TerminalHandler::new(24, 80, 1000);
        handler.process(b"\x1b[5;10H"); // CUP row=5 col=10
        assert_eq!(handler.terminal.cursor.row, 4);
        assert_eq!(handler.terminal.cursor.col, 9);
    }

    #[test]
    fn test_sgr_colors() {
        let mut handler = TerminalHandler::new(24, 80, 1000);
        handler.process(b"\x1b[31mRed\x1b[0m");
        let cell = handler.terminal.grid.cell(0, 0).unwrap();
        assert_eq!(cell.foreground, ANSI_COLORS[1]); // Red
    }

    #[test]
    fn test_clear_screen() {
        let mut handler = TerminalHandler::new(24, 80, 1000);
        handler.process(b"Text\x1b[2J");
        let cell = handler.terminal.grid.cell(0, 0).unwrap();
        assert_eq!(cell.character, ' ');
    }

    #[test]
    fn test_custom_theme_defaults() {
        let custom_fg = Color::new(0xAA, 0xBB, 0xCC);
        let custom_bg = Color::new(0x11, 0x22, 0x33);
        let mut handler = TerminalHandler::with_theme(24, 80, 1000, custom_fg, custom_bg);

        handler.process(b"\x1b[31mRed\x1b[0m ");
        let cell = handler.terminal.grid.cell(0, 3).unwrap();

        assert_eq!(cell.foreground, custom_fg);
        assert_eq!(cell.background, custom_bg);
    }

    #[test]
    fn test_application_cursor_private_mode() {
        let mut handler = TerminalHandler::new(24, 80, 1000);
        handler.process(b"\x1b[?1h");
        assert!(handler.terminal.application_cursor_keys());

        handler.process(b"\x1b[?1l");
        assert!(!handler.terminal.application_cursor_keys());
    }

    #[test]
    fn test_cursor_visibility_private_mode() {
        let mut handler = TerminalHandler::new(24, 80, 1000);
        assert!(handler.terminal.cursor.visible);

        handler.process(b"\x1b[?25l");
        assert!(!handler.terminal.cursor.visible);

        handler.process(b"\x1b[?25h");
        assert!(handler.terminal.cursor.visible);
    }

    #[test]
    fn test_alternate_screen_restores_primary_contents() {
        let mut handler = TerminalHandler::new(4, 8, 100);
        handler.process(b"main");

        handler.process(b"\x1b[?1049h");
        assert!(handler.terminal.alternate_screen());
        let alt_cell = handler.terminal.grid.cell(0, 0).unwrap();
        assert_eq!(alt_cell.character, ' ');

        handler.process(b"alt");
        handler.process(b"\x1b[?1049l");

        assert!(!handler.terminal.alternate_screen());
        let main_cell = handler.terminal.grid.cell(0, 0).unwrap();
        assert_eq!(main_cell.character, 'm');
    }

    #[test]
    fn test_mouse_and_bracketed_paste_private_modes() {
        let mut handler = TerminalHandler::new(24, 80, 1000);

        handler.process(b"\x1b[?1002h\x1b[?1006h\x1b[?2004h");
        assert_eq!(
            handler.terminal.mouse_tracking_mode(),
            MouseTrackingMode::ButtonEvent
        );
        assert!(handler.terminal.mouse_sgr_mode());
        assert!(handler.terminal.bracketed_paste_mode());

        handler.process(b"\x1b[?1002l\x1b[?1006l\x1b[?2004l");
        assert_eq!(
            handler.terminal.mouse_tracking_mode(),
            MouseTrackingMode::Disabled
        );
        assert!(!handler.terminal.mouse_sgr_mode());
        assert!(!handler.terminal.bracketed_paste_mode());
    }

    #[test]
    fn test_ime_preedit_state() {
        let mut handler = TerminalHandler::new(24, 80, 1000);
        handler
            .terminal
            .set_ime_preedit("ni".to_string(), Some((1, 1)));
        let preedit = handler.terminal.ime_preedit().unwrap();
        assert_eq!(preedit.text, "ni");
        assert_eq!(preedit.cursor_range, Some((1, 1)));

        handler.terminal.clear_ime_preedit();
        assert!(handler.terminal.ime_preedit().is_none());
    }
}

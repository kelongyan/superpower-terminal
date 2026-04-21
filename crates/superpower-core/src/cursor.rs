/// 光标样式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorShape {
    #[default]
    Block,
    Underline,
    Beam,
}

/// 光标状态
#[derive(Debug, Clone)]
pub struct Cursor {
    /// 列位置（0-based）
    pub col: usize,
    /// 行位置（0-based）
    pub row: usize,
    /// 光标样式
    pub shape: CursorShape,
    /// 是否可见
    pub visible: bool,
    /// 是否处于应用模式（ keypad ）
    pub app_mode: bool,
}

impl Cursor {
    pub fn new() -> Self {
        Self {
            col: 0,
            row: 0,
            shape: CursorShape::default(),
            visible: true,
            app_mode: false,
        }
    }

    /// 限制光标在给定范围内
    pub fn clamp(&mut self, rows: usize, cols: usize) {
        if rows > 0 {
            self.row = self.row.min(rows - 1);
        }
        if cols > 0 {
            self.col = self.col.min(cols - 1);
        }
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self::new()
    }
}

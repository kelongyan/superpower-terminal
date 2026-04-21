use std::ops::{BitAnd, BitOr, BitOrAssign, Not};

/// Cell 标志位
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CellFlags(u16);

impl CellFlags {
    pub const EMPTY: CellFlags = CellFlags(0);
    pub const BOLD: CellFlags = CellFlags(1 << 0);
    pub const ITALIC: CellFlags = CellFlags(1 << 1);
    pub const UNDERLINE: CellFlags = CellFlags(1 << 2);
    pub const BLINK: CellFlags = CellFlags(1 << 3);
    pub const REVERSE: CellFlags = CellFlags(1 << 4);
    pub const WIDE: CellFlags = CellFlags(1 << 5);
    pub const WIDE_CHAR_SPACER: CellFlags = CellFlags(1 << 6);
    pub const DIM: CellFlags = CellFlags(1 << 7);
    pub const STRIKETHROUGH: CellFlags = CellFlags(1 << 8);
    pub const DOUBLE_UNDERLINE: CellFlags = CellFlags(1 << 9);
    pub const HIDDEN: CellFlags = CellFlags(1 << 10);
}

impl BitOrAssign for CellFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitOr for CellFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        CellFlags(self.0 | rhs.0)
    }
}

impl BitAnd for CellFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        CellFlags(self.0 & rhs.0)
    }
}

impl Not for CellFlags {
    type Output = Self;
    fn not(self) -> Self::Output {
        CellFlags(!self.0)
    }
}

/// RGB 颜色
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub const fn from_u32(v: u32) -> Self {
        Self {
            r: ((v >> 16) & 0xFF) as u8,
            g: ((v >> 8) & 0xFF) as u8,
            b: (v & 0xFF) as u8,
        }
    }
}

/// 终端默认颜色
impl Color {
    pub const DEFAULT_FG: Color = Color::new(0xD4, 0xD4, 0xD4);
    pub const DEFAULT_BG: Color = Color::new(0x1E, 0x1E, 0x1E);
}

/// 单个终端 Cell
#[derive(Debug, Clone)]
pub struct Cell {
    /// 字符（占位字符用 '\0'）
    pub character: char,
    /// 前景色
    pub foreground: Color,
    /// 背景色
    pub background: Color,
    /// 标志位
    pub flags: CellFlags,
}

impl Cell {
    /// 创建带指定颜色的 Cell
    pub fn with_colors(character: char, foreground: Color, background: Color) -> Self {
        Self {
            character,
            foreground,
            background,
            flags: CellFlags::EMPTY,
        }
    }

    /// 创建带默认颜色的空 Cell
    pub fn new(character: char) -> Self {
        Self::with_colors(character, Color::DEFAULT_FG, Color::DEFAULT_BG)
    }

    /// 创建空白 Cell（空格）
    pub fn blank() -> Self {
        Self::new(' ')
    }

    /// 创建带指定默认颜色的空白 Cell
    pub fn blank_with_colors(foreground: Color, background: Color) -> Self {
        Self::with_colors(' ', foreground, background)
    }

    /// 重置为空白
    pub fn reset(&mut self) {
        self.reset_with_colors(Color::DEFAULT_FG, Color::DEFAULT_BG);
    }

    /// 使用指定默认颜色重置为空白
    pub fn reset_with_colors(&mut self, foreground: Color, background: Color) {
        self.character = ' ';
        self.foreground = foreground;
        self.background = background;
        self.flags = CellFlags::EMPTY;
    }

    pub fn is_wide(&self) -> bool {
        (self.flags & CellFlags::WIDE) != CellFlags::EMPTY
    }

    pub fn is_wide_spacer(&self) -> bool {
        (self.flags & CellFlags::WIDE_CHAR_SPACER) != CellFlags::EMPTY
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::blank()
    }
}

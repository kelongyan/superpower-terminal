pub mod cell;
pub mod cursor;
pub mod damage;
pub mod grid;
pub mod parser;
pub mod selection;
pub mod terminal;

// 顶层重导出常用类型
pub use cell::{Cell, CellFlags, Color};
pub use cursor::{Cursor, CursorShape};
pub use damage::DamageTracker;
pub use grid::{Grid, Row};
pub use parser::{Terminal, TerminalHandler};
pub use selection::{cell_bounds, line_bounds, word_bounds, Selection, SelectionPos};

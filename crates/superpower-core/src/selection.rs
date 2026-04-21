use crate::cell::CellFlags;
use crate::grid::Grid;

/// 选区位置
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionPos {
    pub row: usize,
    pub col: usize,
}

impl SelectionPos {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}

impl PartialOrd for SelectionPos {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SelectionPos {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.row.cmp(&other.row) {
            std::cmp::Ordering::Equal => self.col.cmp(&other.col),
            ord => ord,
        }
    }
}

/// 选区
#[derive(Debug, Clone)]
pub struct Selection {
    /// 选区起始
    start: SelectionPos,
    /// 选区结束（含）
    end: SelectionPos,
}

impl Selection {
    pub fn new(start: SelectionPos, end: SelectionPos) -> Self {
        // 确保 start <= end
        if start <= end {
            Self { start, end }
        } else {
            Self {
                start: end,
                end: start,
            }
        }
    }

    pub fn start(&self) -> &SelectionPos {
        &self.start
    }

    pub fn end(&self) -> &SelectionPos {
        &self.end
    }

    /// 检查指定位置是否在选区内
    pub fn contains(&self, row: usize, col: usize) -> bool {
        let pos = SelectionPos::new(row, col);
        pos >= self.start && pos <= self.end
    }

    /// 检查指定行是否与选区有交集
    pub fn intersects_row(&self, row: usize) -> bool {
        row >= self.start.row && row <= self.end.row
    }

    /// 获取选区文本
    pub fn text(&self, grid: &Grid) -> String {
        let visible = grid.visible_lines();
        let mut result = String::new();

        for row in self.start.row..=self.end.row {
            if row >= visible.len() {
                break;
            }
            let line = &visible[row];

            let col_start = if row == self.start.row {
                self.start.col
            } else {
                0
            };
            let col_end = if row == self.end.row {
                self.end.col.min(line.len().saturating_sub(1))
            } else {
                line.len().saturating_sub(1)
            };

            for col in col_start..=col_end {
                if col < line.len() {
                    let cell = &line[col];
                    if cell.character != '\0'
                        && !(cell.flags & CellFlags::WIDE_CHAR_SPACER != CellFlags::EMPTY)
                    {
                        result.push(cell.character);
                    }
                }
            }

            if row != self.end.row {
                result.push('\n');
            }
        }

        result
    }
}

/// 获取某个单词语义边界，返回包含整词的起止位置
pub fn cell_bounds(grid: &Grid, row: usize, col: usize) -> Option<(SelectionPos, SelectionPos)> {
    let visible = grid.visible_lines();
    let line = visible.get(row)?;
    if line.is_empty() {
        return Some((SelectionPos::new(row, 0), SelectionPos::new(row, 0)));
    }

    let lead = normalize_to_lead(line, col.min(line.len().saturating_sub(1)));
    Some((
        SelectionPos::new(row, lead),
        SelectionPos::new(row, logical_end_col(line, lead)),
    ))
}

/// 获取某个单词语义边界，返回包含整词的起止位置
pub fn word_bounds(grid: &Grid, row: usize, col: usize) -> Option<(SelectionPos, SelectionPos)> {
    let visible = grid.visible_lines();
    let line = visible.get(row)?;
    if line.is_empty() {
        return Some((SelectionPos::new(row, 0), SelectionPos::new(row, 0)));
    }

    let anchor = normalize_to_lead(line, col.min(line.len().saturating_sub(1)));
    let class = classify_cell(line, anchor)?;

    let mut start = anchor;
    while let Some(prev) = previous_logical_col(line, start) {
        if classify_cell(line, prev) != Some(class) {
            break;
        }
        start = prev;
    }

    let mut end_lead = anchor;
    while let Some(next) = next_logical_col(line, end_lead) {
        if classify_cell(line, next) != Some(class) {
            break;
        }
        end_lead = next;
    }

    Some((
        SelectionPos::new(row, start),
        SelectionPos::new(row, logical_end_col(line, end_lead)),
    ))
}

/// 获取整行语义边界，默认忽略行尾填充空白
pub fn line_bounds(grid: &Grid, row: usize) -> Option<(SelectionPos, SelectionPos)> {
    let visible = grid.visible_lines();
    let line = visible.get(row)?;
    if line.is_empty() {
        return Some((SelectionPos::new(row, 0), SelectionPos::new(row, 0)));
    }

    let mut last_content_col = None;
    let mut col = 0;
    while col < line.len() {
        let lead = normalize_to_lead(line, col);
        let cell = &line[lead];
        if cell.character != ' ' && cell.character != '\0' && !cell.is_wide_spacer() {
            last_content_col = Some(logical_end_col(line, lead));
        }

        col = match next_logical_col(line, lead) {
            Some(next) => next,
            None => break,
        };
    }

    let end = last_content_col.unwrap_or(0);
    Some((SelectionPos::new(row, 0), SelectionPos::new(row, end)))
}

/// 词法分类，用于双击选词时扩展连续区间
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenClass {
    Word,
    Whitespace,
    Symbol,
}

/// 将落在宽字符占位单元格上的列修正到实际字符起始列
fn normalize_to_lead(line: &[crate::cell::Cell], mut col: usize) -> usize {
    col = col.min(line.len().saturating_sub(1));
    while col > 0 && line[col].is_wide_spacer() {
        col -= 1;
    }
    col
}

/// 获取一个逻辑字符占据的结束列，宽字符会覆盖到 spacer 列
fn logical_end_col(line: &[crate::cell::Cell], col: usize) -> usize {
    if line.get(col).is_some_and(|cell| cell.is_wide()) && col + 1 < line.len() {
        col + 1
    } else {
        col
    }
}

/// 获取前一个逻辑字符的起始列
fn previous_logical_col(line: &[crate::cell::Cell], col: usize) -> Option<usize> {
    if col == 0 {
        return None;
    }

    let mut prev = col - 1;
    while prev > 0 && line[prev].is_wide_spacer() {
        prev -= 1;
    }
    Some(prev)
}

/// 获取后一个逻辑字符的起始列
fn next_logical_col(line: &[crate::cell::Cell], col: usize) -> Option<usize> {
    let next = logical_end_col(line, col).saturating_add(1);
    (next < line.len()).then_some(next)
}

/// 读取指定列对应字符的词法分类
fn classify_cell(line: &[crate::cell::Cell], col: usize) -> Option<TokenClass> {
    let col = normalize_to_lead(line, col);
    let cell = line.get(col)?;

    if cell.is_wide_spacer() {
        return None;
    }

    let ch = cell.character;
    if ch.is_whitespace() {
        Some(TokenClass::Whitespace)
    } else if ch.is_alphanumeric() || ch == '_' || (!ch.is_ascii() && !is_symbol_char(ch)) {
        Some(TokenClass::Word)
    } else {
        Some(TokenClass::Symbol)
    }
}

/// 判断字符是否更接近符号而不是“词字符”
fn is_symbol_char(ch: char) -> bool {
    !ch.is_whitespace() && !ch.is_alphanumeric() && ch != '_'
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::Cell;

    /// 验证滚动视口中的选区复制会基于当前可见窗口取文本
    #[test]
    fn selection_text_uses_visible_viewport() {
        let mut grid = Grid::new(2, 1, 10);
        grid.write_cell(0, 0, Cell::new('A'));
        grid.write_cell(1, 0, Cell::new('B'));
        grid.scroll_up();
        grid.write_cell(1, 0, Cell::new('C'));
        grid.scroll_display_up(1);

        let selection = Selection::new(SelectionPos::new(0, 0), SelectionPos::new(1, 0));
        assert_eq!(selection.text(&grid), "A\nB");
    }

    /// 验证双击选词会按连续词法单元扩展
    #[test]
    fn word_bounds_expand_full_word() {
        let mut grid = Grid::new(1, 16, 10);
        for (idx, ch) in "hello world".chars().enumerate() {
            grid.write_cell(0, idx, Cell::new(ch));
        }

        let (start, end) = word_bounds(&grid, 0, 7).unwrap();
        assert_eq!(start, SelectionPos::new(0, 6));
        assert_eq!(end, SelectionPos::new(0, 10));
    }

    /// 验证三击选行会忽略行尾填充空白
    #[test]
    fn line_bounds_trim_trailing_padding() {
        let mut grid = Grid::new(1, 8, 10);
        for (idx, ch) in "abc".chars().enumerate() {
            grid.write_cell(0, idx, Cell::new(ch));
        }

        let (start, end) = line_bounds(&grid, 0).unwrap();
        assert_eq!(start, SelectionPos::new(0, 0));
        assert_eq!(end, SelectionPos::new(0, 2));
    }
}

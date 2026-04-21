use crate::cell::{Cell, Color};

/// 一行 Cell
pub type Row = Vec<Cell>;

/// 终端网格 — 存储所有可见 Cell 及滚动缓冲
#[derive(Debug)]
pub struct Grid {
    /// 可见行
    lines: Vec<Row>,
    /// 列数
    cols: usize,
    /// 行数
    rows: usize,
    /// 滚动缓冲区
    scrollback: Vec<Row>,
    /// 滚动缓冲区上限
    scrollback_limit: usize,
    /// 滚动区域起始行
    scroll_top: usize,
    /// 滚动区域结束行（含）
    scroll_bottom: usize,
    /// 视口偏移 — 向上滚动到 scrollback 中的行数（0 = 底部/最新）
    display_offset: usize,
    /// 当前 Grid 使用的默认空白 Cell 模板
    template_cell: Cell,
}

impl Grid {
    pub fn new(rows: usize, cols: usize, scrollback_limit: usize) -> Self {
        Self::with_colors(
            rows,
            cols,
            scrollback_limit,
            Color::DEFAULT_FG,
            Color::DEFAULT_BG,
        )
    }

    /// 使用指定默认前景色/背景色创建 Grid
    pub fn with_colors(
        rows: usize,
        cols: usize,
        scrollback_limit: usize,
        default_foreground: Color,
        default_background: Color,
    ) -> Self {
        let template_cell = Cell::blank_with_colors(default_foreground, default_background);
        let lines = (0..rows).map(|_| new_row(cols, &template_cell)).collect();
        Self {
            lines,
            cols,
            rows,
            scrollback: Vec::new(),
            scrollback_limit,
            scroll_top: 0,
            scroll_bottom: rows.saturating_sub(1),
            display_offset: 0,
            template_cell,
        }
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }

    pub fn scrollback(&self) -> &[Row] {
        &self.scrollback
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn scroll_top(&self) -> usize {
        self.scroll_top
    }

    pub fn scroll_bottom(&self) -> usize {
        self.scroll_bottom
    }

    /// 获取指定行的引用
    pub fn row(&self, row: usize) -> Option<&Row> {
        self.lines.get(row)
    }

    /// 获取指定行的可变引用
    pub fn row_mut(&mut self, row: usize) -> Option<&mut Row> {
        self.lines.get_mut(row)
    }

    /// 获取指定 Cell 的引用
    pub fn cell(&self, row: usize, col: usize) -> Option<&Cell> {
        self.lines.get(row).and_then(|r| r.get(col))
    }

    /// 获取指定 Cell 的可变引用
    pub fn cell_mut(&mut self, row: usize, col: usize) -> Option<&mut Cell> {
        self.lines.get_mut(row).and_then(|r| r.get_mut(col))
    }

    /// 在指定位置写入字符
    pub fn write_cell(&mut self, row: usize, col: usize, cell: Cell) {
        if let Some(r) = self.lines.get_mut(row) {
            if col < r.len() {
                r[col] = cell;
            }
        }
    }

    /// 设置滚动区域
    pub fn set_scroll_region(&mut self, top: usize, bottom: usize) {
        self.scroll_top = top.min(self.rows.saturating_sub(1));
        self.scroll_bottom = bottom.min(self.rows.saturating_sub(1));
    }

    /// 重置滚动区域为全屏
    pub fn reset_scroll_region(&mut self) {
        self.scroll_top = 0;
        self.scroll_bottom = self.rows.saturating_sub(1);
    }

    /// 向上滚动一行（在滚动区域内）
    /// 返回被滚出的行（如果有的话）
    pub fn scroll_up(&mut self) {
        let top = self.scroll_top;
        let bottom = self.scroll_bottom;

        if top >= bottom || top >= self.rows {
            return;
        }

        // 滚出顶部行到 scrollback
        if self.scroll_top == 0 {
            let removed =
                std::mem::replace(&mut self.lines[0], new_row(self.cols, &self.template_cell));
            if self.scrollback_limit > 0 {
                self.scrollback.push(removed);
                while self.scrollback.len() > self.scrollback_limit {
                    self.scrollback.remove(0);
                }
            }
        }

        // 行上移
        for row in top..bottom {
            if row + 1 < self.rows {
                let (left, right) = self.lines.split_at_mut(row + 1);
                std::mem::swap(&mut left[row], &mut right[0]);
            }
        }

        // 清空底部行
        self.lines[bottom] = new_row(self.cols, &self.template_cell);
    }

    /// 向下滚动一行（在滚动区域内）
    pub fn scroll_down(&mut self) {
        let top = self.scroll_top;
        let bottom = self.scroll_bottom;

        if top >= bottom || bottom >= self.rows {
            return;
        }

        // 行下移
        for row in (top + 1..=bottom).rev() {
            let (left, right) = self.lines.split_at_mut(row);
            std::mem::swap(&mut left[row - 1], &mut right[0]);
        }

        // 清空顶部行
        self.lines[top] = new_row(self.cols, &self.template_cell);
    }

    /// 调整 Grid 大小
    pub fn resize(&mut self, new_rows: usize, new_cols: usize) {
        // 调整列数
        if new_cols != self.cols {
            for row in &mut self.lines {
                row.resize(new_cols, self.template_cell.clone());
            }
            self.cols = new_cols;
        }

        // 调整行数
        if new_rows != self.rows {
            if new_rows > self.rows {
                // 扩展行
                for _ in 0..(new_rows - self.rows) {
                    self.lines.push(new_row(new_cols, &self.template_cell));
                }
            } else {
                // 缩减行（多余行进入 scrollback）
                while self.lines.len() > new_rows {
                    let removed = self.lines.pop().unwrap();
                    if self.scrollback_limit > 0 {
                        self.scrollback.push(removed);
                    }
                }
            }
            self.rows = new_rows;
        }

        self.scroll_bottom = self.scroll_bottom.min(new_rows.saturating_sub(1));
        self.display_offset = self.display_offset.min(self.scrollback.len());
    }

    /// 清除整行
    pub fn clear_row(&mut self, row: usize) {
        if row < self.rows {
            self.lines[row] = new_row(self.cols, &self.template_cell);
        }
    }

    /// 清除从 (row, col) 到行尾
    pub fn clear_right(&mut self, row: usize, col: usize) {
        if let Some(r) = self.lines.get_mut(row) {
            for c in &mut r[col..] {
                *c = self.template_cell.clone();
            }
        }
    }

    /// 清除从行首到 (row, col)
    pub fn clear_left(&mut self, row: usize, col: usize) {
        if let Some(r) = self.lines.get_mut(row) {
            let end = col.min(r.len().saturating_sub(1));
            for c in &mut r[..=end] {
                *c = self.template_cell.clone();
            }
        }
    }

    /// 清除整屏
    pub fn clear_all(&mut self) {
        for row in &mut self.lines {
            *row = new_row(self.cols, &self.template_cell);
        }
    }

    /// 清除滚动区域以下的所有内容
    pub fn clear_below(&mut self, row: usize, col: usize) {
        // 当前行从 col 到行尾
        self.clear_right(row, col);
        // 下方所有行
        for r in (row + 1)..self.rows {
            self.clear_row(r);
        }
    }

    /// 清除滚动区域以上的所有内容
    pub fn clear_above(&mut self, row: usize, col: usize) {
        // 上方所有行
        for r in 0..row {
            self.clear_row(r);
        }
        // 当前行从行首到 col
        self.clear_left(row, col);
    }

    /// 清除滚动缓冲区
    pub fn clear_scrollback(&mut self) {
        self.scrollback.clear();
        self.display_offset = 0;
    }

    /// 获取视口偏移
    pub fn display_offset(&self) -> usize {
        self.display_offset
    }

    /// 向上滚动视口（查看历史输出）
    pub fn scroll_display_up(&mut self, lines: usize) {
        let max_offset = self.scrollback.len();
        self.display_offset = (self.display_offset + lines).min(max_offset);
    }

    /// 向下滚动视口（回到最新输出）
    pub fn scroll_display_down(&mut self, lines: usize) {
        self.display_offset = self.display_offset.saturating_sub(lines);
    }

    /// 重置视口到底部（最新输出）
    pub fn reset_display_offset(&mut self) {
        self.display_offset = 0;
    }

    /// 是否处于滚动状态（视口不在底部）
    pub fn is_scrolled(&self) -> bool {
        self.display_offset > 0
    }

    /// 获取可见行数据的不可变切片
    pub fn lines(&self) -> &[Row] {
        &self.lines
    }

    /// 获取显示行（包含 scrollback 视口偏移）
    /// 返回一个 Vec，长度等于 rows，其中每行可能是 scrollback 中的行或可见行
    pub fn visible_lines(&self) -> Vec<&Row> {
        let mut result = Vec::with_capacity(self.rows);
        let total_lines = self.scrollback.len() + self.lines.len();
        let visible_count = self.rows.min(total_lines);
        let effective_offset = self.display_offset.min(self.scrollback.len());
        let end = total_lines.saturating_sub(effective_offset);
        let start = end.saturating_sub(visible_count);

        for index in start..end {
            if index < self.scrollback.len() {
                result.push(&self.scrollback[index]);
            } else {
                result.push(&self.lines[index - self.scrollback.len()]);
            }
        }

        result
    }

    /// 当有新的 PTY 输出时，应该重置视口到底部
    /// 在 scroll_up 被调用时自动重置
    pub fn scroll_up_reset_view(&mut self) {
        if self.display_offset > 0 {
            self.display_offset = 0;
        }
    }
}

fn new_row(cols: usize, template_cell: &Cell) -> Row {
    vec![template_cell.clone(); cols]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证视口滚动后仍然只返回当前窗口高度的可见行
    #[test]
    fn visible_lines_respects_window_height() {
        let mut grid = Grid::new(3, 2, 10);

        for row in 0..3 {
            for col in 0..2 {
                grid.write_cell(row, col, Cell::new(char::from(b'a' + row as u8)));
            }
        }

        grid.scrollback.push(vec![Cell::new('1'), Cell::new('1')]);
        grid.scrollback.push(vec![Cell::new('2'), Cell::new('2')]);
        grid.scrollback.push(vec![Cell::new('3'), Cell::new('3')]);
        grid.scroll_display_up(3);

        let visible = grid.visible_lines();
        assert_eq!(visible.len(), 3);
        assert_eq!(visible[0][0].character, '1');
        assert_eq!(visible[1][0].character, '2');
        assert_eq!(visible[2][0].character, '3');
    }

    /// 验证自定义默认颜色会被用于新建空白 Cell
    #[test]
    fn grid_uses_custom_default_colors() {
        let fg = Color::new(0xAA, 0xBB, 0xCC);
        let bg = Color::new(0x11, 0x22, 0x33);
        let grid = Grid::with_colors(2, 2, 10, fg, bg);

        let cell = grid.cell(0, 0).unwrap();
        assert_eq!(cell.foreground, fg);
        assert_eq!(cell.background, bg);
        assert_eq!(cell.character, ' ');
    }
}

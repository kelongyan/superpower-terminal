/// 损伤追踪器 — 标记 Grid 中需要重绘的脏区域
#[derive(Debug)]
pub struct DamageTracker {
    /// 每行是否有脏 Cell
    dirty_rows: Vec<bool>,
    /// 是否需要全屏重绘
    full_redraw: bool,
    /// 当前行数
    num_rows: usize,
}

impl DamageTracker {
    pub fn new(rows: usize) -> Self {
        Self {
            dirty_rows: vec![false; rows],
            full_redraw: false,
            num_rows: rows,
        }
    }

    /// 标记某行为脏
    pub fn mark_row(&mut self, row: usize) {
        if row < self.num_rows {
            self.dirty_rows[row] = true;
        }
    }

    /// 标记范围为脏
    pub fn mark_range(&mut self, start_row: usize, end_row: usize) {
        for row in start_row..=end_row.min(self.num_rows - 1) {
            self.dirty_rows[row] = true;
        }
    }

    /// 请求全屏重绘
    pub fn mark_full_redraw(&mut self) {
        self.full_redraw = true;
    }

    /// 是否需要全屏重绘
    pub fn needs_full_redraw(&self) -> bool {
        self.full_redraw
    }

    /// 获取所有脏行索引
    pub fn dirty_rows(&self) -> Vec<usize> {
        if self.full_redraw {
            (0..self.num_rows).collect()
        } else {
            self.dirty_rows
                .iter()
                .enumerate()
                .filter(|(_, dirty)| **dirty)
                .map(|(i, _)| i)
                .collect()
        }
    }

    /// 是否有任何脏区域
    pub fn is_dirty(&self) -> bool {
        self.full_redraw || self.dirty_rows.iter().any(|d| *d)
    }

    /// 渲染完成后清除所有脏标记
    pub fn clear(&mut self) {
        for d in &mut self.dirty_rows {
            *d = false;
        }
        self.full_redraw = false;
    }

    /// 调整大小
    pub fn resize(&mut self, rows: usize) {
        self.dirty_rows.resize(rows, true);
        self.num_rows = rows;
        self.full_redraw = true;
    }
}

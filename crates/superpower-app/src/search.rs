use regex::Regex;
use superpower_core::Grid;

/// 搜索匹配项
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMatch {
    /// 匹配所在行（相对于 scrollback + visible）
    pub row: usize,
    /// 匹配起始列
    pub col: usize,
    /// 匹配长度
    pub len: usize,
}

/// 搜索状态
#[derive(Debug, Clone)]
pub struct SearchState {
    /// 搜索查询字符串
    pub query: String,
    /// 是否区分大小写
    pub case_sensitive: bool,
    /// 是否使用正则表达式
    pub use_regex: bool,
    /// 所有匹配项
    pub matches: Vec<SearchMatch>,
    /// 当前选中的匹配索引
    pub current_match: Option<usize>,
}

impl SearchState {
    pub fn new(query: String, case_sensitive: bool, use_regex: bool) -> Self {
        Self {
            query,
            case_sensitive,
            use_regex,
            matches: Vec::new(),
            current_match: None,
        }
    }

    /// 执行搜索并更新匹配项
    pub fn search(&mut self, grid: &Grid) -> Result<(), String> {
        self.matches.clear();
        self.current_match = None;

        if self.query.is_empty() {
            return Ok(());
        }

        if self.use_regex {
            self.search_regex(grid)
        } else {
            self.search_literal(grid)
        }
    }

    /// 字面量搜索
    fn search_literal(&mut self, grid: &Grid) -> Result<(), String> {
        let query = if self.case_sensitive {
            self.query.clone()
        } else {
            self.query.to_lowercase()
        };

        // 搜索 scrollback
        for (row_idx, row) in grid.scrollback().iter().enumerate() {
            let text = row_to_string(row);
            let search_text = if self.case_sensitive {
                text.clone()
            } else {
                text.to_lowercase()
            };

            let mut start = 0;
            while let Some(pos) = search_text[start..].find(&query) {
                let col = start + pos;
                self.matches.push(SearchMatch {
                    row: row_idx,
                    col,
                    len: self.query.len(),
                });
                start = col + 1;
            }
        }

        // 搜索可见行
        let scrollback_len = grid.scrollback().len();
        for row_idx in 0..grid.rows() {
            if let Some(row) = grid.row(row_idx) {
                let text = row_to_string(row);
                let search_text = if self.case_sensitive {
                    text.clone()
                } else {
                    text.to_lowercase()
                };

                let mut start = 0;
                while let Some(pos) = search_text[start..].find(&query) {
                    let col = start + pos;
                    self.matches.push(SearchMatch {
                        row: scrollback_len + row_idx,
                        col,
                        len: self.query.len(),
                    });
                    start = col + 1;
                }
            }
        }

        if !self.matches.is_empty() {
            self.current_match = Some(0);
        }

        Ok(())
    }

    /// 正则表达式搜索
    fn search_regex(&mut self, grid: &Grid) -> Result<(), String> {
        let pattern = if self.case_sensitive {
            &self.query
        } else {
            &format!("(?i){}", self.query)
        };

        let regex = Regex::new(pattern).map_err(|e| format!("Invalid regex: {}", e))?;

        // 搜索 scrollback
        for (row_idx, row) in grid.scrollback().iter().enumerate() {
            let text = row_to_string(row);
            for mat in regex.find_iter(&text) {
                self.matches.push(SearchMatch {
                    row: row_idx,
                    col: mat.start(),
                    len: mat.end() - mat.start(),
                });
            }
        }

        // 搜索可见行
        let scrollback_len = grid.scrollback().len();
        for row_idx in 0..grid.rows() {
            if let Some(row) = grid.row(row_idx) {
                let text = row_to_string(row);
                for mat in regex.find_iter(&text) {
                    self.matches.push(SearchMatch {
                        row: scrollback_len + row_idx,
                        col: mat.start(),
                        len: mat.end() - mat.start(),
                    });
                }
            }
        }

        if !self.matches.is_empty() {
            self.current_match = Some(0);
        }

        Ok(())
    }

    /// 跳转到下一个匹配
    pub fn next_match(&mut self) {
        if self.matches.is_empty() {
            return;
        }

        self.current_match = Some(match self.current_match {
            Some(idx) => (idx + 1) % self.matches.len(),
            None => 0,
        });
    }

    /// 跳转到上一个匹配
    pub fn previous_match(&mut self) {
        if self.matches.is_empty() {
            return;
        }

        self.current_match = Some(match self.current_match {
            Some(0) => self.matches.len() - 1,
            Some(idx) => idx - 1,
            None => 0,
        });
    }

    /// 获取当前匹配项
    pub fn current(&self) -> Option<&SearchMatch> {
        self.current_match
            .and_then(|idx| self.matches.get(idx))
    }

    /// 获取匹配数量
    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    /// 获取当前匹配索引（1-based）
    pub fn current_index(&self) -> Option<usize> {
        self.current_match.map(|idx| idx + 1)
    }
}

/// 将行转换为字符串
fn row_to_string(row: &[superpower_core::Cell]) -> String {
    row.iter()
        .map(|cell| cell.character)
        .collect::<String>()
        .trim_end()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use superpower_core::Cell;

    fn create_test_grid() -> Grid {
        let mut grid = Grid::new(3, 10, 100);
        
        // 第一行: "hello world"
        for (i, ch) in "hello worl".chars().enumerate() {
            grid.write_cell(0, i, Cell::new(ch));
        }
        
        // 第二行: "test hello"
        for (i, ch) in "test hello".chars().enumerate() {
            grid.write_cell(1, i, Cell::new(ch));
        }
        
        // 第三行: "HELLO test"
        for (i, ch) in "HELLO test".chars().enumerate() {
            grid.write_cell(2, i, Cell::new(ch));
        }
        
        grid
    }

    #[test]
    fn test_literal_search_case_sensitive() {
        let grid = create_test_grid();
        let mut search = SearchState::new("hello".to_string(), true, false);
        search.search(&grid).unwrap();
        
        assert_eq!(search.match_count(), 2);
        assert_eq!(search.current_index(), Some(1));
    }

    #[test]
    fn test_literal_search_case_insensitive() {
        let grid = create_test_grid();
        let mut search = SearchState::new("hello".to_string(), false, false);
        search.search(&grid).unwrap();
        
        assert_eq!(search.match_count(), 3);
    }

    #[test]
    fn test_navigation() {
        let grid = create_test_grid();
        let mut search = SearchState::new("hello".to_string(), false, false);
        search.search(&grid).unwrap();
        
        assert_eq!(search.current_index(), Some(1));
        
        search.next_match();
        assert_eq!(search.current_index(), Some(2));
        
        search.next_match();
        assert_eq!(search.current_index(), Some(3));
        
        search.next_match();
        assert_eq!(search.current_index(), Some(1)); // 循环回第一个
        
        search.previous_match();
        assert_eq!(search.current_index(), Some(3));
    }

    #[test]
    fn test_empty_query() {
        let grid = create_test_grid();
        let mut search = SearchState::new("".to_string(), true, false);
        search.search(&grid).unwrap();
        
        assert_eq!(search.match_count(), 0);
    }
}

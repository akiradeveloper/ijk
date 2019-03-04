use crate::read_buffer::Cursor;
use crate::edit_buffer::{CursorRange, EditBuffer}; 
use crate::BufElem;

pub struct VisibilityFilter {
    cur_cursor: Cursor,
    pub col_low: usize,
    pub col_high: usize,
    pub row_low: usize,
    pub row_high: usize,
}

impl VisibilityFilter {
    pub fn new(cursor: Cursor) -> VisibilityFilter {
        VisibilityFilter {
            cur_cursor: cursor,
            col_low: 0, col_high: 0,
            row_low: 0, row_high: 0,
        }
    }
    pub fn resize(&mut self, width: usize, height: usize) {
        self.col_low = self.cur_cursor.col; // TODO should be able to be any value like 0
        self.col_high = self.cur_cursor.col + width - 1;
        self.row_low = self.cur_cursor.row;
        self.row_high = self.cur_cursor.row + height - 1;
    }
    pub fn col(&self) -> usize {
        self.col_low
    }
    pub fn row(&self) -> usize {
        self.row_low
    }
    fn width(&self) -> usize {
        self.col_high - self.col_low + 1
    }
    fn height(&self) -> usize {
        self.row_high - self.row_low + 1
    }
    pub fn adjust(&mut self, cursor: Cursor) {
        let prev_cursor = self.cur_cursor;
        self.cur_cursor = cursor;

        let width = self.width();
        let height = self.height();
        let col_diff: i32 = cursor.col as i32 - prev_cursor.col as i32;
        let row_diff: i32 = cursor.row as i32 - prev_cursor.row as i32;
        let col_ok = self.col_low <= cursor.col && cursor.col <= self.col_high;
        let row_ok = self.row_low <= cursor.row && cursor.row <= self.row_high;
        if col_ok && row_ok {
            return
        }
        if !col_ok {
            if col_diff > 0 {
                self.col_high = cursor.col;
                self.col_low = cursor.col - width + 1;
            } else {
                self.col_low = cursor.col;
                self.col_high = cursor.col + width - 1;
            }
        }
        if !row_ok {
            if row_diff > 0 {
                self.row_high = cursor.row;
                self.row_low = cursor.row - height + 1;
            } else {
                self.row_low = cursor.row;
                self.row_high = cursor.row + height - 1;
            }
        }
    }
}
use crate::view;
use crate::Cursor;

pub struct VisibilityWindow {
    cur_cursor: Cursor,
    pub col_low: usize,
    pub col_high: usize,
    pub row_low: usize,
    pub row_high: usize,
}

impl VisibilityWindow {
    pub fn new(cursor: Cursor) -> Self {
        Self {
            cur_cursor: cursor,
            col_low: 0, col_high: 0,
            row_low: 0, row_high: 0,
        }
    }
    pub fn area(&self) -> view::Area {
        view::Area {
            col: self.col(),
            row: self.row(),
            width: self.width(),
            height: self.height(),
        }
    }
    pub fn col(&self) -> usize {
        self.col_low
    }
    pub fn row(&self) -> usize {
        self.row_low
    }
    pub fn width(&self) -> usize {
        self.col_high - self.col_low + 1
    }
    pub fn height(&self) -> usize {
        self.row_high - self.row_low + 1
    }
    fn resize(&mut self, cursor: Cursor, width: usize, height: usize) {
        if self.width() == width && self.height() == height {
            return;
        }
        self.col_low = cursor.col; // TODO should be able to be any value like 0
        self.col_high = cursor.col + width - 1;
        self.row_low = cursor.row;
        self.row_high = cursor.row + height - 1;
        // self.cur_cursor = cursor;
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
    pub fn adjust_window(&mut self, cursor: Cursor, w: usize, h: usize) {
        self.resize(cursor, w, h);
        self.adjust(cursor);
    }
}
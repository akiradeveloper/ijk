use crate::edit_buffer::{Cursor, CursorRange, EditBuffer}; 
use crate::BufElem;

pub struct Drawable {
    pub buf: Vec<Vec<Option<BufElem>>>,
    pub cursor: Cursor,
    pub selected: Option<CursorRange>,
}

pub struct VisibilityFilter {
    cur_cursor: Cursor,
    col_low: usize,
    col_high: usize,
    row_low: usize,
    row_high: usize,
}

impl VisibilityFilter {
    pub fn new(cursor: Cursor) -> VisibilityFilter {
        VisibilityFilter {
            cur_cursor: cursor,
            col_low: 0, col_high: 0,
            row_low: 0, row_high: 0,
        }
    }
    pub fn apply(&self, eb: &EditBuffer) -> Drawable {
        let mut buf = vec![vec![None; self.width()]; self.height()];
        for row in self.row_low .. self.row_high+1 {
            if row >= eb.buf.len() {
                continue;
            }
            for col in self.col_low .. self.col_high+1 {
                if col >= eb.buf[row].len() {
                    continue;
                }
                buf[row-self.row_low][col-self.col_low] = Some(eb.buf[row][col].clone());
            }
        }
        Drawable {
            buf: buf,
            cursor: eb.cursor.translate(-(self.row_low as i32), -(self.col_low as i32)),
            selected: eb.visual_range().map(|r| r.translate(-(self.row_low as i32), -(self.col_low as i32))),
        }
    }
    pub fn resize(&mut self, width: usize, height: usize) {
        self.col_low = self.cur_cursor.col;
        self.col_high = self.cur_cursor.col + width - 1;
        self.row_low = self.cur_cursor.row;
        self.row_high = self.cur_cursor.row + height - 1;
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
use crate::BufElem;
use crate::Key;
use crate::visibility_filter::VisibilityFilter;

#[derive(Copy, Clone, PartialOrd, PartialEq)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
}

pub struct ReadBuffer {
    pub buf: Vec<Vec<BufElem>>,
    pub cursor: Cursor,
    num_buffer: Vec<char>,
    pub filter: VisibilityFilter,
}

impl ReadBuffer {
    pub fn new() -> Self {
        Self {
            buf: vec![vec![]],
            cursor: Cursor { row: 0, col: 0 },
            num_buffer: vec![],
            filter: VisibilityFilter::new(Cursor { col: 0, row: 0 }),
        }
    }
    pub fn reset_with(&mut self, new_buf: Vec<Vec<BufElem>>) {
        self.buf = new_buf;
    }
    fn stabilize_cursor(&mut self) {
        if self.cursor.col > self.buf[self.cursor.row].len() - 1 {
            self.cursor.col = self.buf[self.cursor.row].len() - 1;
        }
    }
    pub fn cursor_up(&mut self, _: Key) {
        if self.cursor.row > 0 {
            self.cursor.row -= 1;
        }
        self.stabilize_cursor()
    }
    pub fn cursor_down(&mut self, _: Key) {
        if self.cursor.row < self.buf.len() - 1 {
            self.cursor.row += 1;
        }
        self.stabilize_cursor()
    }
    pub fn cursor_left(&mut self, _: Key) {
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
        }
    }
    pub fn cursor_right(&mut self, _: Key) {
        if self.cursor.col < self.buf[self.cursor.row].len() - 1 {
            self.cursor.col += 1;
        }
    }
    pub fn jump_line_head(&mut self, _: Key) {
        self.cursor.col = 0;
    }
    pub fn jump_line_last(&mut self, _: Key) {
        self.cursor.col = self.buf[self.cursor.row].len() - 1;
    }
    pub fn enter_jump_mode(&mut self, k: Key) {
        self.num_buffer.clear();
        match k {
            Key::Char(c) => self.num_buffer.push(c),
            _ => panic!(),
        }
    }
    pub fn acc_jump_num(&mut self, k: Key) {
        match k {
            Key::Char(c) => self.num_buffer.push(c),
            _ => panic!(),
        }
    }
    pub fn jump(&mut self, _: Key) {
        let mut s = String::new();
        for c in self.num_buffer.clone() {
            s.push(c);
        }
        let n = s.parse::<usize>().unwrap();
        let row = n-1;
        self.cursor.row = row;
        self.cursor.col = 0;
    }
    pub fn cancel_jump(&mut self, _: Key) {
        self.num_buffer.clear();
    }
    pub fn jump_last(&mut self, _: Key) {
        self.cursor.row = self.buf.len() - 1;
        self.cursor.col = 0;
    }
}
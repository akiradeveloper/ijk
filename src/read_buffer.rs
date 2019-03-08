use crate::{BufElem, Cursor, Key};
use crate::visibility_filter::VisibilityFilter;
use crate::search::Search;

pub struct ReadBuffer {
    pub buf: Vec<Vec<BufElem>>,
    pub cursor: Cursor,
    num_buffer: Vec<char>,
    pub filter: VisibilityFilter,
    pub search: Search,
}

impl ReadBuffer {
    pub fn new(init_buf: Vec<Vec<BufElem>>) -> Self {
        let n_rows = init_buf.len();
        Self {
            buf: init_buf,
            cursor: Cursor { row: 0, col: 0 },
            num_buffer: vec![],
            filter: VisibilityFilter::new(Cursor { col: 0, row: 0 }),
            search: Search::new(n_rows),
        }
    }
    fn stabilize_cursor(&mut self) {
        if self.cursor.col > self.buf[self.cursor.row].len() - 1 {
            self.cursor.col = self.buf[self.cursor.row].len() - 1;
        }
    }
    pub fn cursor_up(&mut self) {
        if self.cursor.row > 0 {
            self.cursor.row -= 1;
        }
        self.stabilize_cursor()
    }
    pub fn cursor_down(&mut self) {
        if self.cursor.row < self.buf.len() - 1 {
            self.cursor.row += 1;
        }
        self.stabilize_cursor()
    }
    pub fn cursor_left(&mut self) {
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
        }
    }
    pub fn cursor_right(&mut self) {
        if self.cursor.col < self.buf[self.cursor.row].len() - 1 {
            self.cursor.col += 1;
        }
    }
    pub fn jump_line_head(&mut self) {
        self.cursor.col = 0;
    }
    pub fn jump_line_last(&mut self) {
        self.cursor.col = self.buf[self.cursor.row].len() - 1;
    }
    pub fn jump_page_forward(&mut self) {
        let dist_from_window_bottom = self.filter.row_high - self.cursor.row;
        for _ in 0 .. self.filter.height() + dist_from_window_bottom {
            self.cursor_down();
        }
        self.filter.adjust(self.cursor);
        for _ in 0 .. dist_from_window_bottom {
            self.cursor_up();
        }
    }
    pub fn jump_page_backward(&mut self) {
        let dist_from_window_top = self.cursor.row - self.filter.row_low;
        for _ in 0 .. self.filter.height() + dist_from_window_top {
            self.cursor_up();
        }
        self.filter.adjust(self.cursor);
        for _ in 0 .. dist_from_window_top {
            self.cursor_down();
        }
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
    pub fn jump(&mut self) {
        let mut s = String::new();
        for c in self.num_buffer.clone() {
            s.push(c);
        }
        let n = s.parse::<usize>().unwrap();
        let row = n-1;
        self.cursor.row = row;
        self.cursor.col = 0;
    }
    pub fn cancel_jump(&mut self) {
        self.num_buffer.clear();
    }
    pub fn jump_last(&mut self) {
        self.cursor.row = self.buf.len() - 1;
        self.cursor.col = 0;
    }
    pub fn enter_search_mode(&mut self) {
    }
    pub fn search_mode_input(&mut self, k: Key) {
        match k {
            Key::Backspace => self.search.pop_search_word(),
            Key::Char(c) => self.search.push_search_word(c),
            _ => {}
        }
    }
    pub fn leave_search_mode(&mut self) {

    }
}
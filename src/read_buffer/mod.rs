use crate::{BufElem, Cursor, Key};
use crate::view;
use self::visibility_window::VisibilityWindow;
use self::search::Search;

mod visibility_window;
pub mod search;

pub struct ReadBuffer {
    pub buf: Vec<Vec<BufElem>>,
    pub cursor: Cursor,
    num_buffer: Vec<char>,
    pub window: VisibilityWindow,
    pub search: Search,
}

impl ReadBuffer {
    pub fn new(init_buf: Vec<Vec<BufElem>>) -> Self {
        let n_rows = init_buf.len();
        Self {
            buf: init_buf,
            cursor: Cursor { row: 0, col: 0 },
            num_buffer: vec![],
            window: VisibilityWindow::new(Cursor { col: 0, row: 0 }),
            search: Search::new(n_rows),
        }
    }
    fn stabilize_buffer(&mut self) {
        if self.buf.is_empty() {
            self.buf = vec![vec![BufElem::Eol]];
            self.search = Search::new(1);
        }
    }
    fn stabilize_cursor(&mut self) {
        if self.cursor.row > self.buf.len() - 1 {
            self.cursor.row = self.buf.len() - 1;
        }
        if self.cursor.col > self.buf[self.cursor.row].len() - 1 {
            self.cursor.col = self.buf[self.cursor.row].len() - 1;
        }
    }
    pub fn stabilize(&mut self) {
        self.stabilize_buffer();
        self.stabilize_cursor();
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
        let dist_from_window_bottom = self.window.row_high - self.cursor.row;
        for _ in 0 .. self.window.height() + dist_from_window_bottom {
            self.cursor_down();
        }
        self.window.adjust(self.cursor);
        for _ in 0 .. dist_from_window_bottom {
            self.cursor_up();
        }
    }
    pub fn jump_page_backward(&mut self) {
        let dist_from_window_top = self.cursor.row - self.window.row_low;
        for _ in 0 .. self.window.height() + dist_from_window_top {
            self.cursor_up();
        }
        self.window.adjust(self.cursor);
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
        self.search.clear_search_word()
    }
    pub fn search_mode_input(&mut self, k: Key) {
        match k {
            Key::Backspace => self.search.pop_search_word(),
            Key::Char(c) if ('a' <= c && c <= 'z') || ('A' <= c && c <= 'Z') => self.search.push_search_word(c),
            _ => {}
        }
    }
    pub fn leave_search_mode(&mut self) {}
    pub fn search_jump_forward(&mut self) {
        let next = self.search.next(self.cursor);
        for x in next {
            self.cursor = x;
        }
    }
    pub fn search_jump_backward(&mut self) {
        let prev = self.search.prev(self.cursor);
        for x in prev {
            self.cursor = x;
        }
    }
    pub fn adjust_window(&mut self, w: usize, h: usize) {
        self.window.adjust_window(self.cursor, w, h);
    }
    pub fn current_window(&self) -> view::Area {
        self.window.area()
    }
    pub fn current_search_word(&self) -> String {
        let mut s = String::new();
        for c in &self.search.cur_word {
            s.push(*c);
        }
        s
    }
    pub fn clear_search_struct(&mut self) {
        self.search.clear_struct(self.buf.len())
    }
    pub fn update_search_results(&mut self) {
        self.search.update_results(self.lineno_range(), &self.buf)
    }
    pub fn lineno_range(&self) -> std::ops::Range<usize> {
        self.window.row_low .. std::cmp::min(self.window.row_high+1, self.buf.len())
    }
}
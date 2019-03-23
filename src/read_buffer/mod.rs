use crate::Key;
use crate::view;
use crate::message_box::MessageBox;
use self::visibility_window::VisibilityWindow;
use self::search::Search;
use std::io::Write;

mod visibility_window;
pub mod search;

type Buf = Vec<Vec<BufElem>>;

pub struct Line<'a> {
    line: &'a [BufElem]
}
impl <'a> Line<'a> {
    pub fn new(line: &'a [BufElem]) -> Self {
        Self { line }
    }
    pub fn first_non_space_index(&self) -> usize {
        self.line.iter().position(|c| c != &BufElem::Char(' ') && c != &BufElem::Char('\t')).unwrap()
    }
    pub fn word_range(&self, col: usize) -> Option<std::ops::Range<usize>> {
        fn is_word_char(e: &BufElem) -> bool {
            match e {
                BufElem::Eol => false,
                BufElem::Char(c) => match c {
                    '_' => true,
                    'a' ... 'z' => true,
                    'A' ... 'Z' => true,
                    _ => false
                }
            }
        }
        if !is_word_char(&self.line[col]) {
            return None
        }

        let lower = (0..col).rev().take_while(|&i| is_word_char(&self.line[i])).last().unwrap();
        let higher = (col..self.line.len()).take_while(|&i| is_word_char(&self.line[i])).last().unwrap();
        Some(lower .. higher+1)
    }
}

pub fn write_to_file<W: Write>(mut out: W, buf: &Buf) {
    // TODO trim the eols from the back
    for i in 0..buf.len() {
        for j in 0..buf[i].len() {
            let e = &buf[i][j];
            match *e {
                BufElem::Char(c) => write!(out, "{}", c).unwrap(),
                BufElem::Eol => writeln!(out).unwrap(),
            }
        }
    }
}

fn convert_to_bufelems(cs: Vec<char>) -> Vec<BufElem> {
    let mut r = vec![];
    for c in cs {
        r.push(BufElem::Char(c));
    }
    r.push(BufElem::Eol);
    r
}
pub fn read_from_string(s: Option<String>) -> Buf {
    s.map(|s| {
        if s.is_empty() {
            vec![vec![BufElem::Eol]]
        } else {
            s.lines()
             .map(|line| convert_to_bufelems(line.chars().collect()))
             .collect()
        }
    }).unwrap_or(vec![vec![BufElem::Eol]])
}

#[derive(Clone, PartialEq, Debug)]
pub enum BufElem {
    Char(char),
    Eol,
}

#[derive(Copy, Clone, PartialOrd, PartialEq, Debug)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
}

#[derive(Copy, Clone, Debug)]
pub struct CursorRange {
    pub start: Cursor,
    pub end: Cursor,
}

pub struct ReadBuffer {
    pub buf: Vec<Vec<BufElem>>,
    pub cursor: Cursor,
    num_buffer: Vec<char>,
    pub window: VisibilityWindow,
    pub search: Search,
    message_box: MessageBox,
}

impl ReadBuffer {
    pub fn new(init_buf: Vec<Vec<BufElem>>, message_box: MessageBox) -> Self {
        let n_rows = init_buf.len();
        Self {
            buf: init_buf,
            cursor: Cursor { row: 0, col: 0 },
            num_buffer: vec![],
            window: VisibilityWindow::new(Cursor { col: 0, row: 0 }),
            search: Search::new(n_rows, message_box.clone()),
            message_box,
        }
    }
    pub fn reset(&mut self) {
        self.search.hide_search()
    }
    pub fn stabilize_cursor(&mut self) {
        let mut cursor = self.cursor;

        let max_row = self.buf.len() - 1;
        if cursor.row > max_row {
            cursor.row = max_row;
        }

        if cursor.col > self.buf[cursor.row].len() - 1 {
            cursor.col = self.buf[cursor.row].len() - 1;
        }
        self.cursor = cursor;
    }
    pub fn cursor_up(&mut self) {
        if self.cursor.row > 0 {
            self.cursor.row -= 1;
        }
    }
    pub fn cursor_down(&mut self) {
        if self.cursor.row < self.buf.len() - 1 {
            self.cursor.row += 1;
        }
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
        self.search.clear_search_word();
        self.search.show_search();
    }
    pub fn search_mode_input(&mut self, k: Key) {
        match k {
            Key::Backspace => self.search.pop_search_word(),
            Key::Char(c) if ('a' <= c && c <= 'z') || ('A' <= c && c <= 'Z') => self.search.push_search_word(c),
            _ => {}
        }
    }
    pub fn leave_search_mode(&mut self) {
    }
    pub fn cancel_search_mode(&mut self) {
        self.search.clear_search_word();
        self.search.hide_search();
    }
    pub fn search_jump_forward(&mut self) {
        self.search.show_search();
        let next = self.search.next(self.cursor, &self.buf);
        for x in next {
            self.cursor = x;
        }
    }
    pub fn search_jump_backward(&mut self) {
        self.search.show_search();
        let prev = self.search.prev(self.cursor, &self.buf);
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
    pub fn cache_insert_new_line(&mut self, row: usize) {
        self.search.cache_insert_new_line(row);
    }
    pub fn cache_remove_line(&mut self, row: usize) {
        self.search.cache_remove_line(row);
    }
    pub fn update_cache(&mut self) {
        flame::start("update search");
        self.search.update_cache(self.lineno_range(), &self.buf);
        flame::end("update search");
    }
    pub fn lineno_range(&self) -> std::ops::Range<usize> {
        self.window.row_low .. std::cmp::min(self.window.row_high+1, self.buf.len())
    }
    pub fn line(&self, row: usize) -> Line {
        Line::new(&self.buf[row])
    }
}
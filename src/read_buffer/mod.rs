use crate::Key;
use crate::view;
use crate::message_box::MessageBox;
use self::visibility_window::VisibilityWindow;
use self::search::Search;
use std::io::Write;

mod visibility_window;
pub mod search;

type Buf = Vec<Vec<BufElem>>;

fn is_word_char(e: &BufElem) -> bool {
    match e {
        BufElem::Eol => false,
        BufElem::Char(c) => match c {
            '_' => true,
            '!' => true,
            'a' ... 'z' => true,
            'A' ... 'Z' => true,
            '0' ... '9' => true,
            _ => false
        }
    }
}

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
        if !is_word_char(&self.line[col]) {
            return None
        }

        let lower = (0..col+1).rev().take_while(|&i| is_word_char(&self.line[i])).last().unwrap();
        let higher = (col..self.line.len()).take_while(|&i| is_word_char(&self.line[i])).last().unwrap();
        Some(lower .. higher+1)
    }
    fn find_next_word(&self, col: Option<usize>) -> Option<usize> {
        let start = match col {
            Some(col) => if !is_word_char(&self.line[col]) {
                    col
                } else {
                    self.word_range(col).unwrap().end
                },
            None => 0,
        };
        self.line[start..].iter().position(|e| is_word_char(e)).map(|i| i+start)
    }
    fn find_prev_word(&self, col: Option<usize>) -> Option<usize> {
        let end = match col {
            Some(col) => if !is_word_char(&self.line[col]) {
                    col
                } else {
                    self.word_range(col).unwrap().start
                },
            None => self.line.len()
        };
        let slice = &self.line[0..end];
        slice.iter().rev().position(|e| is_word_char(e)).map(|i| slice.len() - 1 - i)
    }
}

#[test]
fn test_find_word() {
    use self::BufElem::*;
    let v = vec![Char('a'),Char(' '),Char('a'),Char('a'),Char(' '),Char('a'),Eol];
    let line = Line::new(&v);

    assert_eq!(line.find_next_word(None), Some(0));
    assert_eq!(line.find_next_word(Some(0)), Some(2));
    assert_eq!(line.find_next_word(Some(1)), Some(2));
    assert_eq!(line.find_next_word(Some(2)), Some(5));
    assert_eq!(line.find_next_word(Some(3)), Some(5));
    assert_eq!(line.find_next_word(Some(4)), Some(5));
    assert_eq!(line.find_next_word(Some(5)), None);
    assert_eq!(line.find_next_word(Some(6)), None);

    assert_eq!(line.find_prev_word(Some(0)), None);
    assert_eq!(line.find_prev_word(Some(1)), Some(0));
    assert_eq!(line.find_prev_word(Some(2)), Some(0));
    assert_eq!(line.find_prev_word(Some(3)), Some(0));
    assert_eq!(line.find_prev_word(Some(4)), Some(3));
    assert_eq!(line.find_prev_word(Some(5)), Some(3));
    assert_eq!(line.find_prev_word(Some(6)), Some(5));
    assert_eq!(line.find_prev_word(None), Some(5));
}

#[test]
fn test_line_word_range() {
    use self::BufElem::*;
    let v = vec![Char('a'), Char('b'), Char(' '), Char('c'), Eol];
    let line = Line::new(&v);
    assert_eq!(line.word_range(0), Some(0..2));
    assert_eq!(line.word_range(1), Some(0..2));
    assert_eq!(line.word_range(2), None);
    assert_eq!(line.word_range(3), Some(3..4));
    assert_eq!(line.word_range(4), None);
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

const INIT: &str = "Normal";
const SEARCH: &str = "Search";
const JUMP: &str = "Jump";

pub struct ReadBuffer {
    state: String,
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
            state: INIT.to_owned(),
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
            Key::Char(c) => self.search.push_search_word(c),
            _ => {}
        }
    }
    pub fn leave_search_mode(&mut self) {
        self.search_jump_forward()
    }
    pub fn cancel_search_mode(&mut self) {
        self.search.clear_search_word();
        self.search.hide_search();
    }
    fn word_start_cursor(&self, row: usize, col: usize) -> Cursor {
        Cursor {
            row: row,
            col: self.line(row).word_range(col).unwrap().start,
        }
    }
    pub fn jump_word_forward(&mut self) {
        let next_cursor0 = self.line(self.cursor.row).find_next_word(Some(self.cursor.col)).map(|col| self.word_start_cursor(self.cursor.row, col));
        let nc0 = next_cursor0.or({
            let mut range = vec![];
            for i in self.cursor.row+1 .. self.buf.len() { range.push(i) }
            for i in 0..self.cursor.row+1 { range.push(i) }
            range.into_iter().map(|row|
                self.line(row).find_next_word(None).map(|col| self.word_start_cursor(row, col))
            ).find(|x| x.is_some()).unwrap_or(None)
        });
        for nc in nc0 {
            self.cursor = nc;
        }
    }
    pub fn jump_word_backward(&mut self) {
        let next_cursor0 = self.line(self.cursor.row).find_prev_word(Some(self.cursor.col)).map(|col| self.word_start_cursor(self.cursor.row, col));
        let nc0 = next_cursor0.or({
            let mut range = vec![];
            for i in (0..self.cursor.row).rev() { range.push(i) }
            for i in (self.cursor.row..self.buf.len()).rev() { range.push(i) }
            range.into_iter().map(|row|
                self.line(row).find_prev_word(None).map(|col| self.word_start_cursor(row, col))
            ).find(|x| x.is_some()).unwrap_or(None)
        });
        for nc in nc0 {
            self.cursor = nc;
        }
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

    //
    // eff functions
    //

    pub fn eff_cursor_up(&mut self, _: Key) -> String {
        self.cursor_up();
        INIT.to_owned()
    }
    pub fn eff_cursor_down(&mut self, _: Key) -> String {
        self.cursor_down();
        INIT.to_owned()
    }
    pub fn eff_cursor_left(&mut self, _: Key) -> String {
        self.cursor_left();
        INIT.to_owned()
    }
    pub fn eff_cursor_right(&mut self, _: Key) -> String {
        self.cursor_right();
        INIT.to_owned()
    }
    pub fn eff_jump_line_head(&mut self, _: Key) -> String {
        self.jump_line_head();
        INIT.to_owned()
    }
    pub fn eff_jump_line_last(&mut self, _: Key) -> String {
        self.jump_line_last();
        INIT.to_owned()
    }
    pub fn eff_jump_page_forward(&mut self, _: Key) -> String {
        self.jump_page_forward();
        INIT.to_owned()
    }
    pub fn eff_jump_page_backward(&mut self, _: Key) -> String {
        self.jump_page_backward();
        INIT.to_owned()
    }
    fn eff_jump_word_forward(&mut self, _: Key) -> String {
        self.jump_word_forward();
        INIT.to_owned()
    }
    fn eff_jump_word_backward(&mut self, _: Key) -> String {
        self.jump_word_backward();
        INIT.to_owned()
    }
    pub fn eff_enter_jump_mode(&mut self, k: Key) -> String {
        self.enter_jump_mode(k);
        JUMP.to_owned()
    }
    pub fn eff_acc_jump_num(&mut self, k: Key) -> String {
        self.acc_jump_num(k);
        JUMP.to_owned()
    }
    pub fn eff_jump(&mut self, _: Key) -> String {
        self.jump();
        INIT.to_owned()
    }
    pub fn eff_cancel_jump(&mut self, _: Key) -> String {
        self.cancel_jump();
        INIT.to_owned()
    }
    pub fn eff_jump_last(&mut self, _: Key) -> String {
        self.jump_last();
        INIT.to_owned()
    }
    pub fn eff_enter_search_mode(&mut self, _: Key) -> String {
        self.enter_search_mode();
        SEARCH.to_owned()
    }
    pub fn eff_search_mode_input(&mut self, k: Key) -> String {
        self.search_mode_input(k);
        SEARCH.to_owned()
    }
    pub fn eff_leave_search_mode(&mut self, _: Key) -> String {
        self.leave_search_mode();
        INIT.to_owned()
    }
    fn eff_cancel_search_mode(&mut self, _: Key) -> String {
        self.cancel_search_mode();
        INIT.to_owned()
    }
    pub fn eff_search_jump_forward(&mut self, _: Key) -> String {
        self.search_jump_forward();
        INIT.to_owned()
    }
    pub fn eff_search_jump_backward(&mut self, _: Key) -> String {
        self.search_jump_backward();
        INIT.to_owned()
    }
}

use crate::controller::Effect;
use crate::def_effect;

def_effect!(CursorUp, ReadBuffer, eff_cursor_up);
def_effect!(CursorDown, ReadBuffer, eff_cursor_down);
def_effect!(CursorLeft, ReadBuffer, eff_cursor_left);
def_effect!(CursorRight, ReadBuffer, eff_cursor_right);
def_effect!(JumpLineHead, ReadBuffer, eff_jump_line_head);
def_effect!(JumpLineLast, ReadBuffer, eff_jump_line_last);
def_effect!(JumpPageForward, ReadBuffer, eff_jump_page_forward);
def_effect!(JumpPageBackward, ReadBuffer, eff_jump_page_backward);
def_effect!(EnterJumpMode, ReadBuffer, eff_enter_jump_mode);
def_effect!(AccJumpNum, ReadBuffer, eff_acc_jump_num);
def_effect!(Jump, ReadBuffer, eff_jump);
def_effect!(CancelJump, ReadBuffer, eff_cancel_jump);
def_effect!(JumpLast, ReadBuffer, eff_jump_last);
def_effect!(JumpWordForward, ReadBuffer, eff_jump_word_forward);
def_effect!(JumpWordBackward, ReadBuffer, eff_jump_word_backward);

def_effect!(EnterSearchMode, ReadBuffer, eff_enter_search_mode);
def_effect!(SearchModeInput, ReadBuffer, eff_search_mode_input);
def_effect!(LeaveSearchMode, ReadBuffer, eff_leave_search_mode);
def_effect!(CancelSearchMode, ReadBuffer, eff_cancel_search_mode);
def_effect!(SearchJumpForward, ReadBuffer, eff_search_jump_forward);
def_effect!(SearchJumpBackward, ReadBuffer, eff_search_jump_backward);

pub fn add_edges<S: crate::shared::AsRefMut<ReadBuffer> + 'static>(g: &mut crate::controller::Graph, x: S) {
    use std::rc::Rc;
    use crate::Key::*;

    // normal movement
    g.add_edge(INIT, Char('k'), Rc::new(CursorUp(x.clone())));
    g.add_edge(INIT, Char('j'), Rc::new(CursorDown(x.clone())));
    g.add_edge(INIT, Char('h'), Rc::new(CursorLeft(x.clone())));
    g.add_edge(INIT, Char('l'), Rc::new(CursorRight(x.clone())));
    g.add_edge(INIT, Char('0'), Rc::new(JumpLineHead(x.clone())));
    g.add_edge(INIT, Char('$'), Rc::new(JumpLineLast(x.clone())));
    g.add_edge(INIT, Ctrl('f'), Rc::new(JumpPageForward(x.clone())));
    g.add_edge(INIT, Ctrl('b'), Rc::new(JumpPageBackward(x.clone())));
    g.add_edge(INIT, Char('G'), Rc::new(JumpLast(x.clone())));
    g.add_edge(INIT, Char('n'), Rc::new(SearchJumpForward(x.clone())));
    g.add_edge(INIT, Char('N'), Rc::new(SearchJumpBackward(x.clone())));
    g.add_edge(INIT, Char('w'), Rc::new(JumpWordForward(x.clone())));
    g.add_edge(INIT, Char('b'), Rc::new(JumpWordBackward(x.clone())));

    // num jump
    g.add_edge(INIT, CharRange('1', '9'), Rc::new(EnterJumpMode(x.clone())));
    g.add_edge(JUMP, CharRange('0', '9'), Rc::new(AccJumpNum(x.clone())));
    g.add_edge(JUMP, Char('G'), Rc::new(Jump(x.clone())));
    g.add_edge(JUMP, Esc, Rc::new(CancelJump(x.clone())));

    // search
    g.add_edge(INIT, Char('/'), Rc::new(EnterSearchMode(x.clone())));
    g.add_edge(SEARCH, Char('\n'), Rc::new(LeaveSearchMode(x.clone())));
    g.add_edge(SEARCH, Esc, Rc::new(CancelSearchMode(x.clone())));
    g.add_edge(SEARCH, Otherwise, Rc::new(SearchModeInput(x.clone())))
}
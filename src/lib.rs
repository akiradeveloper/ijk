#[derive(Debug, PartialEq, Clone)]
pub enum Key {
    Left,
    Right,
    Up,
    Down,
    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    Backspace,
    Esc,
    F(u8),
    Char(char), // termion passes space as Char(' ') and tab as Char('\t')
    Alt(char),
    Ctrl(char),

    CharRange(char,char), // only for matcher. inclusive like ...
    Otherwise,
}

#[derive(Clone, PartialEq, Debug)]
pub enum BufElem {
    Char(char),
    Eol,
}

#[derive(Copy, Clone, PartialOrd, PartialEq)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
}

#[derive(Clone)]
pub struct ChangeLog {
    at: Cursor,
    deleted: Vec<BufElem>,
    inserted: Vec<BufElem>,
}
impl ChangeLog {
    pub fn swap(&self) -> Self {
        Self {
            at: self.at,
            deleted: self.inserted.clone(),
            inserted: self.deleted.clone(),
        }
    }
}

pub mod editor;
pub mod read_buffer;
pub mod diff_buffer;
pub mod edit_buffer;
pub mod undo_buffer;
pub mod visibility_filter;
pub mod screen;
pub mod controller;
pub mod view;
pub mod search;
pub mod indent;
pub mod navigator;
pub mod directory;
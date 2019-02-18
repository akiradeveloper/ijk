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
}

#[derive(Clone, PartialEq)]
enum BufElem {
    Char(char),
    Eol,
    Indent
}

pub mod automaton;
pub mod diff_buffer;
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

#[derive(Clone, PartialEq, Debug)]
pub enum BufElem {
    Char(char),
    Eol,
}

pub mod automaton;
pub mod diff_buffer;
pub mod edit_buffer;
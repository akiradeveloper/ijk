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

pub mod editor;
pub mod read_buffer;
pub mod edit_buffer;
pub mod screen;
pub mod controller;
pub mod view;
pub mod navigator;
pub mod directory;
pub mod message_box;
pub mod util;
pub mod theme;
pub mod shared;
mod config;

extern crate flame;
#[macro_use]
extern crate serde_derive;
extern crate toml;
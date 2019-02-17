extern crate termion;

use std::io::{stdin, stdout, Write};
use termion::clear;
use termion::cursor;
use termion::event::{Event, Key};
use termion::input::TermRead;
use termion::raw::IntoRawMode;

fn main() {
    let stdin = stdin();

    let mut stdout = stdout().into_raw_mode().unwrap();

    for evt in stdin.events() {
        println!("{:?}", evt);
        if evt.unwrap() == Event::Key(Key::Ctrl('c')) {
            return;
        }
    }
}
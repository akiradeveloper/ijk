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
    // 画面全体をクリアする
    write!(stdout, "{}", clear::All);
    // カーソルを左上に設定する(1-indexed)
    write!(stdout, "{}", cursor::Goto(1, 1));
    // Hello World!
    write!(stdout, "Hello World!\r\naaaa");

    write!(stdout, "\r\n\n\n");
    // Hello World!
    write!(stdout, "Hello World!\naaaa");
    // 最後にフラッシュする
    stdout.flush().unwrap();

    for evt in stdin.events() {
        println!("{:?}", evt);
        if evt.unwrap() == Event::Key(Key::Ctrl('c')) {
            return;
        }
    }
}
extern crate termion;

use std::io::{stdin, stdout, Write};
use termion::clear;
use termion::cursor;
use termion::event::Event;
use termion::event::Key as TermKey;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use std::ffi::OsStr;
use std::path;
use std::fs;
use clap::{App, Arg};

use ijk::BufElem;
use ijk::Key::*;
use ijk::edit_buffer as EB;

fn convert_to_bufelems(cs: Vec<char>) -> Vec<BufElem> {
    let mut r = vec![];
    for c in cs {
        r.push(BufElem::Char(c));
    }
    r.push(BufElem::Eol);
    r
}

fn main() {
    let stdin = stdin();
    let mut stdout = AlternateScreen::from(stdout().into_raw_mode().unwrap());

    let matches = App::new("ijk")
        .about("A toy editor for fun")
        .bin_name("ijk")
        .arg(Arg::with_name("file"))
        .get_matches();

    let file_path: Option<&OsStr> = matches.value_of_os("file");
    let read_buf: Vec<Vec<BufElem>> = file_path
        .and_then(|file_path| {
            // エラー処理は適当
            fs::read_to_string(path::Path::new(file_path))
                .ok()
                .map(|s| {
                    s.lines()
                     .map(|line| convert_to_bufelems(line.chars().collect()))
                     .collect()
                })
        })
        .unwrap_or(vec![vec![BufElem::Eol]]);

    let mut eb = EB::EditBuffer::new();
    eb.reset_with(read_buf);
    let mut kr = EB::KeyReceiver::new();

    for c in stdin.keys() {
        // draw
        write!(stdout, "{}", clear::All);
        for row in 0 .. eb.buf.len() {
            let line = &eb.buf[row];
            write!(stdout, "{}", cursor::Goto(1,(row+1) as u16));
            for col in 0 .. line.len() {
                let e = eb.buf[row][col].clone();
                match e {
                    BufElem::Char(c) => { write!(stdout, "{}", c); },
                    BufElem::Eol => {}
                }
            }
        }
        stdout.flush().unwrap();

        // conversion
        let k = match c.unwrap() {
            TermKey::Ctrl('c') => return,
            TermKey::Ctrl(c) => Ctrl(c),
            TermKey::Char(c) => Char(c),
            _ => return,
        };
        let act = kr.receive(k);
        eb.receive(act);
    }
}
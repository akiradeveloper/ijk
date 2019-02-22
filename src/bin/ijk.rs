extern crate termion;

use std::io::{stdin, stdout, Write};
use termion::clear;
use termion::cursor;
use termion::color;
use termion::event::Event;
use termion::event::Key as TermKey;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use std::ffi::OsStr;
use std::path;
use std::fs;
use std::{thread, time};
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
    // let stdin = stdin();
    let stdin = termion::async_stdin();

    let stdout = stdout();
    let mut stdout = AlternateScreen::from(stdout.into_raw_mode().unwrap());
    // let mut stdout = stdout().into_raw_mode().unwrap();

    let matches = App::new("ijk")
        .about("A toy editor for fun")
        .bin_name("ijk")
        .arg(Arg::with_name("file"))
        .get_matches();

    let file_path: Option<&OsStr> = matches.value_of_os("file");
    let read_buf: Vec<Vec<BufElem>> = file_path
        .and_then(|file_path| {
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
    let mut vfilter = ijk::visibility_filter::VisibilityFilter::new(eb.cursor);
    let (term_w, term_h) = termion::terminal_size().unwrap();
    vfilter.resize(term_w as usize, term_h as usize);

    let mut keys = stdin.keys();

    loop {
        vfilter.adjust(eb.cursor);
        let drawable = vfilter.apply(&eb);

        let vr0 = eb.visual_range();
        write!(stdout, "{}", clear::All);
        for row in 0 .. eb.buf.len() {
            let line = &eb.buf[row];
            write!(stdout, "{}", cursor::Goto(1,(row+1) as u16));
            for col in 0 .. line.len() {
                let e = eb.buf[row][col].clone();
                let as_cursor = EB::Cursor { row: row, col: col };
                let in_visual_range = vr0.clone().map(|vr| vr.start <= as_cursor && as_cursor < vr.end).unwrap_or(false);
                let c = match e {
                    BufElem::Char(c) => c,
                    BufElem::Eol => ' '
                };
                if in_visual_range {
                    write!(stdout, "{}{}", color::Bg(color::Blue), c);
                } else {
                    write!(stdout, "{}{}", color::Bg(color::Reset), c);
                }
            }
        }
        write!(stdout, "{}", cursor::Goto((eb.cursor.col+1) as u16, (eb.cursor.row+1) as u16));
        stdout.flush().unwrap(); 

        let k = match keys.next() {
            Some(Ok(TermKey::Ctrl('z'))) => break,
            Some(Ok(TermKey::Ctrl('c'))) => Esc,
            Some(Ok(TermKey::Backspace)) => Backspace,
            Some(Ok(TermKey::Ctrl(c))) => Ctrl(c),
            Some(Ok(TermKey::Char(c))) => Char(c),
            // None, Some(Err), Some(Unknown)
            _ => {
                thread::sleep(time::Duration::from_millis(100));
                continue
            },
        };

        let act = kr.receive(k);
        eb.receive(act);
    }
}
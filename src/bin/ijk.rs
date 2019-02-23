extern crate termion;

use std::io::{stdin, stdout, BufWriter, Write};
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
use ijk::screen::*;

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

    let (term_w, term_h) = termion::terminal_size().unwrap();
    // let (term_w, term_h) = (15, 15);
    let mut screen = Screen::new(term_w as usize, term_h as usize);

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
    let window_col: u16 = 0; let window_row: u16 = 0;
    // let window_col: u16 = 5; let window_row: u16 = 5;

    vfilter.resize(term_w as usize, term_h as usize);

    let mut keys = stdin.keys();

    loop {
        vfilter.adjust(eb.cursor);
        let drawable = vfilter.apply(&eb);

        screen.clear();
        for row in 0 .. drawable.buf.len() {
            let line = &drawable.buf[row];
            for col in 0 .. line.len() {
                let e = drawable.buf[row][col].clone();
                let as_cursor = EB::Cursor { row: row, col: col };
                let in_visual_range = drawable.selected.map(|vr| vr.start <= as_cursor && as_cursor < vr.end).unwrap_or(false);
                let c0 = match e {
                    Some(BufElem::Char(c)) => Some(c),
                    Some(BufElem::Eol) => Some(' '),
                    None => None
                };
                let fg = Color::White;
                let bg = if in_visual_range {
                    Color::Blue
                } else {
                    Color::Black
                };
                for c in c0 {
                    screen.draw(col, row, c, Style(fg, bg));
                }
            }
        }
        screen.move_cursor(drawable.cursor.col, drawable.cursor.row);
        screen.present();

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
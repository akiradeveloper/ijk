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
use std::cell::RefCell;
use std::rc::Rc;

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

    let mut eb = Rc::new(RefCell::new(EB::EditBuffer::new()));
    eb.borrow_mut().reset_with(read_buf);
    let mut ctrl = Rc::new(RefCell::new(EB::Controller::new(eb.clone())));
    let mut editor = ijk::editor::Editor::new(ctrl, eb);

    editor.run();
}
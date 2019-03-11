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
use ijk::edit_buffer;
use ijk::screen::*;
use ijk::navigator;

fn main() {
    let matches = App::new("ijk")
        .about("A toy editor for fun")
        .bin_name("ijk")
        .arg(Arg::with_name("file"))
        .get_matches();

    let file_path: Option<&OsStr> = matches.value_of_os("file");
    let path = file_path.map(|fp| path::Path::new(fp));
    
    let mut navigator = Rc::new(RefCell::new(navigator::Navigator::new()));
    let mut eb = Rc::new(RefCell::new(edit_buffer::EditBuffer::open(path)));
    let page = Box::new(edit_buffer::Page::new(eb));
    navigator.borrow_mut().push(page);
    let mut editor = ijk::editor::Editor::new(navigator);

    editor.run();
}
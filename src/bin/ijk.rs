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

fn main() {
    let matches = App::new("ijk")
        .about("A toy editor for fun")
        .bin_name("ijk")
        .arg(Arg::with_name("file"))
        .get_matches();

    let file_path: Option<&OsStr> = matches.value_of_os("file");
    let path = file_path.map(|fp| path::Path::new(fp));

    let mut eb = Rc::new(RefCell::new(EB::EditBuffer::open(path)));

    let mut ctrl = Rc::new(RefCell::new(EB::mk_controller(eb.clone())));
    let mut view_gen = Rc::new(RefCell::new(EB::ViewGen::new(eb.clone())));
    let mut editor = ijk::editor::Editor::new(ctrl, view_gen);

    editor.run();
}
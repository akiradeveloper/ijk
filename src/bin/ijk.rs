extern crate termion;

use std::ffi::OsStr;
use std::path;
use std::fs;
use clap::{App, Arg};
use std::cell::RefCell;
use std::rc::Rc;
use termion::input::TermRead;

use ijk::directory;
use ijk::edit_buffer;
use ijk::navigator;

fn main() {
    let matches = App::new("ijk")
        .about("A real editor for read programmers")
        .bin_name("ijk")
        .arg(Arg::with_name("path"))
        .get_matches();

    let file_path: Option<&OsStr> = matches.value_of_os("path");
    let path = file_path.map(|fp| path::Path::new(fp));
    
    let navigator = Rc::new(RefCell::new(navigator::Navigator::new()));

    let page: Rc<navigator::Page> = match path {
        Some(path) if path.is_dir() => {
            let dir = Rc::new(RefCell::new(directory::Directory::open(path, navigator.clone())));
            Rc::new(directory::Page::new(dir, fs::canonicalize(path).unwrap()))
        },
        Some(path) => { // existing/unexisting file
            let eb = Rc::new(RefCell::new(edit_buffer::EditBuffer::open(Some(path))));
            Rc::new(edit_buffer::Page::new(eb))
        },
        None => {
            let eb = Rc::new(RefCell::new(edit_buffer::EditBuffer::open(None)));
            Rc::new(edit_buffer::Page::new(eb))
        }
    };

    navigator.borrow_mut().push(page);
    let mut editor = ijk::editor::Editor::new(navigator);

    let stdin = std::io::stdin();
    // let stdin = termion::async_stdin();
    let keys = stdin.keys();
    editor.run(keys);
}
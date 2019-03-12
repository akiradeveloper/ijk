extern crate termion;

use std::ffi::OsStr;
use std::path;
use std::fs;
use clap::{App, Arg};
use std::cell::RefCell;
use std::rc::Rc;

use ijk::directory;
use ijk::edit_buffer;
use ijk::navigator;

fn main() {
    let matches = App::new("ijk")
        .about("A toy editor for fun")
        .bin_name("ijk")
        .arg(Arg::with_name("path"))
        .get_matches();

    let file_path: Option<&OsStr> = matches.value_of_os("path");
    let path = file_path.map(|fp| path::Path::new(fp));
    
    let navigator = Rc::new(RefCell::new(navigator::Navigator::new()));

    let page: Box<navigator::Page> = match path {
        Some(path) if path.is_dir() => {
            let dir = Rc::new(RefCell::new(directory::Directory::open(path, navigator.clone())));
            Box::new(directory::Page::new(dir, fs::canonicalize(path).unwrap()))
        },
        Some(path) => { // existing/unexisting file
            let eb = Rc::new(RefCell::new(edit_buffer::EditBuffer::open(Some(path))));
            Box::new(edit_buffer::Page::new(eb))
        },
        None => {
            let eb = Rc::new(RefCell::new(edit_buffer::EditBuffer::open(None)));
            Box::new(edit_buffer::Page::new(eb))
        }
    };

    navigator.borrow_mut().push(page);
    let mut editor = ijk::editor::Editor::new(navigator);

    editor.run();
}
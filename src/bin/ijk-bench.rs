extern crate termion;
extern crate flame;

use std::ffi::OsStr;
use std::path;
use clap::{App, Arg};
use std::cell::RefCell;
use std::rc::Rc;

use ijk::edit_buffer;
use ijk::navigator;
use ijk::util;

fn main() {
    let matches = App::new("ijk-bench")
        .about("benchmark ijk editor")
        .bin_name("ijk-bench")
        .arg(Arg::with_name("file"))
        .arg(Arg::with_name("keys"))
        .get_matches();

    let file: Option<&OsStr> = matches.value_of_os("file");
    let file = file.map(|fp| path::Path::new(fp)).unwrap();
    assert!(file.is_file());

    let keys: Option<&OsStr> = matches.value_of_os("keys");
    let keys = keys.map(|fp| path::Path::new(fp)).unwrap();
    assert!(keys.is_file());
    let keys = util::read_keys_file(keys).into_iter();
    
    let navigator = Rc::new(RefCell::new(navigator::Navigator::new()));

    let eb = Rc::new(RefCell::new(edit_buffer::EditBuffer::open(Some(file))));
    let page = Rc::new(edit_buffer::Page::new(eb));

    navigator.borrow_mut().push(page);
    let (term_w, term_h) = termion::terminal_size().unwrap();
    let mut editor = ijk::editor::Editor::new(navigator, ijk::editor::TerminalScreen::new(term_w as usize, term_h as usize));

    flame::start("run");
    editor.run(keys);
    flame::end("run");

    flame::dump_html(&mut std::fs::File::create("output.html").unwrap()).unwrap();
}
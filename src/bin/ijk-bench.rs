extern crate termion;
extern crate flame;

use std::ffi::OsStr;
use std::path;
use std::fs;
use clap::{App, Arg};
use std::cell::RefCell;
use std::rc::Rc;
use termion::event::Key::*;

use ijk::edit_buffer;
use ijk::navigator;

fn to_term_key(s: &str) -> termion::event::Key {
    match s {
        "EOL" => Char('\n'),
        c => Char(c.chars().nth(0).unwrap()),
        _ => panic!(), // other keys are not necessary in benchmark
    }
}
fn read_buffer(path: &path::Path) -> Vec<Result<termion::event::Key, std::io::Error>> {
    let mut v = vec![];
    let s = fs::read_to_string(path).unwrap();
    for line in s.lines() {
        v.push(Ok(to_term_key(line)))
    }
    v.push(Ok(Ctrl('z')));
    v
}
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
    let keys = read_buffer(keys).into_iter();
    
    let navigator = Rc::new(RefCell::new(navigator::Navigator::new()));

    let eb = Rc::new(RefCell::new(edit_buffer::EditBuffer::open(Some(file))));
    let page = Rc::new(edit_buffer::Page::new(eb));

    navigator.borrow_mut().push(page);
    let mut editor = ijk::editor::Editor::new(navigator);

    flame::start("run");
    editor.run(keys);
    flame::end("run");

    flame::dump_html(&mut std::fs::File::create("output.html").unwrap()).unwrap();
}
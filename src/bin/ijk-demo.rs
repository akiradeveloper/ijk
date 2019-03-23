extern crate termion;
extern crate flame;

use clap::App;
use std::cell::RefCell;
use std::rc::Rc;
use std::fs;
use termion::event::Key::*;

use ijk::directory;
use ijk::navigator;
use ijk::util;

fn main() {
    let matches = App::new("ijk-demo")
        .about("ijk editor demo")
        .bin_name("ijk-demo")
        .get_matches();

    let keys = vec![
        Char('j'),
        Char('l'),
        Ctrl('w'),
        Char('\n'),
        Char('h'),
    ].into_iter().map(|x| Ok(x));
    let keys = util::IntervalIterator::new(keys, 1000);
    
    let navigator = Rc::new(RefCell::new(navigator::Navigator::new()));

    let cur_dir = std::env::current_dir().unwrap();
    let dir = Rc::new(RefCell::new(directory::Directory::open(&cur_dir, navigator.clone())));
    let page = Rc::new(directory::Page::new(dir, fs::canonicalize(cur_dir).unwrap()));

    navigator.borrow_mut().push(page);
    let mut editor = ijk::editor::Editor::new(navigator, ijk::editor::TerminalScreen::new());

    flame::start("run");
    editor.run(keys);
    flame::end("run");
}
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
        Char('j'), Char('l'),
        Ctrl('w'),
        Char('\n'),
        Char('h'),
        // hide/show dot files
        Char('.'), Char('.'),
        // open a.rs
        Char('j'), Char('j'), Char('\n'),
        Char('/'), Char('x'), Char('\n'),
        Char('n'), Char('N'), Esc, Char('N'), Esc,
        Char('w'), Char('w'), Char('w'), Char('b'), Char('b'),
        // cursor is on number and then change
        Char('l'), Char('l'), Char('c'), Char('w'), Char('5'), Char('6'), Char('7'), Esc,
        // undo/redo
        Char('u'), Ctrl('r'), Char('u'), Ctrl('r'),
        // jump
        Char('1'), Char('G'), Char('G'), Char('2'), Char('G'),
        // move to the latest dir
        Ctrl('w'), Char('h'),
        Ctrl('w'), Char('l'),
        Ctrl('w'), Char('h'),
        Char('j'), Char('j'), Char('j'), Char('\n'),
        Ctrl('w'),
    ].into_iter().map(|x| Ok(x));
    let keys = util::IntervalIterator::new(keys, 400);
    
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
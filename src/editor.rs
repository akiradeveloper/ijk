use termion::event::Key as TermKey;
use crate::Key as Key;
use termion::input::TermRead;
use std::cell::RefCell;
use std::rc::Rc;
use std::{thread, time};
use crate::screen::*;
use crate::view;
use crate::navigator::{self, Page};
use crate::controller;

pub struct Editor {
    navigator: Rc<RefCell<navigator::Navigator>>,
    navi_page: Rc<Page>,
}

impl Editor {
    pub fn new(navigator: Rc<RefCell<navigator::Navigator>>) -> Self {
        Self {
            navi_page: Rc::new(navigator::NavigatorPage::new(navigator.clone())),
            navigator: navigator,
        }
    }
    // fn draw() {}
    pub fn run(&mut self) {
        let stdin = termion::async_stdin();

        let (term_w, term_h) = termion::terminal_size().unwrap();
        let mut screen = Screen::new(term_w as usize, term_h as usize);

        let mut keys = stdin.keys();

        loop {
            let region = view::Area {
                col: 0,
                row: 0,
                width: term_w as usize,
                height: term_h as usize,
            };
            let page = self.navigator.borrow().current.clone();
            let view = page.view_gen().gen(region);
            screen.clear();
            for row in 0 .. region.height {
                for col in 0 .. region.width {
                    let (c,fg,bg) = view.get(col,row);
                    screen.draw(col, row, c, Style(fg,bg))
                }
            }
            for cursor in view.get_cursor_pos() {
                screen.move_cursor(cursor.col, cursor.row);
            }
            screen.present();

            match keys.next() {
                Some(Ok(TermKey::Ctrl('z'))) => break,
                Some(Ok(TermKey::Ctrl('w'))) => {
                    self.navigator.borrow_mut().set(self.navi_page.clone());
                },
                other_key => {
                    let kk = match other_key {
                        Some(Ok(TermKey::Ctrl('c'))) => Key::Esc,
                        Some(Ok(TermKey::Backspace)) => Key::Backspace,
                        Some(Ok(TermKey::Ctrl(c))) => Key::Ctrl(c),
                        Some(Ok(TermKey::Char(c))) => Key::Char(c),
                        _ => {
                            thread::sleep(time::Duration::from_millis(100));
                            continue
                        },
                    };
                    let page = self.navigator.borrow().current.clone();
                    page.controller().receive(kk);
                }
            }
        }
    }
}
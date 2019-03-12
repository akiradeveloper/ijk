use termion::event::Key as TermKey;
use crate::Key as Key;
use termion::input::TermRead;
use std::cell::RefCell;
use std::rc::Rc;
use std::{thread, time};
use crate::screen::*;
use crate::view;
use crate::navigator;
use crate::controller;

pub struct Editor {
    navigator: Rc<RefCell<navigator::Navigator>>,
    controller: Rc<RefCell<controller::Controller>>,
    view_gen: Rc<RefCell<view::ViewGen>>,
}

impl Editor {
    pub fn new(navigator: Rc<RefCell<navigator::Navigator>>) -> Self {
        Self {
            controller: Rc::new(RefCell::new(navigator::mk_controller(navigator.clone()))),
            view_gen: Rc::new(RefCell::new(navigator::ViewGen::new(navigator.clone()))),
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
            let view_gen = self.navigator.borrow().view_gen.clone();
            let view = view_gen.borrow_mut().gen(region);
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
                    self.navigator.borrow_mut().set(self.controller.clone(), self.view_gen.clone());
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
                    let controller = self.navigator.borrow().controller.clone();
                    controller.borrow_mut().receive(kk);
                }
            }
        }
    }
}
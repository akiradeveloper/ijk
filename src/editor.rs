use termion::event::Key as TermKey;
use crate::Key as Key;
use crate::controller::KeyController;
use termion::clear;
use termion::cursor;
use termion::color;
use termion::event::Event;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use std::cell::RefCell;
use std::rc::Rc;
use std::{thread, time};
use crate::Key::*;
use crate::edit_buffer as EB;
use crate::screen::*;
use crate::BufElem;
use crate::view;
use crate::view::View;
use crate::view::ViewGen;

pub struct Editor {
    cur_ctrl: Rc<RefCell<EB::Controller>>,
    view_gen: Rc<RefCell<EB::ViewGen>>, // tmp instead of view
}

impl Editor {
    pub fn new(ctrl: Rc<RefCell<EB::Controller>>, view_gen: Rc<RefCell<EB::ViewGen>>) -> Self {
        Self {
            cur_ctrl: ctrl,
            view_gen: view_gen,
        }
    }
    // fn draw() {}
    pub fn run(&mut self) {
        // let stdin = stdin();
        let stdin = termion::async_stdin();

        let (term_w, term_h) = termion::terminal_size().unwrap();
        // let (term_w, term_h) = (15, 15);
        let mut screen = Screen::new(term_w as usize, term_h as usize);
        let window_col: u16 = 0; let window_row: u16 = 0;
        // let window_col: u16 = 5; let window_row: u16 = 5;

        let mut keys = stdin.keys();

        loop {
            let region = view::ViewRegion {
                col: 0,
                row: 0,
                width: term_w as usize,
                height: term_h as usize,
            };
            let view = self.view_gen.borrow_mut().gen(&region);
            screen.clear();
            for row in 0 .. region.width {
                for col in 0 .. region.height {
                    let (c,fg,bg) = view.get(col,row);
                    screen.draw(col, row, c, Style(fg,bg))
                }
            }
            let cursor = view.get_cursor_pos().unwrap();
            screen.move_cursor(cursor.col, cursor.row);
            screen.present();

            match keys.next() {
                Some(Ok(TermKey::Ctrl('z'))) => break,
                // Some(Ok(TermKey::Ctrl('b'))) => self.switch_to_current_buffer(),
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
                    self.cur_ctrl.borrow_mut().receive(kk);
                }
            }
        }
    }
}
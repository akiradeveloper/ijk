use crate::navigator::{self, Page};
use crate::screen::*;
use crate::view;
use crate::Key;
use std::cell::RefCell;
use std::rc::Rc;
use std::{thread, time};
use termion::event::Key as TermKey;
use termion::input::TermRead;
use crate::message_box;
use crate::view::View;

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
            let area = view::Area {
                col: 0,
                row: 0,
                width: term_w as usize,
                height: term_h as usize,
            };
            let (page_area, message_area) = area.split_vertical(area.height - 1);
            let page = self.navigator.borrow().current.clone();
            let page_view = page.view_gen().gen(page_area);

            let message_view = message_box::View::new(message_box::SINGLETON.clone());
            let view = view::MergeVertical {
                top: page_view,
                bottom: message_view,
                row_offset: message_area.row,
            };

            screen.clear();
            for row in 0..area.height {
                for col in 0..area.width {
                    let (c, fg, bg) = view.get(col, row);
                    screen.draw(col, row, c, Style(fg, bg))
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
                }
                other_key => {
                    let kk = match other_key {
                        Some(Ok(TermKey::Ctrl('c'))) => Key::Esc,
                        Some(Ok(TermKey::Backspace)) => Key::Backspace,
                        Some(Ok(TermKey::Ctrl(c))) => Key::Ctrl(c),
                        Some(Ok(TermKey::Char(c))) => Key::Char(c),
                        _ => {
                            thread::sleep(time::Duration::from_millis(100));
                            continue;
                        }
                    };
                    let page = self.navigator.borrow().current.clone();
                    page.controller().receive(kk);
                }
            }
        }
    }
}

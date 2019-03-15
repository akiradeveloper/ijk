use crate::navigator::{self, Page};
use crate::screen::*;
use crate::view;
use crate::{Key, Cursor, BufElem};
use std::cell::RefCell;
use std::rc::Rc;
use std::{thread, time};
use termion::event::Key as TermKey;
use termion::input::TermRead;
use crate::message_box;
use crate::view::View;

struct StatusView {
    x: Vec<char>,
}
impl StatusView {
    fn new(x: &str) -> Self {
        let mut v = vec![];
        for c in x.chars() {
            v.push(c);
        }
        Self {
            x: v
        }
    }
}
impl view::View for StatusView {
    fn get(&self, col: usize, row: usize) -> view::ViewElem {
        if row == 0 && col < self.x.len() {
            let c = self.x[col];
            (c, Color::Black, Color::White)
        } else {
            (' ', Color::Black, Color::White)
        }
    }
    fn get_cursor_pos(&self) -> Option<Cursor> {
        None
    }
}

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
            let (page_area, common_area) = area.split_vertical(area.height - 2);
            let (status_area, message_area) = common_area.split_vertical(1);
            let page = self.navigator.borrow().current.clone();
            let page_view = page.view_gen().gen(page_area);
            let view = page_view;

            let status_view = StatusView::new(&page.status());
            let status_view = view::TranslateView::new(
                status_view,
                status_area.col as i32,
                status_area.row as i32,
            );
            let view = view::MergeVertical {
                top: view,
                bottom: status_view,
                row_offset: status_area.row,
            };

            let message_view = message_box::View::new(page.message());
            let message_view = view::TranslateView::new(
                message_view,
                message_area.col as i32,
                message_area.row as i32,
            );
            let view = view::MergeVertical {
                top: view,
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

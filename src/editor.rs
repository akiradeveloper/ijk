extern crate flame;

use crate::message_box;
use crate::navigator::{self, Page};
use crate::screen::*;
use crate::view;
use crate::view::View;
use crate::Key;
use crate::read_buffer::Cursor;
use std::cell::RefCell;
use std::rc::Rc;
use std::{thread, time};
use termion::event::Key as TermKey;

struct StatusView {
    x: Vec<char>,
}
impl StatusView {
    fn new(x: &str) -> Self {
        let mut v = vec![];
        for c in x.chars() {
            v.push(c);
        }
        Self { x: v }
    }
}
impl view::View for StatusView {
    fn get(&self, col: usize, row: usize) -> view::ViewElem {
        if row == 0 && col < self.x.len() {
            let c = self.x[col];
            (Some(c), Some(Color::Black), Some(Color::White))
        } else {
            (Some(' '), Some(Color::Black), Some(Color::White))
        }
    }
    fn get_cursor_pos(&self) -> Option<Cursor> {
        None
    }
}

pub trait Drawable {
    fn dimension(&mut self) -> (usize, usize);
    fn draw<V: View>(&mut self, view: V);
}

pub struct TerminalScreen {
    screen: Screen,
}
impl TerminalScreen {
    pub fn new() -> Self {
        let (term_w, term_h) = termion::terminal_size().unwrap();
        Self {
            screen: Screen::new(term_w as usize, term_h as usize)
        }
    }
}
impl Drawable for TerminalScreen {
    fn dimension(&mut self) -> (usize, usize) {
        let (term_w, term_h) = termion::terminal_size().unwrap();
        self.screen.resize(term_w as usize, term_h as usize);
        (self.screen.w, self.screen.h)
    }
    fn draw<V: View>(&mut self, view: V) {
        let _flame_guard = flame::start_guard("editor.draw");

        flame::start("screen.clear");
        self.screen.clear();
        flame::end("screen.clear");

        flame::start("screen.draw");
        for row in 0..self.screen.h {
            for col in 0..self.screen.w {
                let (c, fg, bg) = view.get(col, row);
                self.screen.draw(col, row, c.unwrap(), Style(fg.unwrap(), bg.unwrap()))
            }
        }
        flame::end("screen.draw");

        for cursor in view.get_cursor_pos() {
            self.screen.move_cursor(cursor.col, cursor.row);
        }
        flame::start("screen.present");
        self.screen.present();
        flame::end("screen.present");
    }
}

struct NullScreen {
    w: usize,
    h: usize,
}
impl NullScreen {
    fn new(w: usize, h: usize) -> Self {
        Self { w, h }
    }
}
impl Drawable for NullScreen {
    fn dimension(&mut self) -> (usize, usize) {
        (self.w, self.h)
    }
    fn draw<V: View>(&mut self, view: V) {}
}

pub struct Editor<D> {
    navigator: Rc<RefCell<navigator::Navigator>>,
    navi_page: Rc<RefCell<Page>>,
    drawable: D,
}

impl <D: Drawable> Editor<D> {
    pub fn new(navigator: Rc<RefCell<navigator::Navigator>>, drawable: D) -> Self {
        Self {
            navi_page: Rc::new(RefCell::new(navigator::NavigatorPage::new(navigator.clone()))),
            navigator: navigator,
            drawable: drawable,
        }
    }
    fn view_gen(&self, area: view::Area) -> Box<View> {
        let _flame_guard = flame::start_guard("editor.view_gen");

        let (page_area, common_area) = area.split_vertical(area.height - 2);
        let (status_area, message_area) = common_area.split_vertical(1);
        let page = self.navigator.borrow().current_page();
        let page_view = page.borrow_mut().view_gen().gen(page_area);
        let view = page_view;

        let status_view = StatusView::new(&page.borrow().status());
        let status_view =
            view::TranslateView::new(status_view, status_area.col as i32, status_area.row as i32);
        let view = view::MergeVertical {
            top: view,
            bottom: status_view,
            row_offset: status_area.row,
        };

        let message = page.borrow().message();
        let message_view = message_box::View::new(&message);
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

        let view = view::CloneView::new(view, area);
        Box::new(view)
    }
    fn draw<V: View>(&mut self, view: V) {
        self.drawable.draw(view)
    }
    pub fn run<I: Iterator<Item = Result<termion::event::Key, std::io::Error>>>(
        &mut self,
        mut keys: I,
    ) {
        loop {
            let (w, h) = self.drawable.dimension();
            let area = view::Area {
                col: 0,
                row: 0,
                width: w,
                height: h,
            };
            let view = self.view_gen(area);
            self.draw(view);

            match keys.next() {
                Some(Ok(TermKey::Ctrl('z'))) => break,
                Some(Ok(TermKey::Ctrl('w'))) => {
                    self.navigator.borrow_mut().set(self.navi_page.clone());
                }
                other_key => {
                    let kk = match other_key {
                        Some(Ok(TermKey::Esc)) => Key::Esc,
                        Some(Ok(TermKey::Ctrl('c'))) => Key::Esc,
                        Some(Ok(TermKey::Backspace)) => Key::Backspace,
                        Some(Ok(TermKey::Ctrl(c))) => Key::Ctrl(c),
                        Some(Ok(TermKey::Char(c))) => Key::Char(c),
                        _ => {
                            continue;
                        }
                    };
                    let page = self.navigator.borrow().current_page();

                    flame::start("editor.receive");
                    page.borrow().controller().receive(kk);
                    flame::end("editor.receive");
                }
            }
        }
    }
}

extern crate test_generator;
#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::cell::RefCell;
    use std::rc::Rc;
    use crate::edit_buffer;
    use crate::navigator;
    use crate::read_buffer::BufElem;
    use super::*;

    fn normalize(x: Vec<Vec<BufElem>>) -> Vec<Vec<BufElem>> {
        let mut x = x;
        let last = x.last().cloned().unwrap();
        if last == vec![BufElem::Eol] {
            x.pop();
        }
        x
    }

    test_generator::test_expand_paths! { test_editor; "behavior/*" }
    fn test_editor(dir_name: &str) {
        let path = Path::new(dir_name);

        let input = path.join("input");

        let keys = path.join("keys");
        let keys = crate::util::read_keys_file(&keys);
        let keys = keys.into_iter();

        let output = path.join("output");
        let expected: Vec<Vec<BufElem>> = normalize(edit_buffer::read_buffer(&output));

        let navigator = Rc::new(RefCell::new(navigator::Navigator::new()));
        let eb = Rc::new(RefCell::new(edit_buffer::EditBuffer::open(&input, navigator.clone())));
        let page = Rc::new(RefCell::new(edit_buffer::Page::new(eb.clone())));
        navigator.borrow_mut().push(page);
        let mut editor = Editor::new(navigator, NullScreen::new(10,10));

        editor.run(keys);
        let actual: Vec<Vec<BufElem>> = normalize(eb.borrow().rb.buf.clone());

        assert_eq!(actual, expected);
    }
}

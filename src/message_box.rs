use crate::{BufElem, Cursor};
use crate::screen::Color;
use std::rc::Rc;
use std::cell::RefCell;

struct MessageBoxImpl {
    buf: Vec<BufElem>,
}
impl MessageBoxImpl {
    fn new() -> Self {
        Self {
            buf: vec![],
        }
    }
    fn send(&mut self, x: Vec<BufElem>) {
        self.buf = x;
    }
}

pub struct MessageBox {
    x: Rc<RefCell<MessageBoxImpl>>,
}
impl MessageBox {
    pub fn new() -> Self {
        Self {
            x: Rc::new(RefCell::new(MessageBoxImpl::new()))
        }
    }
    pub fn send(&self, x: Vec<BufElem>) {
        self.x.borrow_mut().send(x)
    }
}

use crate::view;
pub struct View {
    x: Rc<RefCell<MessageBoxImpl>>,
}
impl self::View {
    pub fn new(x: MessageBox) -> Self {
        Self { x: x.x }
    }
}
impl view::View for self::View {
    fn get(&self, col: usize, row: usize) -> view::ViewElem {
        if row == 0 && col < self.x.borrow().buf.len() {
            let c = match self.x.borrow().buf[col] {
                BufElem::Char(c) => c,
                BufElem::Eol => ' ',
            };
            (c, Color::White, Color::Black)
        } else {
            (' ', Color::White, Color::Black)
        }
    }
    fn get_cursor_pos(&self) -> Option<Cursor> {
        None
    }
}
use crate::{BufElem, Cursor};
use crate::screen::Color;
use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;
lazy_static! {
    pub static ref SINGLETON: MessageBox = MessageBox::new();
}

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

#[derive(Clone)]
pub struct MessageBox {
    x: Arc<Mutex<MessageBoxImpl>>,
}
impl MessageBox {
    pub fn new() -> Self {
        Self {
            x: Arc::new(Mutex::new(MessageBoxImpl::new()))
        }
    }
    pub fn send(&self, x: Vec<BufElem>) {
        self.x.lock().unwrap().send(x)
    }
}

use crate::view;
pub struct View {
    x: Arc<Mutex<MessageBoxImpl>>,
}
impl self::View {
    pub fn new(x: MessageBox) -> Self {
        Self { x: x.x }
    }
}
impl view::View for self::View {
    fn get(&self, col: usize, row: usize) -> view::ViewElem {
        if row == 0 && col < self.x.lock().unwrap().buf.len() {
            let c = match self.x.lock().unwrap().buf[col] {
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
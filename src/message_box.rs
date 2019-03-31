use crate::read_buffer::Cursor;
use crate::screen::Color;
use std::sync::{Arc, Mutex};

struct MessageBoxImpl {
    buf: Vec<char>,
}
impl MessageBoxImpl {
    fn new() -> Self {
        Self {
            buf: vec![],
        }
    }
    fn send(&mut self, x: Vec<char>) {
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
    pub fn send(&self, x: &str) {
        let mut v = vec![];
        for c in x.chars() {
            v.push(c);
        }
        self.x.lock().unwrap().send(v)
    }
}

use crate::view;
pub struct View<'a> {
    x: &'a MessageBox,
}
impl <'a> self::View<'a> {
    pub fn new(x: &'a MessageBox) -> Self {
        Self { x: x }
    }
}
impl <'a> view::View for self::View<'a> {
    fn get(&self, col: usize, row: usize) -> view::ViewElem {
        if row == 0 && col < self.x.x.lock().unwrap().buf.len() {
            let c = self.x.x.lock().unwrap().buf[col];
            (Some(c), Some(Color::White), Some(view::default_bg()))
        } else {
            (Some(' '), Some(Color::White), Some(view::default_bg()))
        }
    }
    fn get_cursor_pos(&self) -> Option<Cursor> {
        None
    }
}

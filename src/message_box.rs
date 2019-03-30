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
            let c = self.x.lock().unwrap().buf[col];
            (Some(c), Some(Color::White), Some(view::default_bg()))
        } else {
            (Some(' '), Some(Color::White), Some(view::default_bg()))
        }
    }
    fn get_cursor_pos(&self) -> Option<Cursor> {
        None
    }
}

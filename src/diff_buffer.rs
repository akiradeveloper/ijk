use crate::*;

#[derive(Clone)]
pub struct DiffBuffer {
    pub pre_buf: Vec<BufElem>,
    pub diff_buf: Vec<BufElem>,
    pub post_buf: Vec<BufElem>,
}

impl DiffBuffer {
    pub fn is_empty(&self) -> bool {
        self.pre_buf.is_empty() &&
        self.diff_buf.is_empty() &&
        self.post_buf.is_empty()
    }
    pub fn input(&mut self, k: Key) {
        match k {
            Key::Backspace => {
                self.diff_buf.pop();
            },
            Key::Char('\n') => {
                self.diff_buf.push(BufElem::Eol);
            },
            Key::Char(c) => {
                self.diff_buf.push(BufElem::Char(c))
            },
            _ => {}
        }
    }
}
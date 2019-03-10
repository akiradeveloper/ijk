use crate::*;
use crate::indent;

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
                let mut v1 = self.pre_buf.clone();
                let mut v2 = self.diff_buf.clone();
                v1.append(&mut v2);
                let start_of_cur_line = if v1.is_empty() {
                    0
                } else {
                    let mut i = v1.len();
                    while v1[i-1] != BufElem::Eol {
                        i -= 1;
                        if i == 0 {
                            break;
                        }
                    }
                    i
                };
                let auto_indent = indent::AutoIndent {
                    line_predecessors: &v1[start_of_cur_line..v1.len()],
                };
                let mut v = vec![BufElem::Eol];
                v.append(&mut auto_indent.next_indent());

                self.diff_buf.append(&mut v);
            },
            Key::Char(c) => {
                self.diff_buf.push(BufElem::Char(c))
            },
            _ => {}
        }
    }
}
use super::{BufElem, Key};
use super::indent;

#[derive(Clone)]
pub struct DiffBuffer {
    pub pre_buf_raw: Vec<BufElem>,
    pub diff_buf_pre: Vec<BufElem>,
    pub diff_buf_raw: Vec<BufElem>,
    pub diff_buf_post: Vec<BufElem>,
    pub post_buf_raw: Vec<BufElem>,
}

fn concat<T>(x: Vec<T>, y: Vec<T>) -> Vec<T> {
    let mut x = x;
    let mut y = y;
    x.append(&mut y);
    x
}

// pre_buf_raw + inserted() + post_buf_raw = pre_buf() + diff_buf_raw + post_buf()
impl DiffBuffer {
    pub fn pre_buf(&self) -> Vec<BufElem> {
        concat(self.pre_buf_raw.clone(), self.diff_buf_pre.clone())
    }
    pub fn post_buf(&self) -> Vec<BufElem> {
        concat(self.diff_buf_post.clone(), self.post_buf_raw.clone())
    }
    pub fn inserted(&self) -> Vec<BufElem> {
        concat(concat(self.diff_buf_pre.clone(), self.diff_buf_raw.clone()), self.diff_buf_post.clone())
    }
    pub fn input(&mut self, k: Key) {
        match k {
            Key::Backspace => {
                self.diff_buf_raw.pop();
            },
            Key::Char('\n') => {
                let mut v1 = self.pre_buf_raw.clone();
                v1.append(&mut self.diff_buf_pre.clone());
                v1.append(&mut self.diff_buf_raw.clone());
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

                self.diff_buf_raw.append(&mut v);
            },
            Key::Char(c) => {
                self.diff_buf_raw.push(BufElem::Char(c))
            },
            _ => {}
        }
    }
}
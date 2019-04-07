use super::{BufElem, Key};
use super::indent::{self, IndentType};
use super::diff_tree::DiffTree;

pub struct DiffBuffer {
    pub pre_buf_raw: Vec<BufElem>,
    pub diff_buf_pre: Vec<BufElem>,
    pub diff_buf_raw: DiffTree,
    pub diff_buf_post: Vec<BufElem>,
    pub post_buf_raw: Vec<BufElem>,
    indent_type: IndentType,
}

fn concat<T>(x: Vec<T>, y: Vec<T>) -> Vec<T> {
    let mut x = x;
    let mut y = y;
    x.append(&mut y);
    x
}

// pre_buf_raw + inserted() + post_buf_raw = pre_buf() + diff_buf_raw + post_buf()
impl DiffBuffer {
    pub fn new(pre_buf: Vec<BufElem>, diff_buf_pre: Vec<BufElem>, diff_buf_post: Vec<BufElem>, post_buf: Vec<BufElem>, indent_type: IndentType) -> Self {
        let mut pre_buffer = vec![];
        pre_buffer.append(&mut pre_buf.clone());
        pre_buffer.append(&mut diff_buf_pre.clone());

        Self {
            pre_buf_raw: pre_buf,
            diff_buf_pre: diff_buf_pre,
            diff_buf_raw: DiffTree::new(pre_buffer, indent_type),
            diff_buf_post: diff_buf_post,
            post_buf_raw: post_buf,
            indent_type,
        }
    }
    pub fn pre_buf(&self) -> Vec<BufElem> {
        concat(self.pre_buf_raw.clone(), self.diff_buf_pre.clone())
    }
    pub fn post_buf(&self) -> Vec<BufElem> {
        concat(self.diff_buf_post.clone(), self.post_buf_raw.clone())
    }
    pub fn inserted(&self) -> Vec<BufElem> {
        concat(concat(self.diff_buf_pre.clone(), self.diff_buf_raw.flatten().0), self.diff_buf_post.clone())
    }
    pub fn input(&mut self, k: Key) {
        self.diff_buf_raw.input(k);
    }
}
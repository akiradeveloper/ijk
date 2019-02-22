use crate::*;

#[derive(Clone)]
pub struct DiffBuffer {
    pub buf: Vec<BufElem>,
    init_pos: usize,
    pos: usize,
}

#[derive(PartialEq, Debug)]
pub enum CursorDiff {
    Forward,
    Backward,
    Up,
    Down,
    None,
}

impl DiffBuffer {
    pub fn new(line: Vec<BufElem>, col: usize) -> DiffBuffer {
        DiffBuffer {
            buf: line,
            init_pos: col,
            pos: col,
        }
    }
    pub fn collect_inserted(&self) -> Vec<BufElem> {
        self.buf[self.init_pos .. self.pos].to_vec()
    }
    pub fn input(&mut self, k: Key) -> CursorDiff {
        match k {
            Key::Backspace => {
                if self.pos <= self.init_pos {
                    CursorDiff::None
                } else {
                    let e = self.buf[self.pos - 1].clone(); self.buf.remove(self.pos - 1);
                    self.pos -= 1;
                    if e == BufElem::Eol {
                        CursorDiff::Up
                    } else {
                        CursorDiff::Backward
                    }
                }
            },
            Key::Char('\n') => {
                self.buf.insert(self.pos, BufElem::Eol);
                // TODO auto-indent
                self.pos += 1;
                CursorDiff::Down
            },
            Key::Char(c) => {
                self.buf.insert(self.pos, BufElem::Char(c));
                self.pos += 1;
                CursorDiff::Forward
            },
            _ => CursorDiff::None
        }
    }
}

#[test]
fn test_diff_buffer() {
    use crate::BufElem::*;
    use crate::diff_buffer::CursorDiff::*;

    let mut df = DiffBuffer::new(vec![Char('a'),Char('b'),Eol], 1);
    assert_eq!(df.input(Key::Char('a')), Forward); // -> aa[b]e
    assert_eq!(df.input(Key::PageUp), None);
    assert_eq!(df.input(Key::Char('\n')), Down); // -> aae[b]e
    assert_eq!(df.input(Key::Backspace), Up); // -> aa[b]e
    assert_eq!(df.input(Key::Backspace), Backward); // -> a[b]e
    assert_eq!(df.input(Key::Backspace), None); // -> a[b]e
}
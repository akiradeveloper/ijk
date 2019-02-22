use crate::*;

#[derive(Clone)]
pub struct DiffBuffer {
    pub buf: Vec<BufElem>,
    init_pos: usize,
    pos: usize,
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
    pub fn input(&mut self, k: Key) -> Option<i8> {
        match k {
            Key::Backspace => {
                if self.pos <= self.init_pos {
                    None
                } else {
                    let e = self.buf[self.pos - 1].clone(); self.buf.remove(self.pos - 1);
                    self.pos -= 1;
                    if e == BufElem::Eol {
                        Some(-1)
                    } else {
                        Some(0)
                    }
                }
            },
            Key::Char('\n') => {
                self.buf.insert(self.pos, BufElem::Eol);
                // TODO auto-indent
                self.pos += 1;
                Some(1)
            },
            Key::Char(c) => {
                self.buf.insert(self.pos, BufElem::Char(c));
                self.pos += 1;
                Some(0)
            },
            _ => None
        }
    }
}

#[test]
fn test_diff_buffer() {
    use crate::BufElem::*;

    let mut df = DiffBuffer::new(vec![Char('a'),Char('b'),Eol], 1);
    assert_eq!(df.input(Key::Char('a')), Some(0)); // -> aa[b]e
    assert_eq!(df.input(Key::PageUp), None);
    assert_eq!(df.input(Key::Char('\n')), Some(1)); // -> aae[b]e
    assert_eq!(df.input(Key::Backspace), Some(-1)); // -> aa[b]e
    assert_eq!(df.input(Key::Backspace), Some(0)); // -> a[b]e
    assert_eq!(df.input(Key::Backspace), None); // -> a[b]e
}
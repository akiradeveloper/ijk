use crate::*;

struct DiffBuffer {
    stack: Vec<char>
}

impl DiffBuffer {
    fn new() -> DiffBuffer {
        DiffBuffer {
            stack: vec![]
        }
    }
    fn len(&self) -> usize {
        self.stack.len()
    }
    fn input(&mut self, k: Key) -> Option<i8> {
        match k {
            Key::Char(c) => {
                self.stack.push(c);
                if c == '\n' {
                    Some(1)
                } else {
                    Some(0)
                }
            },
            Key::Backspace => {
                let c0 = self.stack.last().cloned();
                match c0 {
                    Some(c) => {
                        self.stack.pop();
                        if c == '\n' {
                            Some(-1)
                        } else {
                            Some(0)
                        }
                    },
                    None => None
                }
            },
            _ => None
        }
    }
}

#[test]
fn test_diff_buffer() {
    let mut df = DiffBuffer::new();
    assert_eq!(df.input(Key::Char('a')), Some(0));
    assert_eq!(df.len(), 1);
    assert_eq!(df.input(Key::PageUp), None);
    assert_eq!(df.input(Key::Char('\n')), Some(1));
    assert_eq!(df.len(), 2);
    assert_eq!(df.input(Key::Backspace), Some(-1));
    assert_eq!(df.len(), 1);
    assert_eq!(df.input(Key::Backspace), Some(0));
    assert_eq!(df.len(), 0);
    assert_eq!(df.input(Key::Backspace), None);
}
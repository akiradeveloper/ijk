use crate::BufElem;

#[derive(Clone, PartialOrd, PartialEq)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
}

#[derive(Clone)]
pub struct CursorRange {
    pub start: Cursor,
    pub end: Cursor,
}

pub struct EditBuffer {
    pub buf: Vec<Vec<BufElem>>,
    pub cursor: Cursor,
    visual_cursor: Option<Cursor>,
}

impl EditBuffer {
    pub fn new() -> EditBuffer {
        EditBuffer {
            buf: vec![vec![]],
            cursor: Cursor { row: 0, col: 0 },
            visual_cursor: None,
        }
    }
    pub fn visual_range(&self) -> Option<CursorRange> {
        self.visual_cursor.clone().map(|vc| 
            if self.cursor > vc {
                CursorRange { start: vc, end: self.cursor.clone() }
            } else {
                CursorRange { start: self.cursor.clone(), end: vc }
            }
        )
    }
    pub fn reset_with(&mut self, new_buf: Vec<Vec<BufElem>>) {
        self.buf = new_buf;
    }
    fn expand_range(&self, r: &CursorRange) -> Vec<(usize, std::ops::Range<usize>)> {
        let mut res = vec![];
        for row in r.start.row .. r.end.row + 1 {
            let col_start = if row == r.start.row {
                r.start.col
            } else {
                0
            };
            let col_end = if row == r.end.row {
                r.end.col + 1
            } else {
                self.buf[row].len()
            };
            res.push((row, col_start .. col_end));
        }
        res
    }
    fn insert(&mut self, at: Cursor, buf: Vec<BufElem>) {
        let mut row = at.row;
        let mut col = at.col;
        let mut should_insert_newline = false;
        for e in buf {
            if should_insert_newline {
                row += 1;
                col = 0;
                self.buf.insert(row, vec![]);
                should_insert_newline = false;
            }
            match e {
                BufElem::Eol => {
                    self.buf[row].insert(col, BufElem::Eol);
                    should_insert_newline = true;
                },
                c @ BufElem::Char(_) => {
                    self.buf[row].insert(col, c);
                    col += 1;
                }
            }
        }
    }
    pub fn receive(&mut self, act: Action) {
        match act {
            Action::Reset => {
                self.visual_cursor = None
            },
            Action::Delete => {
                if self.visual_range().is_none() { return; }
                let vr = self.visual_range().unwrap();
                let mut survivors = vec![];
                let mut removed = vec![];

                for (row, col_range) in self.expand_range(&vr) {
                    for col in 0 .. self.buf[row].len() {
                        if col_range.start <= col && col < col_range.end {
                            removed.push(self.buf[row][col].clone())
                        } else {
                            survivors.push(self.buf[row][col].clone())
                        }
                    }
                }
                for (row, col_range) in self.expand_range(&vr).into_iter().rev() {
                    self.buf.remove(row);
                }
                if !survivors.is_empty() {
                    self.buf.insert(vr.start.row, vec![])
                }
                self.insert(Cursor { row: vr.start.row, col: 0 }, survivors);

                self.cursor = vr.start;
                self.visual_cursor = None;
            },
            Action::EnterVisualMode => {
                self.visual_cursor = Some(self.cursor.clone())
            },
            Action::CursorUp => {
                if self.cursor.row > 0 { self.cursor.row -= 1; }
            },
            Action::CursorDown => {
                if self.cursor.row < self.buf.len() - 1 { self.cursor.row += 1; }
            },
            Action::CursorLeft => {
                if self.cursor.col > 0 { self.cursor.col -= 1; }
            },
            Action::CursorRight => {
                if self.cursor.col < self.buf[self.cursor.row].len() - 1 { self.cursor.col += 1; }
            },
            Action::JumpLineHead => {
                self.cursor.col = 0;
            },
            Action::JumpLineLast => {
                self.cursor.col = self.buf[self.cursor.row].len() - 1;
            },
            Action::Jump(row) => {
                self.cursor.row = row;
                self.cursor.col = 0;
            },
            Action::JumpLast => {
                self.cursor.row = self.buf.len() - 1;
                self.cursor.col = 0;
            },
            Action::None => {}
        }
    }
}

pub enum Action {
    Delete,
    CursorUp,
    CursorDown,
    CursorLeft,
    CursorRight,
    JumpLineHead,
    JumpLineLast,
    Jump(usize),
    JumpLast,
    EnterVisualMode,
    Reset,
    None,
}

use crate::automaton as AM; use crate::Key;
use crate::Key::*;
pub struct KeyReceiver {
    automaton: AM::Node,
    parser: AM::Parser,
}
fn mk_automaton() -> AM::Node {
    let init = AM::Node::new("init");
    let num = AM::Node::new("num");

    init.add_trans(AM::Edge::new(Char('d')), &init);
    init.add_trans(AM::Edge::new(Char('v')), &init);
    init.add_trans(AM::Edge::new(Char('k')), &init);
    init.add_trans(AM::Edge::new(Char('j')), &init);
    init.add_trans(AM::Edge::new(Char('h')), &init);
    init.add_trans(AM::Edge::new(Char('l')), &init);
    init.add_trans(AM::Edge::new(Char('G')), &init);
    init.add_trans(AM::Edge::new(Char('0')), &init);
    init.add_trans(AM::Edge::new(Char('$')), &init);
    init.add_trans(AM::Edge::new(CharRange('1','9')), &num);
    num.add_trans(AM::Edge::new(CharRange('0','9')), &num);
    num.add_trans(AM::Edge::new(Char('G')), &init);

    init.add_trans(AM::Edge::new(Esc), &init);
    num.add_trans(AM::Edge::new(Esc), &init);

    init
}
use std::str::FromStr;
impl KeyReceiver {
    pub fn new() -> KeyReceiver {
        let init = mk_automaton();
        KeyReceiver {
            parser: AM::Parser::new(&init),
            automaton: init,
        }
    }
    pub fn receive(&mut self, k: Key) -> Action {
        self.parser.feed(k);
        let cur_node: &str = &self.parser.cur_node.name();
        let prev_node: &str = &self.parser.prev_node.clone().unwrap().name();
        let last0 = self.parser.rec.back().cloned();
        let mut reset_parser = true;
        let act = match (prev_node, cur_node, last0) {
            (_, _, Some(Esc)) => Action::Reset,
            ("init", "init", Some(Char('d'))) => Action::Delete,
            ("init", "init", Some(Char('v'))) => Action::EnterVisualMode,
            ("init", "init", Some(Char('k'))) => Action::CursorUp,
            ("init", "init", Some(Char('j'))) => Action::CursorDown,
            ("init", "init", Some(Char('h'))) => Action::CursorLeft,
            ("init", "init", Some(Char('l'))) => Action::CursorRight,
            ("init", "init", Some(Char('0'))) => Action::JumpLineHead,
            ("init", "init", Some(Char('$'))) => Action::JumpLineLast,
            ("init", "init", Some(Char('G'))) => Action::JumpLast,
            ("num", "init", Some(Char('G'))) => {
                self.parser.rec.pop_back(); // eliminate EOL
                let cs = self.parser.rec.iter().map(|k| match k.clone() {
                    Char(c) => c,
                    _ => panic!()
                });
                let mut s = String::new();
                for c in cs {
                    s.push(c);
                }
                let n = s.parse::<usize>().unwrap();
                Action::Jump(n-1) // convert to 0-origin
            },
            _ => {
                reset_parser = false;
                Action::None
            },
        };
        if reset_parser {
            self.parser.reset(&self.automaton);
        }
        act
    }
}
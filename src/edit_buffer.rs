use crate::BufElem;
use crate::undo_buffer::UndoBuffer;
use crate::diff_buffer::DiffBuffer;

#[derive(Copy, Clone, PartialOrd, PartialEq)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
}

impl Cursor {
    fn to_cursor_end(&self) -> Cursor {
        Cursor {
            row: self.row,
            col: self.col+1,
        }
    }
}

#[derive(Copy, Clone)]
pub struct CursorRange {
    pub start: Cursor,
    pub end: Cursor,
}

#[derive(Clone)]
struct ChangeLog {
    at: Cursor,
    deleted: Vec<BufElem>,
    inserted: Vec<BufElem>,
}

pub struct EditBuffer {
    pub buf: Vec<Vec<BufElem>>,
    pub cursor: Cursor,
    visual_cursor: Option<Cursor>,
    change_log_buffer: UndoBuffer<ChangeLog>,
    edit_state: Option<EditState>,
}

#[derive(Clone)]
struct EditState {
    diff_buffer: DiffBuffer,
    at: Cursor,
    removed: Vec<BufElem>,
    orig_buf: Vec<Vec<BufElem>>,
}

impl EditBuffer {
    pub fn new() -> EditBuffer {
        EditBuffer {
            buf: vec![vec![]],
            cursor: Cursor { row: 0, col: 0 },
            visual_cursor: None,
            change_log_buffer: UndoBuffer::new(20),
            edit_state: None,
        }
    }
    fn undo(&mut self) -> bool {
        let log = self.change_log_buffer.pop_undo();
        if log.is_none() { return false }
        let mut log = log.unwrap();

        let delete_range = CursorRange {
            start: log.at,
            end: self.find_cursor_pair(log.at, log.inserted.len()),
        };
        let (mut pre_survivors, _, mut post_survivors) = self.prepare_delete(&delete_range);
        pre_survivors.append(&mut log.deleted);
        pre_survivors.append(&mut post_survivors);
        if !pre_survivors.is_empty() {
            self.buf.insert(log.at.row, vec![])
        }
        let mut b = false;
        self.insert(Cursor { row: log.at.row, col: 0 }, pre_survivors, &mut b);
        self.cursor = log.at;
        true
    }
    fn redo(&mut self) -> bool {
        let log = self.change_log_buffer.pop_redo();
        if log.is_none() { return false }
        let mut log = log.unwrap();

        let delete_range = CursorRange {
            start: log.at,
            end: self.find_cursor_pair(log.at, log.deleted.len()),
        };
        let (mut pre_survivors, _, mut post_survivors) = self.prepare_delete(&delete_range);
        let n_inserted = log.inserted.len();
        pre_survivors.append(&mut log.inserted);
        pre_survivors.append(&mut post_survivors);
        if !pre_survivors.is_empty() {
            self.buf.insert(log.at.row, vec![])
        }
        let mut b = false;
        self.insert(Cursor { row: log.at.row, col: 0 }, pre_survivors, &mut b);
        self.cursor = self.find_cursor_pair(log.at, n_inserted);
        true
    }
    pub fn visual_range(&self) -> Option<CursorRange> {
        self.visual_cursor.clone().map(|vc| 
            if self.cursor > vc {
                CursorRange { start: vc, end: self.cursor.to_cursor_end() }
            } else {
                CursorRange { start: self.cursor.clone(), end: vc.to_cursor_end() }
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
                r.end.col
            } else {
                self.buf[row].len()
            };
            res.push((row, col_start .. col_end));
        }
        res
    }
    fn insert(&mut self, at: Cursor, buf: Vec<BufElem>, should_insert_newline: &mut bool) -> Cursor {
        let mut row = at.row;
        let mut col = at.col;
        for e in buf {
            if *should_insert_newline {
                self.buf.insert(row, vec![]);
                *should_insert_newline = false;
            }
            match e {
                x @ BufElem::Eol => {
                    self.buf[row].insert(col, x);
                    *should_insert_newline = true;
                    row += 1;
                    col = 0;
                },
                x @ BufElem::Char(_) => {
                    self.buf[row].insert(col, x);
                    col += 1;
                }
            }
        }
        Cursor { row: row, col: col }
    }
    fn is_eof(&self, row: usize, col: usize) -> bool {
        row == self.buf.len() - 1 && col == self.buf[row].len() - 1
    }
    fn find_cursor_pair(&self, cursor: Cursor, len: usize) -> Cursor {
        let mut row = cursor.row;
        let mut col = cursor.col;
        let mut remaining = len;
        while remaining > 0 {
            let n = std::cmp::min(remaining, self.buf[row].len() - col);
            remaining -= n;
            if remaining > 0 {
                col = 0;
                row += 1;
            } else {
                col += n;
            }
        }
        Cursor { row: row, col: col }
    }
    fn prepare_delete(&mut self, range: &CursorRange) -> (Vec<BufElem>, Vec<BufElem>, Vec<BufElem>) {
        let mut pre_survivors = vec![];
        let mut post_survivors = vec![];
        let mut removed = vec![];

        // if the end of the range is some eol then the current line should be joined with the next line
        //
        // Before ([] is the range):
        // xxxx[xxe]
        // xxe
        //
        // After:
        // xxxxxxe
        let target_region = if range.end.col == self.buf[range.end.row].len() && range.end.row != self.buf.len() - 1 {
            let mut res = self.expand_range(&range);
            res.push((range.end.row+1, 0..0));
            res
        } else {
            self.expand_range(&range)
        };

        // the characters in the range will be deleted and
        // others will survive, be merged and inserted afterward
        for (row, col_range) in target_region.clone() {
            for col in 0 .. self.buf[row].len() {
                if self.is_eof(row, col) {
                    post_survivors.push(self.buf[row][col].clone())
                } else if col_range.start <= col && col < col_range.end {
                    removed.push(self.buf[row][col].clone())
                } else {
                    let as_cursor = Cursor { row, col };
                    if as_cursor < range.start {
                        pre_survivors.push(self.buf[row][col].clone())
                    } else {
                        post_survivors.push(self.buf[row][col].clone())
                    }
                }
            }
        }
        for (row, _) in target_region.into_iter().rev() {
            self.buf.remove(row);
        }

        (pre_survivors, removed, post_survivors)
    }
    fn enter_update_mode(&mut self, r: &CursorRange, init_diff: Vec<BufElem>) {
        let (pre_survivors, removed, post_survivors) = self.prepare_delete(&r);
        let orig_buf = self.buf.clone();
        self.edit_state = Some(EditState {
            diff_buffer: DiffBuffer { pre_buf: pre_survivors, diff_buf: init_diff, post_buf: post_survivors },
            at: r.start,
            removed: removed,
            orig_buf: orig_buf
        });

        // write back the initial diff buffer
        let es = self.edit_state.clone().unwrap();
        assert!(!es.diff_buffer.is_empty());
        self.buf.insert(es.at.row, vec![]);
        let mut b = false;
        let after_pre_inserted = self.insert(Cursor { row: es.at.row, col: 0 }, es.diff_buffer.pre_buf, &mut b);
        let after_diff_inserted = self.insert(after_pre_inserted, es.diff_buffer.diff_buf, &mut b);
        self.insert(after_diff_inserted, es.diff_buffer.post_buf, &mut b);
        self.cursor = after_diff_inserted;
        self.visual_cursor = None;
    }
}

#[derive(Clone)]
pub enum Action {
    EnterInsertMode,
    EnterChangeMode,
    EnterInsertNewLine,
    EditModeInput(Key),
    LeaveEditMode,
    Redo,
    Undo,
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

use crate::controller::ActionController;
impl ActionController for EditBuffer {
    type Action = Action;
    fn receive(&mut self, act: Action) -> bool {
        match act {
            Action::EnterInsertNewLine => {
                let row = self.cursor.row;
                let delete_range = CursorRange {
                    start: Cursor { row: row, col: self.buf[row].len() - 1 },
                    end: Cursor { row: row, col: self.buf[row].len() - 1 },
                };
                self.enter_update_mode(&delete_range, vec![BufElem::Eol]);
            },
            Action::EnterInsertMode => {
                assert!(self.edit_state.is_none());
                let delete_range = CursorRange {
                    start: self.cursor,
                    end: self.cursor,
                };
                self.enter_update_mode(&delete_range, vec![]);
            },
            Action::EnterChangeMode => {
                if self.visual_range().is_none() { return true }
                let vr = self.visual_range().unwrap();
                self.enter_update_mode(&vr, vec![]);
            },
            Action::EditModeInput(k) => {
                let es = self.edit_state.as_mut().unwrap();
                let cursor_diff = es.diff_buffer.input(k.clone());

                let es = self.edit_state.clone().unwrap();
                self.buf = es.orig_buf;
                assert!(!es.diff_buffer.is_empty());
                self.buf.insert(es.at.row, vec![]);
                let mut b = false;
                let after_pre_inserted = self.insert(Cursor { row: es.at.row, col: 0 }, es.diff_buffer.pre_buf, &mut b);
                let after_diff_inserted = self.insert(after_pre_inserted, es.diff_buffer.diff_buf, &mut b);
                self.insert(after_diff_inserted, es.diff_buffer.post_buf, &mut b);
                self.cursor = after_diff_inserted;
            },
            Action::LeaveEditMode => {
                assert!(self.edit_state.is_some());
                // take(): replace the memory region with None and take out the owrnership of the object
                let edit_state = self.edit_state.take().unwrap();
                assert!(self.edit_state.is_none());
                let change_log = ChangeLog {
                    at: edit_state.at,
                    deleted: edit_state.removed,
                    inserted: edit_state.diff_buffer.diff_buf,
                };
                if change_log.deleted.len() > 0 || change_log.inserted.len() > 0 {
                    self.change_log_buffer.save(change_log);
                }
            },
            Action::Undo => {
                self.undo();
            },
            Action::Redo => {
                self.redo();
            },
            Action::Reset => {
                self.visual_cursor = None
            },
            Action::Delete => {
                if self.visual_range().is_none() { return true }
                let vr = self.visual_range().unwrap();
                let (mut pre_survivors, removed, mut post_survivors) = self.prepare_delete(&vr);

                pre_survivors.append(&mut post_survivors);
                if !pre_survivors.is_empty() {
                    self.buf.insert(vr.start.row, vec![])
                }
                let mut b = false;
                self.insert(Cursor { row: vr.start.row, col: 0 }, pre_survivors, &mut b);

                let log = ChangeLog {
                    at: vr.start.clone(),
                    deleted: removed,
                    inserted: vec![],
                };
                self.change_log_buffer.save(log);

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
            Action::None => { return false }
        }
        true
    }
}

use crate::automaton as AM; use crate::Key;
use crate::Key::*;
struct ActionGen {
    automaton: AM::Node,
    parser: AM::Parser,
}
fn mk_automaton() -> AM::Node {
    let init = AM::Node::new("init");
    let num = AM::Node::new("num");
    let edit = AM::Node::new("edit");

    init.add_trans(AM::Edge::new(Char('o')), &edit);
    init.add_trans(AM::Edge::new(Char('i')), &edit);
    init.add_trans(AM::Edge::new(Char('c')), &edit);
    init.add_trans(AM::Edge::new(Ctrl('r')), &init);
    init.add_trans(AM::Edge::new(Char('u')), &init);
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
    init.add_trans(AM::Edge::new(Esc), &init);

    num.add_trans(AM::Edge::new(CharRange('0','9')), &num);
    num.add_trans(AM::Edge::new(Char('G')), &init);
    num.add_trans(AM::Edge::new(Esc), &init);

    edit.add_trans(AM::Edge::new(Esc), &init);
    edit.add_trans(AM::Edge::new(Otherwise), &edit);

    init
}
use std::str::FromStr;
impl ActionGen {
    fn new() -> ActionGen {
        let init = mk_automaton();
        ActionGen {
            parser: AM::Parser::new(&init),
            automaton: init,
        }
    }
    fn receive(&mut self, k: Key) -> Action {
        if !self.parser.feed(k) {
            return Action::None
        }
        let cur_node: &str = &self.parser.cur_node.name();
        let prev_node: &str = &self.parser.prev_node.clone().unwrap().name();
        let last0 = self.parser.rec.back().cloned();
        let mut keep_parser_rec = false;
        let act = match (prev_node, cur_node, last0) {
            ("init", "init", Some(Char('k'))) => Action::CursorUp,
            ("init", "init", Some(Char('j'))) => Action::CursorDown,
            ("init", "init", Some(Char('h'))) => Action::CursorLeft,
            ("init", "init", Some(Char('l'))) => Action::CursorRight,
            ("init", "init", Some(Char('0'))) => Action::JumpLineHead,
            ("init", "init", Some(Char('$'))) => Action::JumpLineLast,
            ("init", "init", Some(Char('G'))) => Action::JumpLast,
            ("init", "init", Some(Esc)) => Action::Reset,
            ("init", "num", _) => {
                keep_parser_rec = true;
                Action::None
            },
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
            ("num", "num", _) => {
                keep_parser_rec = true;
                Action::None
            },
            ("num", "init", Some(Esc)) => Action::Reset,
            ("init", "edit", Some(Char('i'))) => Action::EnterInsertMode,
            ("init", "edit", Some(Char('c'))) => Action::EnterChangeMode,
            ("init", "edit", Some(Char('o'))) => Action::EnterInsertNewLine,
            ("edit", "edit", Some(k)) => Action::EditModeInput(k),
            ("edit", "init", Some(Esc)) => Action::LeaveEditMode,
            ("init", "init", Some(Char('v'))) => Action::EnterVisualMode,
            ("init", "init", Some(Char('d'))) => Action::Delete,
            ("init", "init", Some(Ctrl('r'))) => Action::Redo,
            ("init", "init", Some(Char('u'))) => Action::Undo,
            _ => Action::None, // hope this is unreachable
        };
        if !keep_parser_rec {
            self.parser.clear_rec()
        }
        act
    }
}

use std::rc::Rc;
use std::cell::RefCell;
pub struct Controller {
    action_gen : ActionGen,
    action_ctrl: Rc<RefCell<EditBuffer>>,
}
impl Controller {
    pub fn new(ctrl: Rc<RefCell<EditBuffer>>) -> Self {
        Self {
            action_gen: ActionGen::new(),
            action_ctrl: ctrl,
        }
    }
}
use crate::controller::KeyController;
impl KeyController for Controller {
    fn receive(&mut self, key: crate::Key) {
        let act = self.action_gen.receive(key);
        self.action_ctrl.borrow_mut().receive(act);
    }
}

use crate::visibility_filter::VisibilityFilter;
pub struct ViewGen {
    buf: Rc<RefCell<EditBuffer>>,
    filter: VisibilityFilter,
    old_region: view::ViewRegion,
}
impl ViewGen {
    pub fn new(buf: Rc<RefCell<EditBuffer>>) -> Self {
        Self {
            buf: buf,
            filter: VisibilityFilter::new(Cursor{
                col: 0,
                row: 0,
            }),
            old_region: view::ViewRegion {
                col: 0,
                row: 0,
                width: 0,
                height: 0,
            }
        }
    }
}
use crate::view;
impl view::ViewGen for ViewGen {
    fn gen(&mut self, region: view::ViewRegion) -> Box<view::View> {
        if self.old_region != region {
            self.filter.resize(region.width, region.height);
            self.old_region = region;
        }
        self.filter.adjust(self.buf.borrow().cursor);
        let view = view::ToView::new(self.buf.borrow().buf.clone());
        let view = view::OverlayView::new(
            view,
            view::VisualRangeDiffView::new(self.buf.borrow().visual_range())
        );
        let view = view::AddCursor::new(
            view,
            Some(self.buf.borrow().cursor), // tmp: the cursor is always visible
        );
        let view = view::TranslateView::new(
            view,
            region.col as i32 - self.filter.col() as i32,
            region.row as i32 - self.filter.row() as i32,
        );
        Box::new(view)
    }
}
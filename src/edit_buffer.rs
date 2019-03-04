use crate::diff_buffer::DiffBuffer;
use crate::undo_buffer::UndoBuffer;
use crate::BufElem;

#[derive(Copy, Clone, PartialOrd, PartialEq)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
}

impl Cursor {
    fn to_cursor_end(&self) -> Cursor {
        Cursor {
            row: self.row,
            col: self.col + 1,
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
    num_buffer: Vec<char>,
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
            num_buffer: vec![],
            visual_cursor: None,
            change_log_buffer: UndoBuffer::new(20),
            edit_state: None,
        }
    }
    fn _undo(&mut self) -> bool {
        let log = self.change_log_buffer.pop_undo();
        if log.is_none() {
            return false;
        }
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
        self.insert(
            Cursor {
                row: log.at.row,
                col: 0,
            },
            pre_survivors,
            &mut b,
        );
        self.cursor = log.at;
        true
    }
    fn _redo(&mut self) -> bool {
        let log = self.change_log_buffer.pop_redo();
        if log.is_none() {
            return false;
        }
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
        self.insert(
            Cursor {
                row: log.at.row,
                col: 0,
            },
            pre_survivors,
            &mut b,
        );
        self.cursor = self.find_cursor_pair(log.at, n_inserted);
        true
    }
    pub fn visual_range(&self) -> Option<CursorRange> {
        self.visual_cursor.clone().map(|vc| {
            if self.cursor > vc {
                CursorRange {
                    start: vc,
                    end: self.cursor.to_cursor_end(),
                }
            } else {
                CursorRange {
                    start: self.cursor.clone(),
                    end: vc.to_cursor_end(),
                }
            }
        })
    }
    pub fn reset_with(&mut self, new_buf: Vec<Vec<BufElem>>) {
        self.buf = new_buf;
    }
    fn expand_range(&self, r: &CursorRange) -> Vec<(usize, std::ops::Range<usize>)> {
        let mut res = vec![];
        for row in r.start.row..r.end.row + 1 {
            let col_start = if row == r.start.row { r.start.col } else { 0 };
            let col_end = if row == r.end.row {
                r.end.col
            } else {
                self.buf[row].len()
            };
            res.push((row, col_start..col_end));
        }
        res
    }
    fn insert(
        &mut self,
        at: Cursor,
        buf: Vec<BufElem>,
        should_insert_newline: &mut bool,
    ) -> Cursor {
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
                }
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
    fn prepare_delete(
        &mut self,
        range: &CursorRange,
    ) -> (Vec<BufElem>, Vec<BufElem>, Vec<BufElem>) {
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
        let target_region = if range.end.col == self.buf[range.end.row].len()
            && range.end.row != self.buf.len() - 1
        {
            let mut res = self.expand_range(&range);
            res.push((range.end.row + 1, 0..0));
            res
        } else {
            self.expand_range(&range)
        };

        // the characters in the range will be deleted and
        // others will survive, be merged and inserted afterward
        for (row, col_range) in target_region.clone() {
            for col in 0..self.buf[row].len() {
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
            diff_buffer: DiffBuffer {
                pre_buf: pre_survivors,
                diff_buf: init_diff,
                post_buf: post_survivors,
            },
            at: r.start,
            removed: removed,
            orig_buf: orig_buf,
        });

        // write back the initial diff buffer
        let es = self.edit_state.clone().unwrap();
        assert!(!es.diff_buffer.is_empty());
        self.buf.insert(es.at.row, vec![]);
        let mut b = false;
        let after_pre_inserted = self.insert(
            Cursor {
                row: es.at.row,
                col: 0,
            },
            es.diff_buffer.pre_buf,
            &mut b,
        );
        let after_diff_inserted = self.insert(after_pre_inserted, es.diff_buffer.diff_buf, &mut b);
        self.insert(after_diff_inserted, es.diff_buffer.post_buf, &mut b);
        self.cursor = after_diff_inserted;
        self.visual_cursor = None;
    }

    //
    // effect functions
    //

    pub fn undo(&mut self, _: Key) {
        self._undo();
    }
    pub fn redo(&mut self, _: Key) {
        self._redo();
    }
    pub fn enter_insert_newline(&mut self, _: Key) {
        let row = self.cursor.row;
        let delete_range = CursorRange {
            start: Cursor {
                row: row,
                col: self.buf[row].len() - 1,
            },
            end: Cursor {
                row: row,
                col: self.buf[row].len() - 1,
            },
        };
        self.enter_update_mode(&delete_range, vec![BufElem::Eol]);
    }
    pub fn enter_insert_mode(&mut self, _: Key) {
        assert!(self.edit_state.is_none());
        let delete_range = CursorRange {
            start: self.cursor,
            end: self.cursor,
        };
        self.enter_update_mode(&delete_range, vec![]);
    }
    pub fn enter_change_mode(&mut self, _: Key) {
        if self.visual_range().is_none() {
            return;
        }
        let vr = self.visual_range().unwrap();
        self.enter_update_mode(&vr, vec![]);
    }
    pub fn edit_mode_input(&mut self, k: Key) {
        let es = self.edit_state.as_mut().unwrap();
        let cursor_diff = es.diff_buffer.input(k.clone());

        let es = self.edit_state.clone().unwrap();
        self.buf = es.orig_buf;
        assert!(!es.diff_buffer.is_empty());
        self.buf.insert(es.at.row, vec![]);
        let mut b = false;
        let after_pre_inserted = self.insert(
            Cursor {
                row: es.at.row,
                col: 0,
            },
            es.diff_buffer.pre_buf,
            &mut b,
        );
        let after_diff_inserted = self.insert(after_pre_inserted, es.diff_buffer.diff_buf, &mut b);
        self.insert(after_diff_inserted, es.diff_buffer.post_buf, &mut b);
        self.cursor = after_diff_inserted;
    }
    pub fn leave_edit_mode(&mut self, _: Key) {
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
    }

    pub fn reset(&mut self, _: Key) {
        self.visual_cursor = None
    }
    pub fn delete(&mut self, _: Key) {
        if self.visual_range().is_none() {
            return;
        }
        let vr = self.visual_range().unwrap();
        let (mut pre_survivors, removed, mut post_survivors) = self.prepare_delete(&vr);

        pre_survivors.append(&mut post_survivors);
        if !pre_survivors.is_empty() {
            self.buf.insert(vr.start.row, vec![])
        }
        let mut b = false;
        self.insert(
            Cursor {
                row: vr.start.row,
                col: 0,
            },
            pre_survivors,
            &mut b,
        );

        let log = ChangeLog {
            at: vr.start.clone(),
            deleted: removed,
            inserted: vec![],
        };
        self.change_log_buffer.save(log);

        self.cursor = vr.start;
        self.visual_cursor = None;
    }
    pub fn enter_visual_mode(&mut self, _: Key) {
        self.visual_cursor = Some(self.cursor.clone())
    }
    pub fn cursor_up(&mut self, _: Key) {
        if self.cursor.row > 0 {
            self.cursor.row -= 1;
        }
    }
    pub fn cursor_down(&mut self, _: Key) {
        if self.cursor.row < self.buf.len() - 1 {
            self.cursor.row += 1;
        }
    }
    pub fn cursor_left(&mut self, _: Key) {
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
        }
    }
    pub fn cursor_right(&mut self, _: Key) {
        if self.cursor.col < self.buf[self.cursor.row].len() - 1 {
            self.cursor.col += 1;
        }
    }
    pub fn jump_line_head(&mut self, _: Key) {
        self.cursor.col = 0;
    }
    pub fn jump_line_last(&mut self, _: Key) {
        self.cursor.col = self.buf[self.cursor.row].len() - 1;
    }
    pub fn enter_jump_mode(&mut self, k: Key) {
        self.num_buffer.clear();
        match k {
            Key::Char(c) => self.num_buffer.push(c),
            _ => panic!(),
        }
    }
    pub fn acc_jump_num(&mut self, k: Key) {
        match k {
            Key::Char(c) => self.num_buffer.push(c),
            _ => panic!(),
        }
    }
    pub fn jump(&mut self, _: Key) {
        let mut s = String::new();
        for c in self.num_buffer.clone() {
            s.push(c);
        }
        let n = s.parse::<usize>().unwrap();
        let row = n-1;
        self.cursor.row = row;
        self.cursor.col = 0;
    }
    pub fn cancel_jump(&mut self, _: Key) {
        self.num_buffer.clear();
    }
    pub fn jump_last(&mut self, _: Key) {
        self.cursor.row = self.buf.len() - 1;
        self.cursor.col = 0;
    }
}

use crate::Key;
use std::cell::RefCell;
use std::rc::Rc;

use crate::controller::Effect;

macro_rules! def_effect {
    ($eff_name:ident, $t:ty, $fun_name:ident) => {
        struct $eff_name(Rc<RefCell<$t>>);
        impl Effect for $eff_name {
            fn run(&self, k: Key) {
                self.0.borrow_mut().$fun_name(k);
            }
        }
    };
}
def_effect!(EnterJumpMode, EditBuffer, enter_jump_mode);
def_effect!(AccJumpNum, EditBuffer, acc_jump_num);
def_effect!(Jump, EditBuffer, jump);
def_effect!(CancelJump, EditBuffer, cancel_jump);
def_effect!(JumpLast, EditBuffer, jump_last);

def_effect!(EnterInsertMode, EditBuffer, enter_insert_mode);
def_effect!(EditModeInput, EditBuffer, edit_mode_input);

use crate::controller;
pub fn mk_controller(eb: Rc<RefCell<EditBuffer>>) -> controller::Controller {
    use crate::Key::*;
    let mut g = controller::GraphImpl::new();
    g.add_edge("init", "jump", CharRange('1','9'), Rc::new(EnterJumpMode(eb.clone())));
    g.add_edge("jump", "jump", CharRange('0','9'), Rc::new(AccJumpNum(eb.clone())));
    g.add_edge("jump", "init", Char('G'), Rc::new(Jump(eb.clone())));
    g.add_edge("jump", "init", Esc, Rc::new(CancelJump(eb.clone())));
    g.add_edge("init", "init", Char('G'), Rc::new(JumpLast(eb.clone())));

    controller::Controller {
        cur: "init".to_owned(),
        g: Box::new(g),
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
            filter: VisibilityFilter::new(Cursor { col: 0, row: 0 }),
            old_region: view::ViewRegion {
                col: 0,
                row: 0,
                width: 0,
                height: 0,
            },
        }
    }
}
use crate::view;
impl view::ViewGen for ViewGen {
    fn gen(&mut self, region: view::ViewRegion) -> Box<view::View> {
        let (lineno_reg, buf_reg) = region.split_horizontal(6);

        if self.old_region != region {
            self.filter.resize(region.width, region.height);
            self.old_region = region;
        }
        self.filter.adjust(self.buf.borrow().cursor);
        let max_lineno = std::cmp::min(self.filter.row_high, self.buf.borrow().buf.len() - 1) + 1;
        let lineno_view = view::LineNumber {
            from: self.filter.row_low + 1,
            to: max_lineno,
        };
        let lineno_view =
            view::TranslateView::new(lineno_view, lineno_reg.col as i32, lineno_reg.row as i32);

        let buf_view = view::ToView::new(self.buf.borrow().buf.clone());
        let buf_view = view::OverlayView::new(
            buf_view,
            view::VisualRangeDiffView::new(self.buf.borrow().visual_range()),
        );
        let buf_view = view::AddCursor::new(
            buf_view,
            Some(self.buf.borrow().cursor), // tmp: the cursor is always visible
        );
        let buf_view = view::TranslateView::new(
            buf_view,
            buf_reg.col as i32 - self.filter.col() as i32,
            buf_reg.row as i32 - self.filter.row() as i32,
        );

        let view = view::MergeHorizontal {
            left: lineno_view,
            right: buf_view,
            col_offset: buf_reg.col,
        };

        Box::new(view)
    }
}

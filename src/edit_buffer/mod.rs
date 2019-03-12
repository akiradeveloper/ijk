mod diff_buffer;
mod undo_buffer;
mod indent;

use self::diff_buffer::DiffBuffer;
use self::undo_buffer::UndoBuffer;

use crate::{BufElem, Cursor};
use crate::read_buffer::*;
use crate::navigator;
use std::path;
use std::fs;

#[derive(Copy, Clone)]
pub struct CursorRange {
    pub start: Cursor,
    pub end: Cursor,
}

#[derive(Clone)]
pub struct ChangeLog {
    at: Cursor,
    deleted: Vec<BufElem>,
    inserted: Vec<BufElem>,
}
impl ChangeLog {
    pub fn swap(&self) -> Self {
        Self {
            at: self.at,
            deleted: self.inserted.clone(),
            inserted: self.deleted.clone(),
        }
    }
}

pub struct YankBuffer {
    x: Option<Vec<BufElem>>,
}
impl YankBuffer {
    pub fn new() -> Self {
        Self {
            x: None,
        }
    }
    pub fn push(&mut self, x: Vec<BufElem>) {
        self.x = Some(x)
    }
    pub fn pop(&mut self) -> Option<Vec<BufElem>> {
        self.x.clone()
    }
}

#[derive(PartialEq, Debug)]
enum AffectRange {
    Empty,
    Mid(usize),
    EndEol(usize),
}
fn affect_range_of(buf: &[BufElem]) -> AffectRange {
    if buf.is_empty() {
        return AffectRange::Empty;
    }
    let mut n = 1;
    for e in buf {
        if *e == BufElem::Eol {
            n += 1;
        }
    }
    if *buf.last().unwrap() == BufElem::Eol {
        n -= 1;
        AffectRange::EndEol(n)
    } else {
        AffectRange::Mid(n)
    }
}
#[test]
fn test_affect_range() {
    use self::AffectRange::*;
    use crate::BufElem::*;
    assert_eq!(affect_range_of(&[]), Empty);
    assert_eq!(affect_range_of(&[Char(' ')]), Mid(1));
    assert_eq!(affect_range_of(&[Char(' '),BufElem::Eol]), EndEol(1));
    assert_eq!(affect_range_of(&[Char(' '),BufElem::Eol,Char('a')]), Mid(2));
    assert_eq!(affect_range_of(&[Char(' '),BufElem::Eol,Char('a'),Eol]), EndEol(2));
}
fn calc_n_rows_affected(deleted: &[BufElem], inserted: &[BufElem]) -> (usize, usize) {
    use self::AffectRange::*;
    match (affect_range_of(deleted), affect_range_of(inserted)) {
        (Empty, Empty) => (0, 0),
        (Empty, Mid(n)) => (1, n),
        (Empty, EndEol(n)) => (1, n+1),
        (Mid(n), Empty) => (n, 1),
        (Mid(n), Mid(m)) => (n, m),
        (Mid(n), EndEol(m)) => (n, m+1),
        (EndEol(n), Empty) => (n+1, 1),
        (EndEol(n), Mid(m)) => (n+1, m),
        (EndEol(n), EndEol(m)) => (n+1, m+1),
    }
}

pub struct EditBuffer {
    pub rb: ReadBuffer,
    visual_cursor: Option<Cursor>,
    change_log_buffer: UndoBuffer<ChangeLog>,
    edit_state: Option<EditState>,
    path: Option<path::PathBuf>,
    yank_buffer: YankBuffer,
}

#[derive(Clone)]
struct EditState {
    diff_buffer: DiffBuffer,
    at: Cursor,
    removed: Vec<BufElem>,
    orig_buf: Vec<Vec<BufElem>>,
}

fn convert_to_bufelems(cs: Vec<char>) -> Vec<BufElem> {
    let mut r = vec![];
    for c in cs {
        r.push(BufElem::Char(c));
    }
    r.push(BufElem::Eol);
    r
}

fn read_buffer(path: Option<&path::Path>) -> Vec<Vec<BufElem>> {
    path.and_then(|path|
        fs::read_to_string(path).ok().map(|s| {
            if s.is_empty() {
                vec![vec![BufElem::Eol]]
            } else {
                s.lines().map(|line| convert_to_bufelems(line.chars().collect())).collect()
            }
        })
    ).unwrap_or(vec![vec![BufElem::Eol]])
}

impl EditBuffer {
    pub fn open(path: Option<&path::Path>) -> EditBuffer {
        let init_buf = read_buffer(path);
        EditBuffer {
            rb: ReadBuffer::new(init_buf),
            visual_cursor: None,
            change_log_buffer: UndoBuffer::new(20),
            edit_state: None,
            path: path.map(|x| x.to_owned()),
            yank_buffer: YankBuffer::new(),
        }
    }
    fn apply_log(&mut self, log: &mut ChangeLog) {
        // self.rb.search.update(&log);

        let delete_range = CursorRange {
            start: log.at,
            end: self.find_cursor_pair(log.at, log.deleted.len()),
        };
        let (mut pre_survivors, _, mut post_survivors) = self.prepare_delete(&delete_range);
        pre_survivors.append(&mut log.inserted);
        pre_survivors.append(&mut post_survivors);
        if !pre_survivors.is_empty() {
            self.rb.buf.insert(log.at.row, vec![])
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
        self.rb.clear_search_struct(); // tmp
    }
    fn undo(&mut self) -> bool {
        let log = self.change_log_buffer.pop_undo();
        if log.is_none() {
            return false;
        }
        let mut log = log.unwrap().swap();
        self.apply_log(&mut log);
        self.rb.cursor = log.at;
        true
    }
    fn redo(&mut self) -> bool {
        let log = self.change_log_buffer.pop_redo();
        if log.is_none() {
            return false;
        }
        let mut log = log.unwrap();
        let n_inserted = log.inserted.len();
        self.apply_log(&mut log);
        self.rb.cursor = self.find_cursor_pair(log.at, n_inserted);
        true
    }
    fn to_cursor_range_end(cursor: Cursor) -> Cursor {
        Cursor {
            row: cursor.row,
            col: cursor.col + 1,
        }
    }
    pub fn visual_range(&self) -> Option<CursorRange> {
        self.visual_cursor.clone().map(|vc| {
            if self.rb.cursor > vc {
                CursorRange {
                    start: vc,
                    end: EditBuffer::to_cursor_range_end(self.rb.cursor),
                }
            } else {
                CursorRange {
                    start: self.rb.cursor.clone(),
                    end: EditBuffer::to_cursor_range_end(vc),
                }
            }
        })
    }
    fn expand_range(&self, r: &CursorRange) -> Vec<(usize, std::ops::Range<usize>)> {
        let mut res = vec![];
        for row in r.start.row..r.end.row + 1 {
            let col_start = if row == r.start.row { r.start.col } else { 0 };
            let col_end = if row == r.end.row {
                r.end.col
            } else {
                self.rb.buf[row].len()
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
                self.rb.buf.insert(row, vec![]);
                *should_insert_newline = false;
            }
            match e {
                x @ BufElem::Eol => {
                    self.rb.buf[row].insert(col, x);
                    *should_insert_newline = true;
                    row += 1;
                    col = 0;
                }
                x @ BufElem::Char(_) => {
                    self.rb.buf[row].insert(col, x);
                    col += 1;
                }
            }
        }
        Cursor { row: row, col: col }
    }
    fn find_cursor_pair(&self, cursor: Cursor, len: usize) -> Cursor {
        let mut row = cursor.row;
        let mut col = cursor.col;
        let mut remaining = len;
        while remaining > 0 {
            let n = std::cmp::min(remaining, self.rb.buf[row].len() - col);
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
        let target_region = if range.end.col == self.rb.buf[range.end.row].len()
            && range.end.row != self.rb.buf.len() - 1
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
            for col in 0..self.rb.buf[row].len() {
                if col_range.start <= col && col < col_range.end {
                    removed.push(self.rb.buf[row][col].clone())
                } else {
                    let as_cursor = Cursor { row, col };
                    if as_cursor < range.start {
                        pre_survivors.push(self.rb.buf[row][col].clone())
                    } else {
                        post_survivors.push(self.rb.buf[row][col].clone())
                    }
                }
            }
        }
        for (row, _) in target_region.into_iter().rev() {
            self.rb.buf.remove(row);
        }

        (pre_survivors, removed, post_survivors)
    }
    fn enter_update_mode(&mut self, r: &CursorRange, init_diff: Vec<BufElem>) {
        let (pre_survivors, removed, post_survivors) = self.prepare_delete(&r);
        let orig_buf = self.rb.buf.clone();
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
        self.rb.buf.insert(es.at.row, vec![]);
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
        self.rb.clear_search_struct();
        self.rb.cursor = after_diff_inserted;
        self.visual_cursor = None;
    }

    //
    // effect functions
    //

    pub fn eff_undo(&mut self, _: Key) {
        self.undo();
    }
    pub fn eff_redo(&mut self, _: Key) {
        self.redo();
    }
    pub fn eff_enter_insert_newline(&mut self, _: Key) {
        let row = self.rb.cursor.row;
        let delete_range = CursorRange {
            start: Cursor {
                row: row,
                col: self.rb.buf[row].len() - 1,
            },
            end: Cursor {
                row: row,
                col: self.rb.buf[row].len() - 1,
            },
        };
        let auto_indent = indent::AutoIndent {
            line_predecessors: &self.rb.buf[row][0..self.rb.buf[row].len()-1]
        };
        let mut v = vec![BufElem::Eol];
        v.append(&mut auto_indent.next_indent());
        self.enter_update_mode(&delete_range, v);
    }
    pub fn eff_join_next_line(&mut self, _: Key) {
        let row = self.rb.cursor.row;
        if row == self.rb.buf.len() - 1 {
            return;
        }
        let mut next_line_first_nonspace_pos = 0;
        for x in &self.rb.buf[row+1] {
            match *x {
                BufElem::Char(' ') | BufElem::Char('\t') => next_line_first_nonspace_pos += 1,
                _ => break
            }
        }
        let delete_range = CursorRange {
            start: Cursor {
                row: row,
                col: self.rb.buf[row].len() - 1,
            },
            end: Cursor {
                row: row + 1,
                col: next_line_first_nonspace_pos,
            }
        };
        self.enter_update_mode(&delete_range, vec![]);
    }
    pub fn eff_enter_insert_mode(&mut self, _: Key) {
        assert!(self.edit_state.is_none());
        let delete_range = CursorRange {
            start: self.rb.cursor,
            end: self.rb.cursor,
        };
        self.enter_update_mode(&delete_range, vec![]);
    }
    pub fn eff_enter_append_mode(&mut self, k: Key) {
        self.eff_cursor_right(k.clone());
        self.eff_enter_insert_mode(k);
    }
    pub fn eff_enter_change_mode(&mut self, _: Key) {
        if self.visual_range().is_none() {
            return;
        }
        let vr = self.visual_range().unwrap();
        self.enter_update_mode(&vr, vec![]);
    }
    pub fn eff_edit_mode_input(&mut self, k: Key) {
        let es = self.edit_state.as_mut().unwrap();
        es.diff_buffer.input(k.clone());

        let es = self.edit_state.clone().unwrap();
        self.rb.buf = es.orig_buf;
        self.rb.buf.insert(es.at.row, vec![]);
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
        self.rb.clear_search_struct(); // tmp (too slow)
        self.rb.cursor = after_diff_inserted;
    }
    pub fn eff_leave_edit_mode(&mut self, _: Key) {
        assert!(self.edit_state.is_some());
        // take(): replace the memory region with None and take out the owrnership of the object
        let edit_state = self.edit_state.take().unwrap();
        assert!(self.edit_state.is_none());
        let change_log = ChangeLog {
            at: edit_state.at,
            deleted: edit_state.removed,
            inserted: edit_state.diff_buffer.diff_buf,
        };
        // self.rb.search.update(&change_log);
        self.rb.clear_search_struct(); // tmp
        if change_log.deleted.len() > 0 || change_log.inserted.len() > 0 {
            self.change_log_buffer.push(change_log);
        }
    }
    fn delete_range(&mut self, range: CursorRange) {
        let (mut pre_survivors, removed, mut post_survivors) = self.prepare_delete(&range);

        pre_survivors.append(&mut post_survivors);
        if !pre_survivors.is_empty() {
            self.rb.buf.insert(range.start.row, vec![])
        }
        let mut b = false;
        self.insert(
            Cursor {
                row: range.start.row,
                col: 0,
            },
            pre_survivors,
            &mut b,
        );

        let log = ChangeLog {
            at: range.start.clone(),
            deleted: removed,
            inserted: vec![],
        };
        // self.rb.search.update(&log);
        self.change_log_buffer.push(log);
        self.rb.clear_search_struct(); // tmp

        self.rb.cursor = range.start;
        // this ensures visual mode is cancelled whenever it starts insertion mode.
        self.visual_cursor = None;
    }
    pub fn eff_delete_line(&mut self, _: Key) {
        if self.visual_range().is_none() {
            self.visual_cursor = Some(Cursor {
                row: self.rb.cursor.row,
                col: 0,
            });
            self.rb.jump_line_last();
            self.delete_range(self.visual_range().unwrap());
        } else {
            let vr = self.visual_range().unwrap();
            self.rb.cursor = Cursor {
                row: vr.end.row,
                col: 0,
            };
            self.visual_cursor = Some(Cursor {
                row: vr.start.row,
                col: 0,
            });
            self.rb.jump_line_last();
            self.delete_range(self.visual_range().unwrap());
        }
    }
    pub fn eff_delete_char(&mut self, _: Key) {
        let range = self.visual_range().unwrap_or(
            CursorRange {
                start: self.rb.cursor,
                end: Cursor {
                    row: self.rb.cursor.row,
                    col: self.rb.cursor.col + 1,
                },
            }
        );
        self.delete_range(range);
    }
    pub fn eff_paste(&mut self, _: Key) {
        let yb = self.yank_buffer.pop();
        if yb.is_none() { return; }

        let yb = yb.unwrap();
        let mut log = ChangeLog {
            at: self.rb.cursor,
            deleted: vec![],
            inserted: yb,
        };
        self.change_log_buffer.push(log.clone());
        self.apply_log(&mut log);
    }
    pub fn eff_yank(&mut self, _: Key) {
        let orig_cursor = self.rb.cursor;
        let vr = self.visual_range();
        if vr.is_none() { return; }

        let vr = vr.unwrap();
        self.delete_range(vr);
        let yb = self.change_log_buffer.peek().cloned().unwrap().deleted;
        self.undo();
        self.yank_buffer.push(yb);
        self.rb.cursor = orig_cursor;
    }
    fn indent_back_line(&mut self, row: usize, indent: &[BufElem]) {
        let mut cnt = 0;
        for i in 0 .. indent.len() {
            if self.rb.buf[row][i] != indent[i] {
                break;
            }
            cnt += 1;
        }
        self.delete_range(CursorRange{
            start: Cursor { row: row, col: 0 },
            end: Cursor { row: row, col: cnt }
        })
    }
    fn indent_back_range(&mut self, row_range: std::ops::Range<usize>) {
        for row in row_range {
            self.indent_back_line(row, &vec![BufElem::Char(' '); 4]);
        }
    }
    pub fn eff_indent_back(&mut self, _: Key) {
        if self.visual_range().is_none() {
            self.indent_back_range(self.rb.cursor.row .. self.rb.cursor.row+1);
            return;
        }
        let vr = self.visual_range().unwrap();
        self.indent_back_range(vr.start.row .. vr.end.row+1);
        self.visual_cursor = None;
        // TODO atomic change log
    }
    pub fn eff_enter_visual_mode(&mut self, _: Key) {
        self.visual_cursor = Some(self.rb.cursor.clone());
    }
    pub fn eff_reset(&mut self, _: Key) {
        self.visual_cursor = None;
    }
    pub fn eff_save_to_file(&self, _: Key) {
        use std::io::Write;
        if self.path.is_none() {
            return;
        }
        let path = self.path.clone().unwrap();
        if let Ok(mut file) = fs::File::create(path) {
            let buf = &self.rb.buf;
            for i in 0..buf.len() {
                for j in 0..buf[i].len() {
                    let e = &buf[i][j];
                    match *e {
                        BufElem::Char(c) => write!(file, "{}", c).unwrap(),
                        BufElem::Eol => writeln!(file).unwrap(),
                    }
                }
            }
        }
    }
    pub fn eff_cursor_up(&mut self, _: Key) {
        self.rb.cursor_up();
    }
    pub fn eff_cursor_down(&mut self, _: Key) {
        self.rb.cursor_down();
    }
    pub fn eff_cursor_left(&mut self, _: Key) {
        self.rb.cursor_left();
    }
    pub fn eff_cursor_right(&mut self, _: Key) {
        self.rb.cursor_right();
    }
    pub fn eff_jump_line_head(&mut self, _: Key) {
        self.rb.jump_line_head();
    }
    pub fn eff_jump_line_last(&mut self, _: Key) {
        self.rb.jump_line_last();
    }
    pub fn eff_jump_page_forward(&mut self, _: Key) {
        self.rb.jump_page_forward();
    }
    pub fn eff_jump_page_backward(&mut self, _: Key) {
        self.rb.jump_page_backward();
    }
    pub fn eff_enter_jump_mode(&mut self, k: Key) {
        self.rb.enter_jump_mode(k);
    }
    pub fn eff_acc_jump_num(&mut self, k: Key) {
        self.rb.acc_jump_num(k);
    }
    pub fn eff_jump(&mut self, _: Key) {
        self.rb.jump();
    }
    pub fn eff_cancel_jump(&mut self, _: Key) {
        self.rb.cancel_jump();
    }
    pub fn eff_jump_last(&mut self, _: Key) {
        self.rb.jump_last();
    }
    pub fn eff_enter_search_mode(&mut self, _: Key) {
        self.rb.enter_search_mode();
    }
    pub fn eff_search_mode_input(&mut self, k: Key) {
        self.rb.search_mode_input(k);
    }
    pub fn eff_leave_search_mode(&mut self, _: Key) {
        self.rb.leave_search_mode();
    }
    pub fn eff_search_jump_forward(&mut self, _: Key) {
        self.rb.search_jump_forward();
    }
    pub fn eff_search_jump_backward(&mut self, _: Key) {
        self.rb.search_jump_backward();
    }
}

use crate::Key;
use std::cell::RefCell;
use std::rc::Rc;

use crate::controller::{Effect};
use crate::def_effect;

def_effect!(Undo, EditBuffer, eff_undo);
def_effect!(Redo, EditBuffer, eff_redo);
def_effect!(JoinNextLine, EditBuffer, eff_join_next_line);
def_effect!(EnterInsertNewline, EditBuffer, eff_enter_insert_newline);
def_effect!(EnterInsertMode, EditBuffer, eff_enter_insert_mode);
def_effect!(EnterAppendMode, EditBuffer, eff_enter_append_mode);
def_effect!(EnterChangeMode, EditBuffer, eff_enter_change_mode);
def_effect!(EditModeInput, EditBuffer, eff_edit_mode_input);
def_effect!(LeaveEditMode, EditBuffer, eff_leave_edit_mode);
def_effect!(DeleteLine, EditBuffer, eff_delete_line);
def_effect!(DeleteChar, EditBuffer, eff_delete_char);
def_effect!(Paste, EditBuffer, eff_paste);
def_effect!(Yank, EditBuffer, eff_yank);
def_effect!(IndentBack, EditBuffer, eff_indent_back);
def_effect!(EnterVisualMode, EditBuffer, eff_enter_visual_mode);
def_effect!(SaveToFile, EditBuffer, eff_save_to_file);
def_effect!(Reset, EditBuffer, eff_reset);

def_effect!(CursorUp, EditBuffer, eff_cursor_up);
def_effect!(CursorDown, EditBuffer, eff_cursor_down);
def_effect!(CursorLeft, EditBuffer, eff_cursor_left);
def_effect!(CursorRight, EditBuffer, eff_cursor_right);
def_effect!(JumpLineHead, EditBuffer, eff_jump_line_head);
def_effect!(JumpLineLast, EditBuffer, eff_jump_line_last);
def_effect!(JumpPageForward, EditBuffer, eff_jump_page_forward);
def_effect!(JumpPageBackward, EditBuffer, eff_jump_page_backward);
def_effect!(EnterJumpMode, EditBuffer, eff_enter_jump_mode);
def_effect!(AccJumpNum, EditBuffer, eff_acc_jump_num);
def_effect!(Jump, EditBuffer, eff_jump);
def_effect!(CancelJump, EditBuffer, eff_cancel_jump);
def_effect!(JumpLast, EditBuffer, eff_jump_last);

def_effect!(EnterSearchMode, EditBuffer, eff_enter_search_mode);
def_effect!(SearchModeInput, EditBuffer, eff_search_mode_input);
def_effect!(LeaveSearchMode, EditBuffer, eff_leave_search_mode);
def_effect!(SearchJumpForward, EditBuffer, eff_search_jump_forward);
def_effect!(SearchJumpBackward, EditBuffer, eff_search_jump_backward);

use crate::controller;
pub fn mk_controller(x: Rc<RefCell<EditBuffer>>) -> controller::ControllerFSM {
    use crate::Key::*;
    let mut g = controller::GraphImpl::new();

    // mutable
    g.add_edge("init", "init", Ctrl('s'), Rc::new(SaveToFile(x.clone())));
    g.add_edge("init", "init", Char('v'), Rc::new(EnterVisualMode(x.clone())));
    g.add_edge("init", "init", Esc, Rc::new(Reset(x.clone())));
    g.add_edge("init", "init", Char('d'), Rc::new(DeleteLine(x.clone())));
    g.add_edge("init", "init", Char('x'), Rc::new(DeleteChar(x.clone())));
    g.add_edge("init", "init", Char('<'), Rc::new(IndentBack(x.clone())));
    g.add_edge("init", "insert", Char('J'), Rc::new(JoinNextLine(x.clone())));
    g.add_edge("init", "insert", Char('o'), Rc::new(EnterInsertNewline(x.clone())));
    g.add_edge("init", "insert", Char('i'), Rc::new(EnterInsertMode(x.clone())));
    g.add_edge("init", "insert", Char('a'), Rc::new(EnterAppendMode(x.clone())));
    g.add_edge("init", "insert", Char('c'), Rc::new(EnterChangeMode(x.clone())));
    g.add_edge("init", "init", Char('p'), Rc::new(Paste(x.clone())));
    g.add_edge("init", "init", Char('y'), Rc::new(Yank(x.clone())));
    g.add_edge("insert", "init", Esc, Rc::new(LeaveEditMode(x.clone())));
    g.add_edge("insert", "insert", Otherwise, Rc::new(EditModeInput(x.clone())));

    g.add_edge("init", "init", Ctrl('r'), Rc::new(Redo(x.clone())));
    g.add_edge("init", "init", Char('u'), Rc::new(Undo(x.clone())));

    // immutable
    g.add_edge("init", "init", Char('k'), Rc::new(CursorUp(x.clone())));
    g.add_edge("init", "init", Char('j'), Rc::new(CursorDown(x.clone())));
    g.add_edge("init", "init", Char('h'), Rc::new(CursorLeft(x.clone())));
    g.add_edge("init", "init", Char('l'), Rc::new(CursorRight(x.clone())));
    g.add_edge("init", "init", Char('0'), Rc::new(JumpLineHead(x.clone())));
    g.add_edge("init", "init", Char('$'), Rc::new(JumpLineLast(x.clone())));
    g.add_edge("init", "init", Ctrl('f'), Rc::new(JumpPageForward(x.clone())));
    g.add_edge("init", "init", Ctrl('b'), Rc::new(JumpPageBackward(x.clone())));
    g.add_edge("init", "jump", CharRange('1','9'), Rc::new(EnterJumpMode(x.clone())));
    g.add_edge("jump", "jump", CharRange('0','9'), Rc::new(AccJumpNum(x.clone())));
    g.add_edge("jump", "init", Char('G'), Rc::new(Jump(x.clone())));
    g.add_edge("jump", "init", Esc, Rc::new(CancelJump(x.clone())));
    g.add_edge("init", "init", Char('G'), Rc::new(JumpLast(x.clone())));

    // search
    g.add_edge("init", "search", Char('/'), Rc::new(EnterSearchMode(x.clone())));
    g.add_edge("search", "init", Char('\n'), Rc::new(LeaveSearchMode(x.clone())));
    g.add_edge("search", "search", Otherwise, Rc::new(SearchModeInput(x.clone())));
    g.add_edge("init", "init", Char('n'), Rc::new(SearchJumpForward(x.clone())));
    g.add_edge("init", "init", Char('N'), Rc::new(SearchJumpBackward(x.clone())));

    controller::ControllerFSM {
        cur: "init".to_owned(),
        g: Box::new(g),
    }
}

pub struct ViewGen {
    buf: Rc<RefCell<EditBuffer>>,
}
impl ViewGen {
    pub fn new(buf: Rc<RefCell<EditBuffer>>) -> Self {
        Self {
            buf: buf,
        }
    }
}
use crate::view;
impl view::ViewGen for ViewGen {
    fn gen(&mut self, region: view::Area) -> Box<view::View> {
        let (edit_reg, search_reg) = region.split_vertical(region.height-1);
        let (lineno_reg, buf_reg) = edit_reg.split_horizontal(6);

        self.buf.borrow_mut().rb.stabilize();
        self.buf.borrow_mut().rb.adjust_window(buf_reg.width, buf_reg.height);
        self.buf.borrow_mut().rb.update_search_results();

        let lineno_range = self.buf.borrow().rb.lineno_range();
        let lineno_view = view::LineNumber {
            from: lineno_range.start+1 ,
            to: lineno_range.end ,
        };
        let lineno_view =
            view::TranslateView::new(lineno_view, lineno_reg.col as i32, lineno_reg.row as i32);

        let buf_view = view::ToView::new(self.buf.borrow().rb.buf.clone());
        let buf_view = view::OverlayView::new(
            buf_view,
            search::DiffView::new(self.buf.borrow().rb.search.clone()),
        );
        let buf_view = view::OverlayView::new(
            buf_view,
            view::VisualRangeDiffView::new(self.buf.borrow().visual_range()),
        );
        let buf_view = view::AddCursor::new(
            buf_view,
            Some(self.buf.borrow().rb.cursor), // tmp: the cursor is always visible
        );
        let buf_view = view::TranslateView::new(
            buf_view,
            buf_reg.col as i32 - self.buf.borrow().rb.window.col() as i32,
            buf_reg.row as i32 - self.buf.borrow().rb.window.row() as i32,
        );

        let view = view::MergeHorizontal {
            left: lineno_view,
            right: buf_view,
            col_offset: buf_reg.col,
        };

        let search_bar = view::SearchBar::new(&self.buf.borrow().rb.current_search_word());
        let search_bar = view::TranslateView::new(
            search_bar,
            search_reg.col as i32,
            search_reg.row as i32,
        );
        let view = view::MergeVertical {
            top: view,
            bottom: search_bar,
            row_offset: search_reg.row,
        };

        Box::new(view)
    }
}

pub struct Page {
    controller: Rc<RefCell<controller::Controller>>,
    view_gen: Rc<RefCell<view::ViewGen>>,
    x: Rc<RefCell<EditBuffer>>,
}
impl Page {
    pub fn new(x: Rc<RefCell<EditBuffer>>) -> Self {
        Self {
            controller: Rc::new(RefCell::new(mk_controller(x.clone()))),
            view_gen: Rc::new(RefCell::new(ViewGen::new(x.clone()))),
            x: x,
        }
    }
}
impl navigator::Page for Page {
    fn controller(&self) -> Rc<RefCell<controller::Controller>> {
        self.controller.clone()
    }
    fn view_gen(&self) -> Rc<RefCell<view::ViewGen>> {
        self.view_gen.clone()
    }
    fn desc(&self) -> String {
        let s = match self.x.borrow().path.clone() {
            Some(p) => p.to_str().unwrap().to_owned(),
            None => "null".to_owned(),
        };
        format!("[BUFFER] {}", s)
    }
    fn kind(&self) -> navigator::PageKind {
        navigator::PageKind::Buffer
    }
    fn id(&self) -> String {
        // "aaa".to_owned()
        match self.x.borrow().path.clone() {
            Some(p) => p.to_str().unwrap().to_owned(),
            None => "noname".to_owned(),
        }
    }
}
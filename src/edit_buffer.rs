use crate::diff_buffer::DiffBuffer;
use crate::undo_buffer::UndoBuffer;
use crate::{BufElem, Cursor, ChangeLog};
use crate::read_buffer::*;
use crate::search;
use crate::navigator;
use std::path;
use std::fs;
use crate::indent;

#[derive(Copy, Clone)]
pub struct CursorRange {
    pub start: Cursor,
    pub end: Cursor,
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
    fn _undo(&mut self) -> bool {
        let log = self.change_log_buffer.pop_undo();
        if log.is_none() {
            return false;
        }
        let mut log = log.unwrap().swap();
        self.apply_log(&mut log);
        self.rb.cursor = log.at;
        true
    }
    fn _redo(&mut self) -> bool {
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

    pub fn undo(&mut self, _: Key) {
        self._undo();
    }
    pub fn redo(&mut self, _: Key) {
        self._redo();
    }
    pub fn enter_insert_newline(&mut self, _: Key) {
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
    pub fn join_next_line(&mut self, _: Key) {
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
    pub fn enter_insert_mode(&mut self, _: Key) {
        assert!(self.edit_state.is_none());
        let delete_range = CursorRange {
            start: self.rb.cursor,
            end: self.rb.cursor,
        };
        self.enter_update_mode(&delete_range, vec![]);
    }
    pub fn enter_append_mode(&mut self, k: Key) {
        self.cursor_right(k.clone());
        self.enter_insert_mode(k);
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
    pub fn delete_line(&mut self, _: Key) {
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
    pub fn delete_char(&mut self, _: Key) {
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
    pub fn paste(&mut self, _: Key) {
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
    pub fn yank(&mut self, _: Key) {
        let orig_cursor = self.rb.cursor;
        let vr = self.visual_range();
        if vr.is_none() { return; }

        let vr = vr.unwrap();
        self.delete_range(vr);
        let yb = self.change_log_buffer.peek().cloned().unwrap().deleted;
        self._undo();
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
    pub fn indent_back(&mut self, _: Key) {
        if self.visual_range().is_none() {
            self.indent_back_range(self.rb.cursor.row .. self.rb.cursor.row+1);
            return;
        }
        let vr = self.visual_range().unwrap();
        self.indent_back_range(vr.start.row .. vr.end.row+1);
        self.visual_cursor = None;
        // TODO atomic change log
    }
    pub fn enter_visual_mode(&mut self, _: Key) {
        self.visual_cursor = Some(self.rb.cursor.clone());
    }
    pub fn reset(&mut self, _: Key) {
        self.visual_cursor = None;
    }
    pub fn save_to_file(&self, _: Key) {
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
    pub fn cursor_up(&mut self, k: Key) {
        self.rb.cursor_up();
    }
    pub fn cursor_down(&mut self, k: Key) {
        self.rb.cursor_down();
    }
    pub fn cursor_left(&mut self, k: Key) {
        self.rb.cursor_left();
    }
    pub fn cursor_right(&mut self, k: Key) {
        self.rb.cursor_right();
    }
    pub fn jump_line_head(&mut self, k: Key) {
        self.rb.jump_line_head();
    }
    pub fn jump_line_last(&mut self, k: Key) {
        self.rb.jump_line_last();
    }
    pub fn jump_page_forward(&mut self, k: Key) {
        self.rb.jump_page_forward();
    }
    pub fn jump_page_backward(&mut self, k: Key) {
        self.rb.jump_page_backward();
    }
    pub fn enter_jump_mode(&mut self, k: Key) {
        self.rb.enter_jump_mode(k);
    }
    pub fn acc_jump_num(&mut self, k: Key) {
        self.rb.acc_jump_num(k);
    }
    pub fn jump(&mut self, k: Key) {
        self.rb.jump();
    }
    pub fn cancel_jump(&mut self, k: Key) {
        self.rb.cancel_jump();
    }
    pub fn jump_last(&mut self, k: Key) {
        self.rb.jump_last();
    }
    pub fn enter_search_mode(&mut self, k: Key) {
        self.rb.enter_search_mode();
    }
    pub fn search_mode_input(&mut self, k: Key) {
        self.rb.search_mode_input(k);
    }
    pub fn leave_search_mode(&mut self, k: Key) {
        self.rb.leave_search_mode();
    }
    pub fn search_jump_forward(&mut self, k: Key) {
        self.rb.search_jump_forward();
    }
    pub fn search_jump_backward(&mut self, k: Key) {
        self.rb.search_jump_backward();
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
def_effect!(Undo, EditBuffer, undo);
def_effect!(Redo, EditBuffer, redo);
def_effect!(JoinNextLine, EditBuffer, join_next_line);
def_effect!(EnterInsertNewline, EditBuffer, enter_insert_newline);
def_effect!(EnterInsertMode, EditBuffer, enter_insert_mode);
def_effect!(EnterAppendMode, EditBuffer, enter_append_mode);
def_effect!(EnterChangeMode, EditBuffer, enter_change_mode);
def_effect!(EditModeInput, EditBuffer, edit_mode_input);
def_effect!(LeaveEditMode, EditBuffer, leave_edit_mode);
def_effect!(DeleteLine, EditBuffer, delete_line);
def_effect!(DeleteChar, EditBuffer, delete_char);
def_effect!(Paste, EditBuffer, paste);
def_effect!(Yank, EditBuffer, yank);
def_effect!(IndentBack, EditBuffer, indent_back);
def_effect!(EnterVisualMode, EditBuffer, enter_visual_mode);
def_effect!(SaveToFile, EditBuffer, save_to_file);
def_effect!(Reset, EditBuffer, reset);

def_effect!(CursorUp, EditBuffer, cursor_up);
def_effect!(CursorDown, EditBuffer, cursor_down);
def_effect!(CursorLeft, EditBuffer, cursor_left);
def_effect!(CursorRight, EditBuffer, cursor_right);
def_effect!(JumpLineHead, EditBuffer, jump_line_head);
def_effect!(JumpLineLast, EditBuffer, jump_line_last);
def_effect!(JumpPageForward, EditBuffer, jump_page_forward);
def_effect!(JumpPageBackward, EditBuffer, jump_page_backward);
def_effect!(EnterJumpMode, EditBuffer, enter_jump_mode);
def_effect!(AccJumpNum, EditBuffer, acc_jump_num);
def_effect!(Jump, EditBuffer, jump);
def_effect!(CancelJump, EditBuffer, cancel_jump);
def_effect!(JumpLast, EditBuffer, jump_last);

def_effect!(EnterSearchMode, EditBuffer, enter_search_mode);
def_effect!(SearchModeInput, EditBuffer, search_mode_input);
def_effect!(LeaveSearchMode, EditBuffer, leave_search_mode);
def_effect!(SearchJumpForward, EditBuffer, search_jump_forward);
def_effect!(SearchJumpBackward, EditBuffer, search_jump_backward);

use crate::controller;
pub fn mk_controller(eb: Rc<RefCell<EditBuffer>>) -> controller::ControllerFSM {
    use crate::Key::*;
    let mut g = controller::GraphImpl::new();

    // mutable
    g.add_edge("init", "init", Ctrl('s'), Rc::new(SaveToFile(eb.clone())));
    g.add_edge("init", "init", Char('v'), Rc::new(EnterVisualMode(eb.clone())));
    g.add_edge("init", "init", Esc, Rc::new(Reset(eb.clone())));
    g.add_edge("init", "init", Char('d'), Rc::new(DeleteLine(eb.clone())));
    g.add_edge("init", "init", Char('x'), Rc::new(DeleteChar(eb.clone())));
    g.add_edge("init", "init", Char('<'), Rc::new(IndentBack(eb.clone())));
    g.add_edge("init", "insert", Char('J'), Rc::new(JoinNextLine(eb.clone())));
    g.add_edge("init", "insert", Char('o'), Rc::new(EnterInsertNewline(eb.clone())));
    g.add_edge("init", "insert", Char('i'), Rc::new(EnterInsertMode(eb.clone())));
    g.add_edge("init", "insert", Char('a'), Rc::new(EnterAppendMode(eb.clone())));
    g.add_edge("init", "insert", Char('c'), Rc::new(EnterChangeMode(eb.clone())));
    g.add_edge("init", "init", Char('p'), Rc::new(Paste(eb.clone())));
    g.add_edge("init", "init", Char('y'), Rc::new(Yank(eb.clone())));
    g.add_edge("insert", "init", Esc, Rc::new(LeaveEditMode(eb.clone())));
    g.add_edge("insert", "insert", Otherwise, Rc::new(EditModeInput(eb.clone())));

    g.add_edge("init", "init", Ctrl('r'), Rc::new(Redo(eb.clone())));
    g.add_edge("init", "init", Char('u'), Rc::new(Undo(eb.clone())));

    // immutable
    g.add_edge("init", "init", Char('k'), Rc::new(CursorUp(eb.clone())));
    g.add_edge("init", "init", Char('j'), Rc::new(CursorDown(eb.clone())));
    g.add_edge("init", "init", Char('h'), Rc::new(CursorLeft(eb.clone())));
    g.add_edge("init", "init", Char('l'), Rc::new(CursorRight(eb.clone())));
    g.add_edge("init", "init", Char('0'), Rc::new(JumpLineHead(eb.clone())));
    g.add_edge("init", "init", Char('$'), Rc::new(JumpLineLast(eb.clone())));
    g.add_edge("init", "init", Ctrl('f'), Rc::new(JumpPageForward(eb.clone())));
    g.add_edge("init", "init", Ctrl('b'), Rc::new(JumpPageBackward(eb.clone())));
    g.add_edge("init", "jump", CharRange('1','9'), Rc::new(EnterJumpMode(eb.clone())));
    g.add_edge("jump", "jump", CharRange('0','9'), Rc::new(AccJumpNum(eb.clone())));
    g.add_edge("jump", "init", Char('G'), Rc::new(Jump(eb.clone())));
    g.add_edge("jump", "init", Esc, Rc::new(CancelJump(eb.clone())));
    g.add_edge("init", "init", Char('G'), Rc::new(JumpLast(eb.clone())));

    // search
    g.add_edge("init", "search", Char('/'), Rc::new(EnterSearchMode(eb.clone())));
    g.add_edge("search", "init", Char('\n'), Rc::new(LeaveSearchMode(eb.clone())));
    g.add_edge("search", "search", Otherwise, Rc::new(SearchModeInput(eb.clone())));
    g.add_edge("init", "init", Char('n'), Rc::new(SearchJumpForward(eb.clone())));
    g.add_edge("init", "init", Char('N'), Rc::new(SearchJumpBackward(eb.clone())));

    controller::ControllerFSM {
        cur: "init".to_owned(),
        g: Box::new(g),
    }
}

use crate::visibility_filter::VisibilityFilter;
pub struct ViewGen {
    buf: Rc<RefCell<EditBuffer>>,
    old_region: view::ViewRegion,
}
impl ViewGen {
    pub fn new(buf: Rc<RefCell<EditBuffer>>) -> Self {
        Self {
            buf: buf,
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
        let (edit_reg, search_reg) = region.split_vertical(region.height-1);
        let (lineno_reg, buf_reg) = edit_reg.split_horizontal(6);

        self.buf.borrow_mut().rb.stabilize();
        if self.old_region != region {
            self.buf.borrow_mut().rb.resize_window(region.width - 6, region.height - 1);
            self.old_region = region;
        }
        self.buf.borrow_mut().rb.adjust_window();
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
            buf_reg.col as i32 - self.buf.borrow().rb.filter.col() as i32,
            buf_reg.row as i32 - self.buf.borrow().rb.filter.row() as i32,
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
    eb: Rc<RefCell<EditBuffer>>,
}
impl Page {
    pub fn new(eb: Rc<RefCell<EditBuffer>>) -> Self {
        Self {
            controller: Rc::new(RefCell::new(mk_controller(eb.clone()))),
            view_gen: Rc::new(RefCell::new(ViewGen::new(eb.clone()))),
            eb: eb,
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
        "tmp".to_owned()
    }
}
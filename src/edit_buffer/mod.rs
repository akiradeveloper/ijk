pub mod change_log;
pub mod clipboard;
pub mod diff_buffer;
pub mod highlight;
pub mod indent;
pub mod undo_buffer;

use self::change_log::{ChangeLog, ChangeLogBuffer};
use self::diff_buffer::DiffBuffer;

use crate::message_box::MessageBox;
use crate::navigator;
use crate::read_buffer::*;
use crate::screen;
use crate::{BufElem, Cursor};
use std::fs;
use std::path;
use std::time::Instant;

const INIT: &str = "Normal";
const REPLACE_ONCE: &str = "ReplaceOnce";
const WARP: &str = "Warp";
const LINES: &str = "Lines";
const INSERT: &str = "Insert";
const SEARCH: &str = "Search";
const JUMP: &str = "Jump";

#[derive(Copy, Clone, Debug)]
pub struct CursorRange {
    pub start: Cursor,
    pub end: Cursor,
}

fn to_cursor_range_end(cursor: Cursor) -> Cursor {
    Cursor {
        row: cursor.row,
        col: cursor.col + 1,
    }
}
pub struct EditBuffer {
    pub rb: ReadBuffer,
    state: String,
    visual_cursor: Option<Cursor>,
    change_log_buffer: ChangeLogBuffer,
    edit_state: Option<EditState>,
    path: Option<path::PathBuf>,
    sync_clock: Option<Instant>,
    highlighter: highlight::Highlighter,
    message_box: MessageBox,
}

#[derive(Clone)]
struct EditState {
    diff_buffer: DiffBuffer,
    at: Cursor,
    removed: Vec<BufElem>,
    orig_buf: Vec<Vec<BufElem>>,
}

fn trim_right(xs: Vec<BufElem>) -> Vec<BufElem> {
    let mut v = xs;
    if v.is_empty() {
        v
    } else if v[v.len()-1] == BufElem::Eol {
        v.pop();
        v
    } else {
        v
    }
}

pub fn read_buffer(path: Option<&path::Path>) -> Vec<Vec<BufElem>> {
    let s = path.and_then(|path| fs::read_to_string(path).ok());
    crate::normalize::read_from_string(s)
}

impl EditBuffer {
    pub fn open(path: Option<&path::Path>) -> EditBuffer {
        let ext: Option<&str> = path.and_then(|p| p.extension()).map(|ext| ext.to_str().unwrap());
        let init_buf = read_buffer(path);
        let n_rows = init_buf.len();
        let message_box = MessageBox::new();
        EditBuffer {
            rb: ReadBuffer::new(init_buf, message_box.clone()),
            state: INIT.to_owned(),
            visual_cursor: None,
            change_log_buffer: ChangeLogBuffer::new(),
            edit_state: None,
            path: path.map(|x| x.to_owned()),
            sync_clock: None,
            highlighter: highlight::Highlighter::new(n_rows, ext),
            message_box,
        }
    }
    fn insert_new_line(&mut self, row: usize) {
        self.rb.buf.insert(row, vec![]);
        self.rb.cache_insert_new_line(row);
        // self.highlighter.cache_insert_new_line(row);
        self.highlighter.clear_cache(self.rb.buf.len());
    }
    fn remove_line(&mut self, row: usize) {
        self.rb.buf.remove(row);
        self.rb.cache_remove_line(row);
        // self.highlighter.cache_remove_line(row);
        self.highlighter.clear_cache(self.rb.buf.len());
    }
    fn update_cache(&mut self) {
        self.rb.update_cache();

        flame::start("update highlight");
        self.highlighter
            .update_cache(self.rb.lineno_range(), &self.rb.buf);
        flame::end("update highlight");
    }
    fn is_dirty(&self) -> bool {
        match (self.sync_clock, self.change_log_buffer.clock()) {
            (_, None) => false,
            (None, Some(_)) => true,
            (Some(x), Some(y)) => x < y,
        }
    }
    fn apply_log(&mut self, log: &mut ChangeLog) {
        let delete_range = CursorRange {
            start: log.at,
            end: self.find_cursor_pair(log.at, log.deleted.len()),
        };
        let (mut pre_survivors, _, mut post_survivors) = self.prepare_delete(&delete_range);
        pre_survivors.append(&mut log.inserted);
        pre_survivors.append(&mut post_survivors);
        assert!(!pre_survivors.is_empty());
        self.insert_new_line(log.at.row);
        let mut b = false;
        self.insert(
            Cursor {
                row: log.at.row,
                col: 0,
            },
            pre_survivors,
            &mut b,
        );
    }
    fn rollback_sync_clock(&mut self) {
        match (self.sync_clock, self.change_log_buffer.clock()) {
            (Some(a), Some(b)) => {
                if a > b {
                    self.sync_clock = None;
                }
            }
            (Some(_), None) => {
                self.sync_clock = None;
            }
            (None, _) => {}
        }
    }
    fn undo(&mut self) -> bool {
        let log = self.change_log_buffer.pop_undo();
        if log.is_none() {
            return false;
        }
        self.rollback_sync_clock();
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
    
    pub fn visual_range(&self) -> Option<CursorRange> {
        self.visual_cursor.clone().map(|vc| {
            if self.rb.cursor > vc {
                CursorRange {
                    start: vc,
                    end: to_cursor_range_end(self.rb.cursor),
                }
            } else {
                CursorRange {
                    start: self.rb.cursor.clone(),
                    end: to_cursor_range_end(vc),
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
                self.insert_new_line(row);
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
    fn prepare_delete(&mut self, r: &CursorRange) -> (Vec<BufElem>, Vec<BufElem>, Vec<BufElem>) {
        let mut pre_survivors = vec![];
        let mut post_survivors = vec![];
        let mut removed = vec![];

        // invariant:
        // never eliminate the last eol
        let back_tail: bool =
            if r.end.col == self.rb.buf[r.end.row].len() && r.end.row == self.rb.buf.len() - 1 {
                true
            } else {
                false
            };

        let range = if back_tail {
            CursorRange {
                start: r.start,
                end: Cursor {
                    row: r.end.row,
                    col: r.end.col - 1, // the end.col is exclusive so it is safe to -1
                },
            }
        } else {
            r.clone()
        };

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
            self.remove_line(row);
        }

        (pre_survivors, removed, post_survivors)
    }
    fn enter_edit_mode(
        &mut self,
        r: &CursorRange,
        init_pre: Vec<BufElem>,
        init_post: Vec<BufElem>,
    ) -> Vec<BufElem> {
        let (pre_survivors, removed, post_survivors) = self.prepare_delete(&r);
        let orig_buf = self.rb.buf.clone();
        self.edit_state = Some(EditState {
            diff_buffer: DiffBuffer {
                pre_buf_raw: pre_survivors,
                diff_buf_pre: init_pre,
                diff_buf_raw: vec![],
                diff_buf_post: init_post,
                post_buf_raw: post_survivors,
            },
            at: r.start,
            removed: removed.clone(),
            orig_buf: orig_buf,
        });

        let es = self.edit_state.clone().unwrap();
        self.insert_new_line(es.at.row);
        let mut b = false;
        let after_pre_inserted = self.insert(
            Cursor {
                row: es.at.row,
                col: 0,
            },
            es.diff_buffer.pre_buf(),
            &mut b,
        );
        let after_diff_inserted = self.insert(
            after_pre_inserted,
            es.diff_buffer.diff_buf_raw.clone(),
            &mut b,
        ); // will delete
        self.insert(after_diff_inserted, es.diff_buffer.post_buf(), &mut b);
        self.rb.cursor = after_diff_inserted;
        self.visual_cursor = None;

        removed
    }
    fn leave_edit_mode(&mut self) {
        assert!(self.edit_state.is_some());
        // take(): replace the memory region with None and take out the owrnership of the object
        let edit_state = self.edit_state.take().unwrap();
        assert!(self.edit_state.is_none());
        let change_log = ChangeLog::new(
            edit_state.at,
            edit_state.removed,
            edit_state.diff_buffer.inserted(),
        );
        if change_log.deleted.len() > 0 || change_log.inserted.len() > 0 {
            self.change_log_buffer.push(change_log);
        }
    }

    //
    // effect functions
    //

    pub fn eff_undo(&mut self, _: Key) -> String {
        self.undo();
        INIT.to_owned()
    }
    pub fn eff_redo(&mut self, _: Key) -> String {
        self.redo();
        INIT.to_owned()
    }
    pub fn eff_enter_insert_newline(&mut self, _: Key) -> String {
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
        let mut v = vec![BufElem::Eol];
        let auto_indent = indent::AutoIndent {
            line_predecessors: &self.rb.buf[row][0..self.rb.buf[row].len() - 1],
        };
        v.append(&mut auto_indent.next_indent());
        self.enter_edit_mode(&delete_range, v, vec![]);
        INSERT.to_owned()
    }
    pub fn eff_enter_insert_newline_above(&mut self, _: Key) -> String {
        let row = self.rb.cursor.row;
        let delete_range = CursorRange {
            start: Cursor { row: row, col: 0 },
            end: Cursor { row: row, col: 0 },
        };
        let auto_indent = indent::AutoIndent {
            line_predecessors: &self.rb.buf[row][0..self.rb.buf[row].len() - 1],
        };
        self.enter_edit_mode(
            &delete_range,
            auto_indent.current_indent(),
            vec![BufElem::Eol],
        );
        INSERT.to_owned()
    }
    pub fn eff_join_next_line(&mut self, _: Key) -> String {
        let row = self.rb.cursor.row;
        if row == self.rb.buf.len() - 1 {
            return INIT.to_owned();
        }
        let mut next_line_first_nonspace_pos = 0;
        for x in &self.rb.buf[row + 1] {
            match *x {
                BufElem::Char(' ') | BufElem::Char('\t') => next_line_first_nonspace_pos += 1,
                _ => break,
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
            },
        };
        self.enter_edit_mode(&delete_range, vec![], vec![]);
        INSERT.to_owned()
    }
    pub fn eff_enter_insert_mode(&mut self, _: Key) -> String {
        assert!(self.edit_state.is_none());
        let delete_range = CursorRange {
            start: self.rb.cursor,
            end: self.rb.cursor,
        };
        self.enter_edit_mode(&delete_range, vec![], vec![]);
        INSERT.to_owned()
    }
    pub fn eff_enter_insert_mode_line_last(&mut self, _: Key) -> String {
        let eol_cursor = Cursor {
            row: self.rb.cursor.row,
            col: self.rb.buf[self.rb.cursor.row].len() - 1,
        };
        let delete_range = CursorRange {
            start: eol_cursor,
            end: eol_cursor,
        };
        self.enter_edit_mode(&delete_range, vec![], vec![]);
        INSERT.to_owned()
    }
    pub fn eff_enter_append_mode(&mut self, k: Key) -> String {
        self.rb.cursor_right(); 
        self.eff_enter_insert_mode(k)
    }
    pub fn eff_enter_change_mode(&mut self, _: Key) -> String {
        let delete_range = if self.visual_range().is_none() {
            CursorRange {
                start: self.rb.cursor,
                end: self.rb.cursor,
            }
        } else {
            self.visual_range().unwrap()
        };
        self.enter_edit_mode(&delete_range, vec![], vec![]);
        INSERT.to_owned()
    }
    pub fn eff_edit_mode_input(&mut self, k: Key) -> String {
        let es = self.edit_state.as_mut().unwrap();
        es.diff_buffer.input(k.clone());

        let es = self.edit_state.clone().unwrap();

        {
            let n = self.rb.buf.len();
            let m = es.orig_buf.len();
            let rows_to_remove = n - m;
            for i in (0..rows_to_remove).rev() {
                let row = es.at.row + i;
                self.remove_line(row);
            }
        }

        self.rb.buf = es.orig_buf;
        self.insert_new_line(es.at.row);

        let mut b = false;
        let after_pre_inserted = self.insert(
            Cursor {
                row: es.at.row,
                col: 0,
            },
            es.diff_buffer.pre_buf(),
            &mut b,
        );
        let after_diff_inserted = self.insert(
            after_pre_inserted,
            es.diff_buffer.diff_buf_raw.clone(),
            &mut b,
        );
        self.insert(after_diff_inserted, es.diff_buffer.post_buf(), &mut b);
        self.rb.cursor = after_diff_inserted;

        INSERT.to_owned()
    }

    pub fn eff_leave_edit_mode(&mut self, _: Key) -> String {
        self.leave_edit_mode();
        INIT.to_owned()
    }
    fn delete_range(&mut self, range: CursorRange) -> Vec<BufElem> {
        let removed = self.enter_edit_mode(&range, vec![], vec![]);
        self.leave_edit_mode();
        removed
    }
    pub fn eff_delete_line_tail(&mut self, _: Key) -> String {
        let start = self.rb.cursor;
        let line = &self.rb.buf[start.row];
        assert!(!line.is_empty());
        let end = if line.len() == 1 {
            start
        } else {
            Cursor {
                row: start.row,
                col: line.len() - 1
            }
        };
        let delete_range = CursorRange { start, end };
        self.delete_range(delete_range);
        INIT.to_owned()
    }
    pub fn eff_delete_line(&mut self, _: Key) -> String {
        let range = CursorRange {
            start: Cursor {
                row: self.rb.cursor.row,
                col: 0,
            },
            end: Cursor {
                row: self.rb.cursor.row,
                col: self.rb.buf[self.rb.cursor.row].len(),
            },
        };
        let removed = self.delete_range(range);
        clipboard::SINGLETON.copy(clipboard::Type::Line(removed));
        INIT.to_owned()
    }
    pub fn eff_delete_range(&mut self, _: Key) -> String {
        if self.visual_range().is_none() {
            LINES.to_owned()
        } else {
            let vr = self.visual_range().unwrap();
            let removed = self.delete_range(vr);
            clipboard::SINGLETON.copy(clipboard::Type::Range(removed));
            INIT.to_owned()
        }
    }
    pub fn eff_delete_char(&mut self, _: Key) -> String {
        let range = self.visual_range().unwrap_or(CursorRange {
            start: self.rb.cursor,
            end: Cursor {
                row: self.rb.cursor.row,
                col: self.rb.cursor.col + 1,
            },
        });
        let removed = self.delete_range(range);
        clipboard::SINGLETON.copy(clipboard::Type::Range(removed));

        INIT.to_owned()
    }
    pub fn eff_cancel_lines_mode(&mut self, _: Key) -> String {
        INIT.to_owned()
    }
    pub fn eff_paste_system(&mut self, _ :Key) -> String {
        let pasted = clipboard::paste_system();
        if pasted.is_none() {
            self.message_box.send("nothing to paste");
            return INIT.to_owned();
        }
        let pasted = pasted.unwrap();

        let mut log = ChangeLog::new(self.rb.cursor, vec![], pasted);
        self.change_log_buffer.push(log.clone());
        self.apply_log(&mut log);

        INIT.to_owned()
    }
    pub fn eff_paste(&mut self, _: Key) -> String {
        let pasted = clipboard::SINGLETON.paste();
        if pasted.is_none() {
            self.message_box.send("yank not found");
            return INIT.to_owned();
        }

        let pasted = pasted.unwrap();
        match pasted {
            clipboard::Type::Range(v) => {
                self.rb.cursor_right();
                let mut log = ChangeLog::new(self.rb.cursor, vec![], v);
                self.change_log_buffer.push(log.clone());
                self.apply_log(&mut log);

                INIT.to_owned()
            },
            clipboard::Type::Line(v) => {
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
                let mut v = trim_right(v);
                let mut vv = vec![BufElem::Eol];
                vv.append(&mut v);
                self.enter_edit_mode(
                    &delete_range,
                    vv,
                    vec![],
                );
                self.leave_edit_mode();

                self.rb.jump_line_head();

                INIT.to_owned()
            }
        }
    }
    pub fn eff_paste_above(&mut self, _: Key) -> String {
        let pasted = clipboard::SINGLETON.paste();
        if pasted.is_none() {
            return INIT.to_owned();
        }

        match pasted.unwrap() {
            clipboard::Type::Line(v) => {
                let mut v = trim_right(v);
                v.push(BufElem::Eol);

                let row = self.rb.cursor.row;
                let delete_range = CursorRange {
                    start: Cursor {
                        row: row,
                        col: 0,
                    },
                    end: Cursor {
                        row: row,
                        col: 0,
                    },
                };
                self.enter_edit_mode(
                    &delete_range,
                    v,
                    vec![],
                );
                self.leave_edit_mode();

                self.rb.cursor_up();
                self.rb.jump_line_head()
            },
            clipboard::Type::Range(v) => {
                let row = self.rb.cursor.row;
                let line = self.rb.line(self.rb.cursor.row);
                let delete_range = CursorRange {
                    start: Cursor {
                        row: row,
                        col: line.first_non_space_index(),
                    },
                    end: Cursor {
                        row: row,
                        col: line.first_non_space_index(),
                    },
                };
                self.enter_edit_mode(
                    &delete_range,
                    v,
                    vec![],
                );
                self.leave_edit_mode();
            }
        };

;

        INIT.to_owned()
    }
    // tmp:
    // to get the yank region and restore the editor state
    // we do this trick of delete and then undo.
    fn get_buffer(&mut self, range: CursorRange) -> Vec<BufElem> {
        self.delete_range(range);
        let to_copy = self.change_log_buffer.peek().cloned().unwrap().deleted;
        self.undo();
        to_copy
    }
    pub fn eff_yank_range(&mut self, _: Key) -> String {
        let orig_cursor = self.rb.cursor;
        let vr = self.visual_range();
        if vr.is_none() {
            return LINES.to_owned();
        }

        let to_copy = self.get_buffer(vr.unwrap());
        clipboard::SINGLETON.copy(clipboard::Type::Range(to_copy));
        self.rb.cursor = orig_cursor;

        INIT.to_owned()
    }
    pub fn eff_yank_line(&mut self, _: Key) -> String {
        let range = CursorRange {
            start: Cursor {
                row: self.rb.cursor.row,
                col: 0,
            },
            end: Cursor {
                row: self.rb.cursor.row,
                col: self.rb.buf[self.rb.cursor.row].len(),
            },
        };
        let to_copy = self.get_buffer(range);
        clipboard::SINGLETON.copy(clipboard::Type::Line(to_copy));

        INIT.to_owned()
    }
    fn indent_back_line(&mut self, row: usize, indent: &[BufElem]) {
        let mut cnt = 0;
        for i in 0..indent.len() {
            if self.rb.buf[row][i] != indent[i] {
                break;
            }
            cnt += 1;
        }
        self.delete_range(CursorRange {
            start: Cursor { row: row, col: 0 },
            end: Cursor { row: row, col: cnt },
        });
    }
    fn indent_back_range(&mut self, row_range: std::ops::Range<usize>) {
        for row in row_range {
            self.indent_back_line(row, &vec![BufElem::Char(' '); 4]);
        }
    }
    pub fn eff_indent_back(&mut self, _: Key) -> String {
        if self.visual_range().is_none() {
            self.indent_back_range(self.rb.cursor.row..self.rb.cursor.row + 1);
            return INIT.to_owned();
        }
        let vr = self.visual_range().unwrap();
        self.indent_back_range(vr.start.row..vr.end.row + 1);
        self.visual_cursor = None;
        // TODO atomic change log

        INIT.to_owned()
    }
    pub fn eff_enter_visual_mode(&mut self, _: Key) -> String {
        self.visual_cursor = Some(self.rb.cursor.clone());
        INIT.to_owned()
    }
    pub fn eff_reset(&mut self, _: Key) -> String {
        self.visual_cursor = None;
        self.rb.reset();
        INIT.to_owned()
    }
    pub fn eff_save_to_file(&mut self, _: Key) -> String {
        if self.path.is_none() {
            return INIT.to_owned();
        }
        let path = self.path.clone().unwrap();
        if let Ok(file) = fs::File::create(path) {
            let buf = &self.rb.buf;
            crate::normalize::write_to_file(file, &buf);
            self.sync_clock = self.change_log_buffer.clock();
            self.message_box.send("Saved")
        }
        INIT.to_owned()
    }
    pub fn eff_cursor_up(&mut self, _: Key) -> String {
        self.rb.cursor_up();
        INIT.to_owned()
    }
    pub fn eff_cursor_down(&mut self, _: Key) -> String {
        self.rb.cursor_down();
        INIT.to_owned()
    }
    pub fn eff_cursor_left(&mut self, _: Key) -> String {
        self.rb.cursor_left();
        INIT.to_owned()
    }
    pub fn eff_cursor_right(&mut self, _: Key) -> String {
        self.rb.cursor_right();
        INIT.to_owned()
    }
    pub fn eff_jump_line_head(&mut self, _: Key) -> String {
        self.rb.jump_line_head();
        INIT.to_owned()
    }
    pub fn eff_jump_line_last(&mut self, _: Key) -> String {
        self.rb.jump_line_last();
        INIT.to_owned()
    }
    pub fn eff_jump_page_forward(&mut self, _: Key) -> String {
        self.rb.jump_page_forward();
        INIT.to_owned()
    }
    pub fn eff_jump_page_backward(&mut self, _: Key) -> String {
        self.rb.jump_page_backward();
        INIT.to_owned()
    }
    pub fn eff_enter_jump_mode(&mut self, k: Key) -> String {
        self.rb.enter_jump_mode(k);
        JUMP.to_owned()
    }
    pub fn eff_acc_jump_num(&mut self, k: Key) -> String {
        self.rb.acc_jump_num(k);
        JUMP.to_owned()
    }
    pub fn eff_jump(&mut self, _: Key) -> String {
        self.rb.jump();
        INIT.to_owned()
    }
    pub fn eff_cancel_jump(&mut self, _: Key) -> String {
        self.rb.cancel_jump();
        INIT.to_owned()
    }
    pub fn eff_jump_last(&mut self, _: Key) -> String {
        self.rb.jump_last();
        INIT.to_owned()
    }
    pub fn eff_enter_search_mode(&mut self, _: Key) -> String {
        self.rb.enter_search_mode();
        SEARCH.to_owned()
    }
    pub fn eff_search_mode_input(&mut self, k: Key) -> String {
        self.rb.search_mode_input(k);
        SEARCH.to_owned()
    }
    pub fn eff_leave_search_mode(&mut self, _: Key) -> String {
        self.rb.leave_search_mode();
        INIT.to_owned()
    }
    fn eff_cancel_search_mode(&mut self, _: Key) -> String {
        self.rb.cancel_search_mode();
        INIT.to_owned()
    }
    pub fn eff_search_jump_forward(&mut self, _: Key) -> String {
        self.rb.search_jump_forward();
        INIT.to_owned()
    }
    pub fn eff_search_jump_backward(&mut self, _: Key) -> String {
        self.rb.search_jump_backward();
        INIT.to_owned()
    }

    pub fn eff_enter_replace_once_mode(&mut self, _: Key) -> String {
        REPLACE_ONCE.to_owned()
    }
    pub fn eff_cancel_replace_once_mode(&mut self, _: Key) -> String {
        INIT.to_owned()
    }
    pub fn eff_commit_replace_once(&mut self, k: Key) -> String {
        match k {
            Key::Char(c) => {
                let before = self.rb.buf[self.rb.cursor.row][self.rb.cursor.col].clone();
                if before == BufElem::Eol {
                    // do nothing
                    INIT.to_owned()
                } else {
                    let delete_range = CursorRange {
                        start: self.rb.cursor,
                        end: to_cursor_range_end(self.rb.cursor),
                    };
                    self.enter_edit_mode(&delete_range, vec![BufElem::Char(c)], vec![]);
                    self.leave_edit_mode();
                    INIT.to_owned()
                }
            },
            _ => REPLACE_ONCE.to_owned()
        }
    }
    fn eff_enter_warp(&mut self, _: Key) -> String {
        WARP.to_owned()
    }
    fn eff_cancel_warp(&mut self, _: Key) -> String {
        INIT.to_owned()
    }
    fn eff_warp(&mut self, k: Key) -> String {
        self.rb.enter_search_mode();
        self.rb.search_mode_input(k);
        self.rb.leave_search_mode();
        INIT.to_owned()
    }
}

use crate::Key;
use std::cell::RefCell;
use std::rc::Rc;

use crate::controller::Effect;
use crate::def_effect;

def_effect!(Undo, EditBuffer, eff_undo);
def_effect!(Redo, EditBuffer, eff_redo);
def_effect!(JoinNextLine, EditBuffer, eff_join_next_line);
def_effect!(EnterInsertNewline, EditBuffer, eff_enter_insert_newline);
def_effect!(EnterInsertNewlineAbove, EditBuffer, eff_enter_insert_newline_above);
def_effect!(EnterInsertMode, EditBuffer, eff_enter_insert_mode);
def_effect!(EnterInsertModeLineLast, EditBuffer, eff_enter_insert_mode_line_last);
def_effect!(EnterAppendMode, EditBuffer, eff_enter_append_mode);
def_effect!(EnterChangeMode, EditBuffer, eff_enter_change_mode);
def_effect!(EditModeInput, EditBuffer, eff_edit_mode_input);
def_effect!(LeaveEditMode, EditBuffer, eff_leave_edit_mode);
def_effect!(DeleteLineTail, EditBuffer, eff_delete_line_tail);
def_effect!(DeleteLine, EditBuffer, eff_delete_line);
def_effect!(DeleteRange, EditBuffer, eff_delete_range);
def_effect!(DeleteChar, EditBuffer, eff_delete_char);
def_effect!(CancelLinesMode, EditBuffer, eff_cancel_lines_mode);
def_effect!(Paste, EditBuffer, eff_paste);
def_effect!(PasteAbove, EditBuffer, eff_paste_above);
def_effect!(PasteSystem, EditBuffer, eff_paste_system);
def_effect!(YankRange, EditBuffer, eff_yank_range);
def_effect!(YankLine, EditBuffer, eff_yank_line);
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

def_effect!(EnterReplaceOnceMode, EditBuffer, eff_enter_replace_once_mode);
def_effect!(CancelReplaceOnceMode, EditBuffer, eff_cancel_replace_once_mode);
def_effect!(CommitReplaceOnce, EditBuffer, eff_commit_replace_once);

def_effect!(EnterSearchMode, EditBuffer, eff_enter_search_mode);
def_effect!(SearchModeInput, EditBuffer, eff_search_mode_input);
def_effect!(LeaveSearchMode, EditBuffer, eff_leave_search_mode);
def_effect!(CancelSearchMode, EditBuffer, eff_cancel_search_mode);
def_effect!(SearchJumpForward, EditBuffer, eff_search_jump_forward);
def_effect!(SearchJumpBackward, EditBuffer, eff_search_jump_backward);

def_effect!(EnterWarp, EditBuffer, eff_enter_warp);
def_effect!(CancelWarp, EditBuffer, eff_cancel_warp);
def_effect!(Warp, EditBuffer, eff_warp);

use crate::controller;
pub fn mk_controller(x: Rc<RefCell<EditBuffer>>) -> controller::ControllerFSM {
    use crate::Key::*;
    let mut g = controller::GraphImpl::new();

    // mutable
    g.add_edge(INIT, Ctrl('s'), Rc::new(SaveToFile(x.clone())));
    g.add_edge(INIT, Char('v'), Rc::new(EnterVisualMode(x.clone())));
    g.add_edge(INIT, Char('D'), Rc::new(DeleteLineTail(x.clone())));
    g.add_edge(INIT, Char('d'), Rc::new(DeleteRange(x.clone())));
    g.add_edge(INIT, Char('x'), Rc::new(DeleteChar(x.clone())));
    g.add_edge(INIT, Char('<'), Rc::new(IndentBack(x.clone())));
    g.add_edge(INIT, Char('J'), Rc::new(JoinNextLine(x.clone())));
    g.add_edge(INIT, Char('o'), Rc::new(EnterInsertNewline(x.clone())));
    g.add_edge(INIT, Char('O'), Rc::new(EnterInsertNewlineAbove(x.clone())));
    g.add_edge(INIT, Char('i'), Rc::new(EnterInsertMode(x.clone())));
    g.add_edge(INIT, Char('A'), Rc::new(EnterInsertModeLineLast(x.clone())));
    g.add_edge(INIT, Char('a'), Rc::new(EnterAppendMode(x.clone())));
    g.add_edge(INIT, Char('c'), Rc::new(EnterChangeMode(x.clone())));
    g.add_edge(INIT, Char('p'), Rc::new(Paste(x.clone())));
    g.add_edge(INIT, Char('P'), Rc::new(PasteAbove(x.clone())));
    g.add_edge(INIT, Ctrl('p'), Rc::new(PasteSystem(x.clone())));
    g.add_edge(INIT, Char('y'), Rc::new(YankRange(x.clone())));
    g.add_edge(INIT, Esc, Rc::new(Reset(x.clone())));

    g.add_edge(INIT, Char('r'), Rc::new(EnterReplaceOnceMode(x.clone())));
    g.add_edge(REPLACE_ONCE, Esc, Rc::new(CancelReplaceOnceMode(x.clone())));
    g.add_edge(REPLACE_ONCE, Otherwise, Rc::new(CommitReplaceOnce(x.clone())));
    
    g.add_edge(LINES, Char('y'), Rc::new(YankLine(x.clone())));
    g.add_edge(LINES, Char('d'), Rc::new(DeleteLine(x.clone())));
    g.add_edge(LINES, Esc, Rc::new(CancelLinesMode(x.clone())));

    g.add_edge(INSERT, Esc, Rc::new(LeaveEditMode(x.clone())));
    g.add_edge(INSERT, Otherwise, Rc::new(EditModeInput(x.clone())));

    g.add_edge(INIT, Ctrl('r'), Rc::new(Redo(x.clone())));
    g.add_edge(INIT, Char('u'), Rc::new(Undo(x.clone())));

    // immutable
    g.add_edge(INIT, Char('k'), Rc::new(CursorUp(x.clone())));
    g.add_edge(INIT, Char('j'), Rc::new(CursorDown(x.clone())));
    g.add_edge(INIT, Char('h'), Rc::new(CursorLeft(x.clone())));
    g.add_edge(INIT, Char('l'), Rc::new(CursorRight(x.clone())));
    g.add_edge(INIT, Char('0'), Rc::new(JumpLineHead(x.clone())));
    g.add_edge(INIT, Char('$'), Rc::new(JumpLineLast(x.clone())));
    g.add_edge(INIT, Ctrl('f'), Rc::new(JumpPageForward(x.clone())));
    g.add_edge(INIT, Ctrl('b'), Rc::new(JumpPageBackward(x.clone())));
    g.add_edge(INIT, Char('G'), Rc::new(JumpLast(x.clone())));
    g.add_edge(INIT, Char('n'), Rc::new(SearchJumpForward(x.clone())));
    g.add_edge(INIT, Char('N'), Rc::new(SearchJumpBackward(x.clone())));

    // num jump
    g.add_edge(INIT, CharRange('1', '9'), Rc::new(EnterJumpMode(x.clone())));
    g.add_edge(JUMP, CharRange('0', '9'), Rc::new(AccJumpNum(x.clone())));
    g.add_edge(JUMP, Char('G'), Rc::new(Jump(x.clone())));
    g.add_edge(JUMP, Esc, Rc::new(CancelJump(x.clone())));

    // search
    g.add_edge(INIT, Char('/'), Rc::new(EnterSearchMode(x.clone())));
    g.add_edge(SEARCH, Char('\n'), Rc::new(LeaveSearchMode(x.clone())));
    g.add_edge(SEARCH, Esc, Rc::new(CancelSearchMode(x.clone())));
    g.add_edge(SEARCH, Otherwise, Rc::new(SearchModeInput(x.clone())));

    g.add_edge(INIT, Char('g'), Rc::new(EnterWarp(x.clone())));
    g.add_edge(WARP, Esc, Rc::new(CancelWarp(x.clone())));
    g.add_edge(WARP, Otherwise, Rc::new(Warp(x.clone())));

    controller::ControllerFSM::new(INIT, Box::new(g))
}

use crate::view;
pub struct VisualRangeDiffView {
    range: Option<CursorRange>, // doubtful design to have option here
}
impl view::DiffView for VisualRangeDiffView {
    fn get(&self, col: usize, row: usize) -> view::ViewElemDiff {
        let as_cursor = Cursor { row, col };
        let in_visual_range = self
            .range
            .map(|r| r.start <= as_cursor && as_cursor < r.end)
            .unwrap_or(false);
        if in_visual_range {
            (None, None, Some(screen::Color::Blue))
        } else {
            (None, None, None)
        }
    }
}
impl VisualRangeDiffView {
    pub fn new(range: Option<CursorRange>) -> Self {
        Self { range }
    }
}

pub struct ViewGen {
    buf: Rc<RefCell<EditBuffer>>,
}
impl ViewGen {
    pub fn new(buf: Rc<RefCell<EditBuffer>>) -> Self {
        Self { buf: buf }
    }
}
impl view::ViewGen for ViewGen {
    fn gen(&self, region: view::Area) -> Box<view::View> {
        let (lineno_reg, buf_reg) = region.split_horizontal(view::LINE_NUMBER_W);

        self.buf.borrow_mut().rb.stabilize_cursor();
        self.buf
            .borrow_mut()
            .rb
            .adjust_window(buf_reg.width, buf_reg.height);
        self.buf.borrow_mut().update_cache();

        let lineno_range = self.buf.borrow().rb.lineno_range();
        let lineno_view = view::LineNumber {
            from: lineno_range.start + 1,
            to: lineno_range.end,
        };
        let lineno_view =
            view::TranslateView::new(lineno_view, lineno_reg.col as i32, lineno_reg.row as i32);

        // let buf_view = view::ToView::new(self.buf.borrow().rb.buf.clone());
        let buf_ref = self.buf.borrow();
        let buf_window = self.buf.borrow().rb.current_window();
        let buf_view = view::ToViewRef::new(&buf_ref.rb.buf);
        // let buf_view = view::ToView::new(&self.buf.borrow().rb.buf, buf_window);
        // let highlight_diff = highlight::HighlightDiffView::new(&self.buf.borrow().highlighter, buf_window);
        let highlight_diff = highlight::HighlightDiffViewRef::new(&buf_ref.highlighter);
        let buf_view = view::OverlayView::new(buf_view, highlight_diff);
        let buf_view = view::OverlayView::new(
            buf_view,
            search::DiffView::new(self.buf.borrow().rb.search.clone()),
        );
        let buf_view = view::OverlayView::new(
            buf_view,
            VisualRangeDiffView::new(self.buf.borrow().visual_range()),
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

        let view = view::CloneView::new(view, region);
        Box::new(view)
    }
}

pub struct Page {
    controller: Box<controller::Controller>,
    view_gen: Box<view::ViewGen>,
    x: Rc<RefCell<EditBuffer>>,
}
impl Page {
    pub fn new(x: Rc<RefCell<EditBuffer>>) -> Self {
        Self {
            controller: Box::new(mk_controller(x.clone())),
            view_gen: Box::new(ViewGen::new(x.clone())),
            x: x,
        }
    }
}
impl navigator::Page for Page {
    fn controller(&self) -> &Box<controller::Controller> {
        &self.controller
    }
    fn view_gen(&self) -> &Box<view::ViewGen> {
        &self.view_gen
    }
    fn status(&self) -> String {
        let s = match self.x.borrow().path.clone() {
            Some(p) => p.to_str().unwrap().to_owned(),
            None => "noname".to_owned(),
        };
        let dirty_mark = if self.x.borrow().is_dirty() {
            "[D] "
        } else {
            ""
        };
        let state = self.x.borrow().state.clone();
        format!("[{}] {}{}", state, dirty_mark, s)
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
    fn message(&self) -> MessageBox {
        self.x.borrow().message_box.clone()
    }
}

pub mod change_log;
pub mod clipboard;
pub mod diff_buffer;
pub mod highlight;
pub mod indent;
pub mod undo_buffer;
mod diff_tree;
mod snippet;
pub mod config;

use self::change_log::{ChangeLog, ChangeLogBuffer};
use self::diff_buffer::DiffBuffer;

pub use self::indent::IndentType;
pub use self::config::Config;

use crate::view::{ViewGen, Area};
use crate::navigator::Navigator;
use crate::message_box::MessageBox;
use crate::navigator;
use crate::read_buffer::{self, *};
use crate::screen::{self, Color};
use crate::read_buffer::{BufElem, Cursor, CursorRange};
use std::fs;
use std::path;
use std::time::Instant;
use self::snippet::SnippetElem;
use self::diff_tree::ChildComponent;
use crate::read_buffer::{INIT, SEARCH, JUMP};

const COMMAND: &str = "Command";
const REPLACE_ONCE: &str = "ReplaceOnce";
const WARP: &str = "Warp";
const WILL_DELETE: &str = "WillDelete";
const WILL_YANK: &str = "WillYank";
const WILL_CHANGE: &str = "WillChange";
const INSERT: &str = "Insert";
const SNIPPET: &str = "Snippet";

fn to_elems(x: &str) -> Vec<BufElem> {
    let mut v = vec![];
    for c in x.chars() {
        let e = match c {
            '\n' => BufElem::Eol,
            c => BufElem::Char(c),
        };
        v.push(e)
    }
    v
}

fn to_cursor_range_end(cursor: Cursor) -> Cursor {
    Cursor {
        row: cursor.row,
        col: cursor.col + 1,
    }
}

pub struct EditBuffer {
    pub rb: ReadBuffer,
    config: Config,
    visual_cursor: Option<Cursor>,
    change_log_buffer: ChangeLogBuffer,
    edit_state: Option<EditState>,
    path: path::PathBuf,
    sync_clock: Option<Instant>,
    highlighter: highlight::Highlighter,
    snippet_repo: snippet::SnippetRepo,
    navigator: Rc<RefCell<Navigator>>,
    state: PageState,
    message_box: MessageBox,
}

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

pub fn read_buffer(path: &path::Path) -> Vec<Vec<BufElem>> {
    let s = fs::read_to_string(path).ok();
    crate::read_buffer::read_from_string(s)
}

impl EditBuffer {
    pub fn open(path: &path::Path, navigator: Rc<RefCell<Navigator>>) -> EditBuffer {
        let ext: Option<&str> = path.extension().map(|ext| ext.to_str().unwrap());
        let init_buf = read_buffer(path);
        let n_rows = init_buf.len();
        let state = PageState::new(INIT.to_owned());
        let message_box = MessageBox::new();
        let config = crate::config::SINGLETON.get_config(path);

        EditBuffer {
            rb: ReadBuffer::new(init_buf, state.clone(), message_box.clone()),
            snippet_repo: snippet::SnippetRepo::new(config.snippet.clone(), state.clone(), message_box.clone()),
            config: config,
            visual_cursor: None,
            change_log_buffer: ChangeLogBuffer::new(),
            edit_state: None,
            path: path.to_owned(),
            sync_clock: None,
            highlighter: highlight::Highlighter::new(n_rows, ext),
            navigator,
            state,
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
            &pre_survivors,
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
        buf: &[BufElem],
        should_insert_newline: &mut bool,
    ) -> Cursor {
        let mut row = at.row;
        let mut col = at.col;
        for e in buf {
            if *should_insert_newline {
                self.insert_new_line(row);
                *should_insert_newline = false;
            }
            match e.clone() {
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
    fn create_edit_state(
        &mut self,
        r: &CursorRange,
        init_pre: Vec<BufElem>,
        init_post: Vec<BufElem>,
    ) {
        let (pre_survivors, removed, post_survivors) = self.prepare_delete(&r);
        self.edit_state = Some(EditState {
            diff_buffer: DiffBuffer::new(
                pre_survivors,
                init_pre,
                init_post,
                post_survivors,
                self.config.indent_type,
            ),
            at: r.start,
            removed: removed,
            orig_buf: self.rb.buf.clone(),
        });

        self.writeback_edit_state();

        self.visual_cursor = None;
    }
    fn commit_edit_state(&mut self) -> Vec<BufElem> {
        assert!(self.edit_state.is_some());
        // take(): replace the memory region with None and take out the owrnership of the object
        let edit_state = self.edit_state.take().unwrap();
        assert!(self.edit_state.is_none());
        let change_log = ChangeLog::new(
            edit_state.at,
            edit_state.removed.clone(),
            edit_state.diff_buffer.inserted(),
        );
        if change_log.deleted.len() > 0 || change_log.inserted.len() > 0 {
            self.change_log_buffer.push(change_log);
        }

        edit_state.removed
    }
    fn delete_range(&mut self, range: CursorRange) -> Vec<BufElem> {
        self.create_edit_state(&range, vec![], vec![]);
        let removed = self.commit_edit_state();
        removed
    }
    fn word_range(&self) -> Option<CursorRange> {
        let col_range = self.rb.line(self.rb.cursor.row).word_range(self.rb.cursor.col);
        col_range.map(|r| CursorRange {
            start: Cursor { row: self.rb.cursor.row, col: r.start },
            end: Cursor { row: self.rb.cursor.row, col: r.end }
        })
    }
    fn line_tail_range(&self) -> CursorRange {
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
        CursorRange { start, end }
    }

    //
    // eff functions
    //

    fn eff_enter_command_mode(&mut self, _: Key) -> String {
        COMMAND.to_owned()
    }
    fn eff_cancel_command_mode(&mut self, _: Key) -> String {
        INIT.to_owned()
    }
    fn close_buffer(&self) {
        self.navigator.borrow_mut().pop()
    }
    fn save_to_file(&mut self) {
        if let Ok(file) = fs::File::create(&self.path) {
            let buf = &self.rb.buf;
            crate::read_buffer::write_to_file(file, &buf);
            self.sync_clock = self.change_log_buffer.clock();
            self.message_box.send("Saved")
        }
    }
    fn eff_execute_command(&mut self, k: Key) -> String {
        match k {
            Key::Char('w') => self.save_to_file(),
            Key::Char('q') => self.close_buffer(),
            _ => {},
        }
        INIT.to_owned()
    }
    fn eff_undo(&mut self, _: Key) -> String {
        self.undo();
        INIT.to_owned()
    }
    fn eff_redo(&mut self, _: Key) -> String {
        self.redo();
        INIT.to_owned()
    }
    fn eff_enter_insert_newline(&mut self, _: Key) -> String {
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
        let auto_indent = indent::AutoIndent::new(
            &self.rb.buf[row][0..self.rb.buf[row].len() - 1],
            self.config.indent_type,
        );
        v.append(&mut auto_indent.next_indent());
        self.create_edit_state(&delete_range, v, vec![]);
        INSERT.to_owned()
    }
    fn eff_enter_insert_newline_above(&mut self, _: Key) -> String {
        let row = self.rb.cursor.row;
        let delete_range = CursorRange {
            start: Cursor { row: row, col: 0 },
            end: Cursor { row: row, col: 0 },
        };
        let auto_indent = indent::AutoIndent::new(
            &self.rb.buf[row][0..self.rb.buf[row].len() - 1],
            self.config.indent_type,
        );
        self.create_edit_state(
            &delete_range,
            auto_indent.current_indent(),
            vec![BufElem::Eol],
        );
        INSERT.to_owned()
    }
    fn eff_join_next_line(&mut self, _: Key) -> String {
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
        self.create_edit_state(&delete_range, vec![], vec![]);
        INSERT.to_owned()
    }
    fn eff_enter_insert_mode(&mut self, _: Key) -> String {
        assert!(self.edit_state.is_none());
        let delete_range = CursorRange {
            start: self.rb.cursor,
            end: self.rb.cursor,
        };
        self.create_edit_state(&delete_range, vec![], vec![]);
        INSERT.to_owned()
    }
    fn eff_enter_insert_mode_line_last(&mut self, _: Key) -> String {
        let eol_cursor = Cursor {
            row: self.rb.cursor.row,
            col: self.rb.buf[self.rb.cursor.row].len() - 1,
        };
        let delete_range = CursorRange {
            start: eol_cursor,
            end: eol_cursor,
        };
        self.create_edit_state(&delete_range, vec![], vec![]);
        INSERT.to_owned()
    }
    fn eff_enter_append_mode(&mut self, k: Key) -> String {
        self.rb.cursor_right(); 
        self.eff_enter_insert_mode(k)
    }
    fn eff_change_range(&mut self, _: Key) -> String {
        if self.visual_range().is_none() {
            WILL_CHANGE.to_owned()
        } else {
            let delete_range = self.visual_range().unwrap();
            self.create_edit_state(&delete_range, vec![], vec![]);
            INSERT.to_owned()
        }
    }
    fn eff_change_word(&mut self, _: Key) -> String {
        match self.word_range() {
            Some(range) => {
                self.create_edit_state(&range, vec![], vec![]);
                INSERT.to_owned()
            },
            None => {
                INIT.to_owned()
            }
        }
    }
    fn eff_change_line_tail(&mut self, _: Key) -> String {
        let delete_range = self.line_tail_range();
        self.create_edit_state(&delete_range, vec![], vec![]);
        INSERT.to_owned()
    }
    fn es_ref(&self) -> &EditState {
        self.edit_state.as_ref().unwrap()
    }
    fn es_mut(&mut self) -> &mut EditState {
        self.edit_state.as_mut().unwrap()
    }
    fn restore_buf_before_writeback(&mut self) {
        let n = self.rb.buf.len();
        let m = self.es_ref().orig_buf.len();
        let rows_to_remove = n - m;
        for i in (0..rows_to_remove).rev() {
            let row = self.es_ref().at.row + i;
            self.remove_line(row);
        }
    }
    fn writeback_edit_state(&mut self) {
        self.rb.buf = self.es_ref().orig_buf.clone(); // needless?
        self.insert_new_line(self.es_ref().at.row);

        let mut b = false;
        let after_pre_inserted = self.insert(Cursor { row: self.es_ref().at.row, col: 0, }, &self.es_ref().diff_buffer.pre_buf(), &mut b);
        let (mut diff_buf_raw0, cursor_offset) = self.es_ref().diff_buffer.diff_buf_raw.flatten();
        let diff_buf_raw1 = diff_buf_raw0.split_off(cursor_offset);
        let after_diff0_inserted = self.insert(after_pre_inserted, &diff_buf_raw0, &mut b);
        let after_diff1_inserted = self.insert(after_diff0_inserted, &diff_buf_raw1, &mut b);
        self.insert(after_diff1_inserted, &self.es_ref().diff_buffer.post_buf(), &mut b);
        
        self.rb.cursor = after_diff0_inserted;
    }
    fn eff_edit_mode_input(&mut self, k: Key) -> String {
        self.edit_state.as_mut().unwrap().diff_buffer.input(k.clone());
        self.snippet_repo.set_searcher(self.edit_state.as_mut().unwrap().diff_buffer.diff_buf_raw.current_word());

        self.restore_buf_before_writeback();
        self.writeback_edit_state();
        INSERT.to_owned()
    }
    fn eff_leave_edit_mode(&mut self, _: Key) -> String {
        self.commit_edit_state();
        INIT.to_owned()
    }
    fn eff_delete_line_tail(&mut self, _: Key) -> String {
        let delete_range = self.line_tail_range();
        let removed = self.delete_range(delete_range);
        clipboard::copy(clipboard::Type::Range(removed));
        INIT.to_owned()
    }
    fn eff_delete_line(&mut self, _: Key) -> String {
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
        clipboard::copy(clipboard::Type::Line(removed));
        INIT.to_owned()
    }
    fn eff_delete_range(&mut self, _: Key) -> String {
        if self.visual_range().is_none() {
            WILL_DELETE.to_owned()
        } else {
            let vr = self.visual_range().unwrap();
            let removed = self.delete_range(vr);
            clipboard::copy(clipboard::Type::Range(removed));
            INIT.to_owned()
        }
    }
    fn eff_delete_char(&mut self, _: Key) -> String {
        let range = self.visual_range().unwrap_or(CursorRange {
            start: self.rb.cursor,
            end: Cursor {
                row: self.rb.cursor.row,
                col: self.rb.cursor.col + 1,
            },
        });
        let removed = self.delete_range(range);
        clipboard::copy(clipboard::Type::Range(removed));

        INIT.to_owned()
    }
    fn eff_delete_word(&mut self, _: Key) -> String {
        for range in self.word_range() {
            let removed = self.delete_range(range);
            clipboard::copy(clipboard::Type::Range(removed));
        }
        INIT.to_owned()
    }
    fn eff_cancel_will_mode(&mut self, _: Key) -> String {
        INIT.to_owned()
    }
    fn eff_paste_system(&mut self, _ :Key) -> String {
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
    fn eff_paste(&mut self, _: Key) -> String {
        let pasted = clipboard::paste();
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
                self.create_edit_state(
                    &delete_range,
                    vv,
                    vec![],
                );
                self.commit_edit_state();

                self.rb.jump_line_head();

                INIT.to_owned()
            }
        }
    }
    fn eff_paste_above(&mut self, _: Key) -> String {
        let pasted = clipboard::paste();
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
                self.create_edit_state(
                    &delete_range,
                    v,
                    vec![],
                );
                self.commit_edit_state();

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
                self.create_edit_state(
                    &delete_range,
                    v,
                    vec![],
                );
                self.commit_edit_state();
            }
        };

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
    fn eff_yank_range(&mut self, _: Key) -> String {
        let orig_cursor = self.rb.cursor;
        let vr = self.visual_range();
        if vr.is_none() {
            return WILL_YANK.to_owned();
        }

        let to_copy = self.get_buffer(vr.unwrap());
        clipboard::copy(clipboard::Type::Range(to_copy));
        self.rb.cursor = orig_cursor;

        INIT.to_owned()
    }
    fn eff_yank_line(&mut self, _: Key) -> String {
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
        clipboard::copy(clipboard::Type::Line(to_copy));

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
            self.indent_back_line(row, &indent::into_bufelems(self.config.indent_type));
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
    fn indent_forward(&mut self, row: usize) {
        let delete_range = CursorRange {
            start: Cursor { row, col: 0, },
            end: Cursor { row, col: 0, },
        };
        let v = indent::into_bufelems(self.config.indent_type);
        self.create_edit_state(&delete_range, v, vec![]);
        self.commit_edit_state();
    }
    fn eff_indent_forward(&mut self, _: Key) -> String {
        if self.visual_range().is_none() {
            self.indent_forward(self.rb.cursor.row);
            return INIT.to_owned()
        }
        let vr = self.visual_range().unwrap();
        for row in vr.start.row .. vr.end.row+1 {
            self.indent_forward(row);
        }
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
    fn eff_enter_replace_once_mode(&mut self, _: Key) -> String {
        REPLACE_ONCE.to_owned()
    }
    fn eff_cancel_replace_once_mode(&mut self, _: Key) -> String {
        INIT.to_owned()
    }
    fn eff_commit_replace_once(&mut self, k: Key) -> String {
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
                    self.create_edit_state(&delete_range, vec![BufElem::Char(c)], vec![]);
                    self.commit_edit_state();
                    INIT.to_owned()
                }
            },
            _ => REPLACE_ONCE.to_owned()
        }
    }
    fn eff_enter_snippet_mode(&mut self, _: Key) -> String {
        let no_candidates = self.snippet_repo.current_matches().is_empty();
        if no_candidates {
            INSERT.to_owned()
        } else {
            SNIPPET.to_owned()
        }
    }
    fn eff_cursor_up_snippet_mode(&mut self, _: Key) -> String {
        self.snippet_repo.rb.cursor_up();
        SNIPPET.to_owned()
    }
    fn eff_cursor_down_snippet_mode(&mut self, _: Key) -> String {
        self.snippet_repo.rb.cursor_down();
        SNIPPET.to_owned()
    }
    fn eff_insert_snippet(&mut self, _: Key) -> String {
        self.es_mut().diff_buffer.diff_buf_raw.rollback_current_word();
        let snippet = self.snippet_repo.current_snippet();
        let mut children = vec![];
        for es in &snippet.body {
            for e in es {
                let child = match e.clone() {
                    SnippetElem::TabStop(s, order) => {
                        let order = if order == 0 { 10000000 } else { order };
                        ChildComponent::Dynamic(s.chars().map(|c| BufElem::Char(c)).collect(), order)
                    },
                    SnippetElem::Str(s) => {
                        ChildComponent::Fixed(s.chars().map(|c| BufElem::Char(c)).collect())
                    }
                };
                children.push(child);
            }
            children.push(ChildComponent::Eol);
        }
        children.pop(); // pop the last eol

        // if there is no tabstop insert one tabstop at end
        let no_dynamic = !children.iter().any(|child| match child {
            ChildComponent::Dynamic(_,_) => true,
            _ => false
        });
        if no_dynamic {
            children.push(ChildComponent::Dynamic(vec![], 10000000));
        }

        self.es_mut().diff_buffer.diff_buf_raw.add_children(children);
        self.snippet_repo.set_searcher(self.edit_state.as_mut().unwrap().diff_buffer.diff_buf_raw.current_word());
        self.restore_buf_before_writeback();
        self.writeback_edit_state();
        INSERT.to_owned()
    }
    fn eff_leave_snippet_mode(&mut self, _: Key) -> String {
        INSERT.to_owned()
    }
}

use crate::Key;
use std::cell::RefCell;
use std::rc::Rc;

use crate::controller::{PageState, Effect};
use crate::def_effect;

def_effect!(Undo, EditBuffer, eff_undo);
def_effect!(Redo, EditBuffer, eff_redo);
def_effect!(JoinNextLine, EditBuffer, eff_join_next_line);
def_effect!(EnterInsertNewline, EditBuffer, eff_enter_insert_newline);
def_effect!(EnterInsertNewlineAbove, EditBuffer, eff_enter_insert_newline_above);
def_effect!(EnterInsertMode, EditBuffer, eff_enter_insert_mode);
def_effect!(EnterInsertModeLineLast, EditBuffer, eff_enter_insert_mode_line_last);
def_effect!(EnterAppendMode, EditBuffer, eff_enter_append_mode);
def_effect!(ChangeLineTail, EditBuffer, eff_change_line_tail);
def_effect!(ChangeRange, EditBuffer, eff_change_range);
def_effect!(ChangeWord, EditBuffer, eff_change_word);
def_effect!(EditModeInput, EditBuffer, eff_edit_mode_input);
def_effect!(LeaveEditMode, EditBuffer, eff_leave_edit_mode);
def_effect!(DeleteLineTail, EditBuffer, eff_delete_line_tail);
def_effect!(DeleteLine, EditBuffer, eff_delete_line);
def_effect!(DeleteRange, EditBuffer, eff_delete_range);
def_effect!(DeleteWord, EditBuffer, eff_delete_word);
def_effect!(DeleteChar, EditBuffer, eff_delete_char);
def_effect!(CancelWillMode, EditBuffer, eff_cancel_will_mode);
def_effect!(Paste, EditBuffer, eff_paste);
def_effect!(PasteAbove, EditBuffer, eff_paste_above);
def_effect!(PasteSystem, EditBuffer, eff_paste_system);
def_effect!(YankRange, EditBuffer, eff_yank_range);
def_effect!(YankLine, EditBuffer, eff_yank_line);
def_effect!(IndentBack, EditBuffer, eff_indent_back);
def_effect!(IndentForward, EditBuffer, eff_indent_forward);
def_effect!(EnterVisualMode, EditBuffer, eff_enter_visual_mode);
def_effect!(Reset, EditBuffer, eff_reset);

def_effect!(EnterReplaceOnceMode, EditBuffer, eff_enter_replace_once_mode);
def_effect!(CancelReplaceOnceMode, EditBuffer, eff_cancel_replace_once_mode);
def_effect!(CommitReplaceOnce, EditBuffer, eff_commit_replace_once);

def_effect!(EnterCommandMode, EditBuffer, eff_enter_command_mode);
def_effect!(CancelCommandMode, EditBuffer, eff_cancel_command_mode);
def_effect!(ExecuteCommand, EditBuffer, eff_execute_command);

def_effect!(EnterSnippetMode, EditBuffer, eff_enter_snippet_mode);
def_effect!(InsertSnippet, EditBuffer, eff_insert_snippet);
def_effect!(LeaveSnippetMode, EditBuffer, eff_leave_snippet_mode);
def_effect!(CursorUpSnippetMode, EditBuffer, eff_cursor_up_snippet_mode);
def_effect!(CursorDownSnippetMode, EditBuffer, eff_cursor_down_snippet_mode);

use crate::shared::AsRefMut;
use crate::controller;
pub fn mk_controller(x: Rc<RefCell<EditBuffer>>) -> controller::ControllerFSM {
    use crate::Key::*;
    let mut g = controller::Graph::new();

    let y = x.clone().map(|a| &mut a.rb);
    read_buffer::add_edges(&mut g, y);

    g.add_edge(INIT, Char('v'), Rc::new(EnterVisualMode(x.clone())));
    g.add_edge(INIT, Char('D'), Rc::new(DeleteLineTail(x.clone())));
    g.add_edge(INIT, Char('d'), Rc::new(DeleteRange(x.clone())));
    g.add_edge(INIT, Char('x'), Rc::new(DeleteChar(x.clone())));
    g.add_edge(INIT, Char('<'), Rc::new(IndentBack(x.clone())));
    g.add_edge(INIT, Char('>'), Rc::new(IndentForward(x.clone())));
    g.add_edge(INIT, Char('J'), Rc::new(JoinNextLine(x.clone())));
    g.add_edge(INIT, Char('o'), Rc::new(EnterInsertNewline(x.clone())));
    g.add_edge(INIT, Char('O'), Rc::new(EnterInsertNewlineAbove(x.clone())));
    g.add_edge(INIT, Char('i'), Rc::new(EnterInsertMode(x.clone())));
    g.add_edge(INIT, Char('A'), Rc::new(EnterInsertModeLineLast(x.clone())));
    g.add_edge(INIT, Char('a'), Rc::new(EnterAppendMode(x.clone())));
    g.add_edge(INIT, Char('C'), Rc::new(ChangeLineTail(x.clone())));
    g.add_edge(INIT, Char('c'), Rc::new(ChangeRange(x.clone())));
    g.add_edge(INIT, Char('p'), Rc::new(Paste(x.clone())));
    g.add_edge(INIT, Char('P'), Rc::new(PasteAbove(x.clone())));
    g.add_edge(INIT, Ctrl('p'), Rc::new(PasteSystem(x.clone())));
    g.add_edge(INIT, Char('y'), Rc::new(YankRange(x.clone())));
    g.add_edge(INIT, Esc, Rc::new(Reset(x.clone())));

    g.add_edge(INIT, Char('r'), Rc::new(EnterReplaceOnceMode(x.clone())));
    g.add_edge(REPLACE_ONCE, Esc, Rc::new(CancelReplaceOnceMode(x.clone())));
    g.add_edge(REPLACE_ONCE, Otherwise, Rc::new(CommitReplaceOnce(x.clone())));
    
    g.add_edge(WILL_YANK, Char('y'), Rc::new(YankLine(x.clone())));
    g.add_edge(WILL_YANK, Esc, Rc::new(CancelWillMode(x.clone())));

    g.add_edge(WILL_DELETE, Char('d'), Rc::new(DeleteLine(x.clone())));
    g.add_edge(WILL_DELETE, Char('w'), Rc::new(DeleteWord(x.clone())));
    g.add_edge(WILL_DELETE, Esc, Rc::new(CancelWillMode(x.clone())));

    g.add_edge(WILL_CHANGE, Char('w'), Rc::new(ChangeWord(x.clone())));
    g.add_edge(WILL_CHANGE, Esc, Rc::new(CancelWillMode(x.clone())));

    g.add_edge(INSERT, Ctrl('s'), Rc::new(EnterSnippetMode(x.clone())));
    g.add_edge(INSERT, Esc, Rc::new(LeaveEditMode(x.clone())));
    g.add_edge(INSERT, Otherwise, Rc::new(EditModeInput(x.clone())));

    g.add_edge(INIT, Ctrl('r'), Rc::new(Redo(x.clone())));
    g.add_edge(INIT, Char('u'), Rc::new(Undo(x.clone())));

    // completion
    g.add_edge(SNIPPET, Char('k'), Rc::new(CursorUpSnippetMode(x.clone())));
    g.add_edge(SNIPPET, Char('j'), Rc::new(CursorDownSnippetMode(x.clone())));
    g.add_edge(SNIPPET, Char('\n'), Rc::new(InsertSnippet(x.clone())));
    g.add_edge(SNIPPET, Esc, Rc::new(LeaveSnippetMode(x.clone())));

    g.add_edge(INIT, Char(' '), Rc::new(EnterCommandMode(x.clone())));
    g.add_edge(COMMAND, Esc, Rc::new(CancelCommandMode(x.clone())));
    g.add_edge(COMMAND, Otherwise, Rc::new(ExecuteCommand(x.clone())));

    controller::ControllerFSM::new(INIT, Box::new(g))
}

use crate::view;
pub struct VisualRangeDiffView {
    range: Option<CursorRange>, // doubtful design to have option here
}
impl view::View for VisualRangeDiffView {
    fn get(&self, col: usize, row: usize) -> view::ViewElem {
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

pub struct EbViewGen {
    buf: Rc<RefCell<EditBuffer>>,
}
impl EbViewGen {
    pub fn new(buf: Rc<RefCell<EditBuffer>>) -> Self {
        Self { buf: buf }
    }
}
fn compute_snippet_area(area: &Area, cursor: &Cursor, w: usize, h: usize) -> Area {
    assert!(w>0);
    assert!(h>0);

    if cursor.row >= h {
        let area0 = Area { col: cursor.col+1, row: cursor.row-h, width: w, height: h };
        if area.contains_area(&area0) { return area0 }
    }

    let area1 = Area { col: cursor.col+1, row: cursor.row+1, width: w, height: h };
    if area.contains_area(&area1) { return area1 }

    if cursor.col >= w && cursor.row >= h {
        let area2 = Area { col: cursor.col-w, row: cursor.row-h, width: w, height: h };
        if area.contains_area(&area2) { return area2 }
    }

    if cursor.col >= w {
        let area3 = Area { col: cursor.col-w, row: cursor.row+1, width: w, height: h };
        if area.contains_area(&area3) { return area3 }
    }
    
    area1
}
fn gen_impl(buf_ref: &mut EditBuffer, region: view::Area) -> Box<view::View> {
    let (lineno_reg, buf_reg) = region.split_horizontal(view::LINE_NUMBER_W);

    buf_ref.rb.stabilize_cursor();
    buf_ref.rb.adjust_window(buf_reg.width, buf_reg.height);
    buf_ref.update_cache();

    let lineno_range = buf_ref.rb.lineno_range();
    let lineno_view = view::LineNumber {
        from: lineno_range.start + 1,
        to: lineno_range.end,
    };
    let lineno_view =
        view::TranslateView::new(lineno_view, lineno_reg.col as i32, lineno_reg.row as i32);

    // let buf_view = view::ToView::new(self.buf.borrow().rb.buf.clone());
    
    let buf_window = buf_ref.rb.current_window();
    let buf_view = view::ToView::new(&buf_ref.rb.buf);

    // let buf_view = view::ToView::new(&self.buf.borrow().rb.buf, buf_window);
    // let highlight_diff = highlight::HighlightDiffView::new(&self.buf.borrow().highlighter, buf_window);
    let buf_view = view::OverlayView::new(
        buf_view,
        highlight::HighlightDiffViewRef::new(&buf_ref.highlighter),
    );

    let buf_view = view::OverlayView::new(
        buf_view,
        view::BufArea::new(&buf_ref.rb.buf).map(|e0| {
            if e0 == &BufElem::Char('\t') {
                (Some(' '), None, Some(Color::Red))
            } else {
                (None, None, None)
            }
        })
    );

    let buf_view = view::OverlayView::new(
        buf_view,
        search::DiffView::new(&buf_ref.rb.search),
    );
    
    let buf_view = view::OverlayView::new(
        buf_view,
        VisualRangeDiffView::new(buf_ref.visual_range()),
    );

    let snippet_view = {
        if buf_ref.snippet_repo.current_matches().is_empty() {
            None
        } else {
            let cursor = buf_ref.rb.cursor;
            let buf_area = Area { row: 0, col: 0, width: buf_reg.width, height: buf_reg.height };
            let snippet_area = compute_snippet_area(&buf_area, &cursor, 15, buf_ref.snippet_repo.current_matches().len());
            let mut view_gen = snippet::SnippetViewGen::new(&mut buf_ref.snippet_repo);
            Some(view_gen.gen(snippet_area))
        }
    };
    let buf_view: Box<view::View> = match snippet_view {
        None => Box::new(buf_view),
        Some(v) => Box::new(view::OverlayView::new(buf_view, v))
    };

    let add_cursor = view::AddCursor::new(buf_ref.rb.cursor);
    let hide_buf_cursor = buf_ref.state.get() == SNIPPET;
    let add_cursor = view::EnableView::new(add_cursor, !hide_buf_cursor);
    let buf_view = view::OverlayView::new(buf_view, add_cursor);

    let buf_view = view::TranslateView::new(
        buf_view,
        buf_reg.col as i32 - buf_ref.rb.window.col() as i32,
        buf_reg.row as i32 - buf_ref.rb.window.row() as i32,
    );
    
    let view = view::MergeHorizontal {
        left: lineno_view,
        right: buf_view,
        col_offset: buf_reg.col,
    };

    let view = view::CloneView::new(view, region);
    Box::new(view)
}
impl ViewGen for EbViewGen {
    fn gen(&mut self, area: view::Area) -> Box<view::View> {
        gen_impl(&mut self.buf.borrow_mut(), area)
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
            view_gen: Box::new(EbViewGen::new(x.clone())),
            x: x,
        }
    }
}
impl navigator::Page for Page {
    fn controller(&self) -> &Box<controller::Controller> {
        &self.controller
    }
    fn view_gen(&mut self) -> &mut Box<view::ViewGen> {
        &mut self.view_gen
    }
    fn status(&self) -> String {
        let state: &str = match self.x.borrow().state.get().as_str() {
            read_buffer::INIT => "*",
            read_buffer::SEARCH => "/",
            COMMAND => ":",
            REPLACE_ONCE => "r",
            WARP => "w",
            WILL_DELETE => "d",
            WILL_CHANGE => "c",
            WILL_YANK => "y",
            INSERT => "i",
            SNIPPET => "s",
            _ => "*",
        };
        let dirty_mark = if self.x.borrow().is_dirty() {
            "[D] "
        } else {
            ""
        };
        let path = self.x.borrow().path.to_str().unwrap().to_owned();
        format!("[Buffer -{}-] {}{}", state, dirty_mark, path)
    }
    fn kind(&self) -> navigator::PageKind {
        navigator::PageKind::Buffer
    }
    fn id(&self) -> String {
        self.x.borrow().path.to_str().unwrap().to_owned()
    }
    fn message(&self) -> MessageBox {
        self.x.borrow().message_box.clone()
    }
}

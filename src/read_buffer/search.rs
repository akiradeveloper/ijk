use crate::view;
use crate::screen;
use super::{Cursor, BufElem};
use crate::message_box::MessageBox;

#[derive(Clone, Debug)]
/// invariant: L(search_word) == L(results)
struct CacheLine {
    search_word: Vec<char>,
    results: Vec<Vec<usize>>
}
fn eq(e: &BufElem, c: char) -> bool {
    match e {
        &BufElem::Eol => false,
        &BufElem::Char(x) => x.to_string().to_lowercase() == c.to_string().to_lowercase()
    }
}
impl CacheLine {
    fn new() -> Self {
        CacheLine {
            search_word: vec![],
            results: vec![],
        }
    }
    fn rollback_search(&mut self, new_search_word: &[char]) -> usize {
        let mut i = 0;
        while self.search_word.len() > i && new_search_word.len() > i && self.search_word[i] == new_search_word[i] {
            i += 1;
        }
        let n_drop = if self.search_word.len() > i {
            self.search_word.len() - i
        } else {
            0
        };
        for _ in 0..n_drop {
            self.search_word.pop();
            self.results.pop();
        }
        i
    }

    fn inc_search(&mut self, new_c: char, line: &[BufElem]) {
        let mut v = vec![];
        let n_sw = self.search_word.len();
        if n_sw == 0 {
            for (i, e) in line.iter().enumerate() {
                if eq(e, new_c) {
                    v.push(i)
                }
            }
        } else {
            let last = &self.results[n_sw-1];
            let n = self.search_word.len();
            for i in last {
                // as hit eol once the hit is removed
                // this code is safe without the care for out of boundary case.
                if eq(&line[i+n], new_c) {
                    v.push(*i);
                }
            }
        }
        self.search_word.push(new_c);
        self.results.push(v);
    }
    fn result(&self) -> Vec<usize> {
        let n_sw = self.search_word.len();
        if n_sw == 0 {
            vec![]
        } else {
            self.results[n_sw-1].clone()
        }
    }
    fn next(&self, i: Option<usize>) -> Option<usize> {
        match i {
            Some(i) => {
                self.result().into_iter().find(|j| *j > i)
            },
            None => self.result().first().cloned()
        }
    }
    fn prev(&self, i: Option<usize>) -> Option<usize> {
        match i {
            Some(i) => {
                self.result().into_iter().rev().find(|j| *j < i)
            },
            None => self.result().last().cloned()
        }
    }
}

#[test]
fn test_hit() {
    use super::BufElem::*;
    let mut hit = CacheLine::new();
    assert_eq!(hit.rollback_search(&[]), 0);
    assert!(hit.result().is_empty());

    let line = [Char('a'),Char('b'),Char('a'),Char('b'),Char('a'),Eol];
    let sw = ['a','b','a','b'];
    hit.inc_search('a', &line);
    dbg!(&hit);
    assert_eq!(hit.result(), &[0,2,4]);
    hit.inc_search('b', &line);
    dbg!(&hit);
    assert_eq!(hit.result(), &[0,2]);
    hit.inc_search('a', &line);
    dbg!(&hit);
    assert_eq!(hit.result(), &[0,2]);
    hit.inc_search('b', &line);
    assert_eq!(hit.result(), &[0]);

    assert_eq!(hit.rollback_search(&['a']), 1);
    assert_eq!(hit.result(), &[0,2,4]);

    assert_eq!(hit.rollback_search(&[]), 0);
    assert!(hit.result().is_empty());
}

#[derive(Clone)]
pub struct Search {
    pub cur_word: Vec<char>,
    cache: Vec<CacheLine>,
    message_box: MessageBox,
    show: bool,
}
impl Search {
    pub fn new(n_rows: usize, message_box: MessageBox) -> Self {
        Self {
            cur_word: vec![],
            cache: vec![CacheLine::new(); n_rows],
            show: false,
            message_box,
        }
    }
    pub fn hide_search(&mut self) {
        self.show = false;
    }
    pub fn show_search(&mut self) {
        self.show = true;
    }
    pub fn clear_search_word(&mut self) {
        self.cur_word.clear();
        self.show_search_word();
    }
    fn show_search_word(&self) {
        if self.cur_word.is_empty() {
            self.message_box.send("");
        } else {
            let mut x = String::new();
            for c in &self.cur_word {
                x.push(*c)
            }
            self.message_box.send(&x);
        }
    }
    pub fn push_search_word(&mut self, c: char) {
        self.cur_word.push(c);
        self.show_search_word();
    }
    pub fn pop_search_word(&mut self) {
        self.cur_word.pop();
        self.show_search_word();
    }
    fn restruct_cache(&mut self, row: usize, deleted: usize, inserted: usize) {
        for _ in 0..deleted {
            self.cache.remove(row);
        }
        for _ in 0..inserted {
            self.cache.insert(row, CacheLine::new());
        }
    }
    pub fn cache_insert_new_line(&mut self, row: usize) {
        self.cache.insert(row, CacheLine::new());
    }
    pub fn cache_remove_line(&mut self, row: usize) {
        self.cache.remove(row);
    }
    // tmp: instead of diff update
    // slow version. clear the data on every change
    pub fn clear_cache(&mut self, n_rows: usize) {
        self.cache = vec![CacheLine::new(); n_rows];
    }
    fn update_cache_line(&mut self, row: usize, buf: &[Vec<BufElem>]) {
        let n = self.cache[row].rollback_search(&self.cur_word);
        // if L(cur_word) == n this slice is empty
        for c in &self.cur_word[n..] {
            self.cache[row].inc_search(*c, &buf[row]);
        }
    }
    /// ensure:
    /// L(this) == L(buf)
    pub fn update_cache(&mut self, range: std::ops::Range<usize>, buf: &[Vec<BufElem>]) {
        for row in range {
            self.update_cache_line(row, buf)
        }
    }
    pub fn next(&mut self, cur: Cursor, buf: &[Vec<BufElem>]) -> Option<Cursor> {
        match self.cache[cur.row].next(Some(cur.col)) {
            Some(next_col) => Some(Cursor { row: cur.row, col: next_col }),
            None => {
                let mut search_rows = vec![];
                for i in cur.row+1 .. self.cache.len() {
                    search_rows.push(i);
                }
                for i in 0 .. cur.row+1 {
                    search_rows.push(i);
                }

                search_rows.into_iter().map(|row| {
                    self.update_cache_line(row, buf);
                    let first0 = self.cache[row].next(None);
                    match first0 {
                        Some(first) => Some(Cursor { row: row, col: first }),
                        None => None,
                    }
                }).find(|x| x.is_some()).unwrap_or(None)
            }
        }
    }
    pub fn prev(&mut self, cur: Cursor, buf: &[Vec<BufElem>]) -> Option<Cursor> {
        match self.cache[cur.row].prev(Some(cur.col)) {
            Some(prev_col) => Some(Cursor { row: cur.row, col: prev_col }),
            None => {
                let mut search_rows = vec![];
                for i in (0..cur.row).rev() {
                    search_rows.push(i);
                }
                for i in (cur.row..self.cache.len()).rev() {
                    search_rows.push(i);
                }
                search_rows.into_iter().map(|row| {
                    self.update_cache_line(row, buf);
                    let last0 = self.cache[row].prev(None);
                    match last0 {
                        Some(last) => Some(Cursor { row: row, col: last }),
                        None => None,
                    }
                }).find(|x| x.is_some()).unwrap_or(None)
            }
        }
    }
}

pub struct DiffView {
    model: Search,
}
impl DiffView {
    pub fn new(search: Search) -> Self {
        Self {
            model: search
        }
    }
}
impl view::DiffView for DiffView {
    fn get(&self, col: usize, row: usize) -> view::ViewElemDiff {
        let search_word_len = self.model.cur_word.len();
        if row >= self.model.cache.len() {
            (None, None, None)
        } else if self.model.cache[row].result().iter().any(|&s| s <= col && col < s+search_word_len) {
            if self.model.show {
                (None, None, Some(screen::Color::Green))
            } else {
                (None, None, None)
            }
        } else {
            (None, None, None)
        }
    }
}
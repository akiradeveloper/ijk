use crate::view;
use crate::screen;
use crate::{Cursor, BufElem, ChangeLog};
use std::ops::Range;

#[derive(Clone, Debug)]
/// invariant: L(search_word) == L(results)
struct Hit {
    search_word: Vec<char>,
    results: Vec<Vec<usize>>
}

impl Hit {
    fn new() -> Self {
        Self {
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
                if *e == BufElem::Char(new_c) {
                    v.push(i)
                }
            }
        } else {
            let last = &self.results[n_sw-1];
            let n = self.search_word.len();
            for i in last {
                if line[i+n] == BufElem::Char(new_c) {
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
    use crate::BufElem::*;
    let mut hit = Hit::new();
    assert_eq!(hit.rollback_search(&[]), 0);
    assert_eq!(hit.result(), &[]);

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
    assert_eq!(hit.result(), &[]);
}
#[derive(PartialEq, Debug)]
enum AffectRange {
    Empty,
    Mid(usize),
    EndEol(usize),
}
#[derive(Clone)]
pub struct Search {
    pub cur_word: Vec<char>,
    hits: Vec<Hit>,
}
impl Search {
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
    fn calc_n_rows_affected(deleted: &[BufElem], inserted: &[BufElem]) -> (usize, usize) {
        use crate::search::AffectRange::*;
        match (Self::affect_range_of(deleted), Self::affect_range_of(inserted)) {
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
    pub fn new(n_rows: usize) -> Self {
        Self {
            cur_word: vec![],
            hits: vec![Hit::new(); n_rows],
        }
    }
    pub fn push_search_word(&mut self, c: char) {
        self.cur_word.push(c);
    }
    pub fn pop_search_word(&mut self) {
        self.cur_word.pop();
    }
    // TODO
    // optimized version
    pub fn update_struct(&mut self, log: &ChangeLog) {
        let (deleted, inserted) = Self::calc_n_rows_affected(&log.deleted, &log.inserted);
        for _ in 0..deleted {
            self.hits.remove(log.at.row);
        }
        for _ in 0..inserted {
            self.hits.insert(log.at.row, Hit::new());
        }
    }
    // tmp: instead of update
    // slow version. clear the data on every change
    pub fn clear_struct(&mut self, n_rows_after_change: usize) {
        self.hits = vec![Hit::new(); n_rows_after_change];
    }
    /// ensure:
    /// L(this) == L(buf)
    pub fn update_results(&mut self, range: std::ops::Range<usize>, buf: &[Vec<BufElem>]) {
        for i in range {
            let n = self.hits[i].rollback_search(&self.cur_word);
            // if L(cur_word) == n this slice is empty
            for c in &self.cur_word[n..] {
                self.hits[i].inc_search(*c, &buf[i]);
            }
        }
    }
    pub fn next(&self, cur: Cursor) -> Option<Cursor> {
        match self.hits[cur.row].next(Some(cur.col)) {
            Some(next_col) => Some(Cursor { row: cur.row, col: next_col }),
            None => {
                (cur.row+1..self.hits.len()).map(|row| {
                    let first0 = self.hits[row].next(None);
                    match first0 {
                        Some(first) => Some(Cursor { row: row, col: first }),
                        None => None,
                    }
                }).find(|x| x.is_some()).unwrap_or(None)
            }
        }
    }
    pub fn prev(&self, cur: Cursor) -> Option<Cursor> {
        match self.hits[cur.row].prev(Some(cur.col)) {
            Some(prev_col) => Some(Cursor { row: cur.row, col: prev_col }),
            None => {
                (0..cur.row).rev().map(|row| {
                    let last0 = self.hits[row].prev(None);
                    match last0 {
                        Some(last) => Some(Cursor { row: row, col: last }),
                        None => None,
                    }
                }).find(|x| x.is_some()).unwrap_or(None)
            }
        }
    }
}

#[test]
fn test_affect_range() {
    use crate::search::AffectRange::*;
    use crate::BufElem::*;
    assert_eq!(Search::affect_range_of(&[]), Empty);
    assert_eq!(Search::affect_range_of(&[Char(' ')]), Mid(1));
    assert_eq!(Search::affect_range_of(&[Char(' '),BufElem::Eol]), EndEol(1));
    assert_eq!(Search::affect_range_of(&[Char(' '),BufElem::Eol,Char('a')]), Mid(2));
    assert_eq!(Search::affect_range_of(&[Char(' '),BufElem::Eol,Char('a'),Eol]), EndEol(2));
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
        if row >= self.model.hits.len() {
            (None, None, None)
        } else if self.model.hits[row].result().iter().any(|&s| s <= col && col < s+search_word_len) {
            (None, None, Some(screen::Color::Green))
        } else {
            (None, None, None)
        }
    }
}
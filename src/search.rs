use crate::view;
use crate::screen;
use crate::{BufElem, ChangeLog};
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
    fn hits_pos(&self) -> Vec<usize> {
        let n_sw = self.search_word.len();
        if n_sw == 0 {
            vec![]
        } else {
            self.results[n_sw-1].clone()
        }
    }
}

#[test]
fn test_hit() {
    use crate::BufElem::*;
    let mut hit = Hit::new();
    let line = [Char('a'),Char('b'),Char('a'),Char('b'),Char('a'),Eol];
    let sw = ['a','b','a','b'];
    hit.inc_search('a', &line);
    dbg!(&hit);
    assert_eq!(hit.hits_pos(), &[0,2,4]);
    hit.inc_search('b', &line);
    dbg!(&hit);
    assert_eq!(hit.hits_pos(), &[0,2]);
    hit.inc_search('a', &line);
    dbg!(&hit);
    assert_eq!(hit.hits_pos(), &[0,2]);
    hit.inc_search('b', &line);
    assert_eq!(hit.hits_pos(), &[0]);

    assert_eq!(hit.rollback_search(&['a']), 1);
    assert_eq!(hit.hits_pos(), &[0,2,4]);
}

pub struct Search {
    cur_word: Vec<char>,
    hits: Vec<Hit>,
}
impl Search {
    pub fn new() -> Self {
        Self {
            cur_word: vec![],
            hits: vec![],
        }
    }
    fn cur_gen(&self) -> usize {
        self.cur_word.len() - 1
    }
    pub fn push_search_word(&mut self, c: char) {
        self.cur_word.push(c);
    }
    pub fn pop_search_word(&mut self) {
        self.cur_word.pop();
    }
    pub fn update(&mut self, log: ChangeLog) {

    }
    /// ensure:
    /// L(this) == L(buf)
    pub fn refresh_search(&mut self, range: std::ops::Range<usize>, buf: &[Vec<BufElem>]) {
        for i in range {
            let n = self.hits[i].rollback_search(&self.cur_word);
            // if L(cur_word) == n this slice is empty
            for c in &self.cur_word[n..] {
                self.hits[i].inc_search(*c, &buf[i]);
            }
        }
    }
}

struct SearchView<'a> {
    model: &'a Search,
}

impl <'a> view::DiffView for SearchView<'a> {
   fn get(&self, col: usize, row: usize) -> view::ViewElemDiff {
        let search_word_len = self.model.cur_word.len();
        if self.model.hits[row].hits_pos().iter().any(|&s| s <= col && col < s+search_word_len) {
            (None, None, Some(screen::Color::Green))
        } else {
            (None, None, None)
        }
    }
}
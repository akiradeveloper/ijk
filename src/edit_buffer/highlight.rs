use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Style};
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use crate::BufElem;
use crate::view;

pub struct Highlight {
    buf: Vec<Vec<BufElem>>,
    cache: Vec<Option<Style>>, // L(buf) == L(cache)
}
impl Highlight {
    pub fn new() -> Self {
        Self {
            buf: vec![],
            cache: vec![],
        }
    }
    pub fn update_buffer(&mut self, row: usize, n_deleted: usize, n_inserted: &[Vec<BufElem>]) {

    }
    pub fn update_highlight(&mut self, range: std::ops::Range<usize>) {

    }
}

pub struct HighlightDiffView {
}
impl HighlightDiffView {
    pub fn new(x: &Highlight, area: view::Area) -> Self {
        Self {}
    }
}
impl view::DiffView for HighlightDiffView {
    fn get(&self, col: usize, row: usize) -> view::ViewElemDiff {
        unimplemented!()
    }
}
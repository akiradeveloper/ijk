use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Style, Color};
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use crate::BufElem;
use crate::view;
use crate::screen;

pub struct Highlight {
    buf: Vec<Vec<BufElem>>,
    cache: Vec<Vec<Style>>, // L(buf) == L(cache)
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

fn conv(c: Color) -> screen::Color {
    screen::Color::Rgb(c.r, c.g, c.b)
}
pub struct HighlightDiffView {
    buf_area: view::BufArea<Style>,
    bg_default: Color,
}
impl HighlightDiffView {
    pub fn new(x: &Highlight, area: view::Area) -> Self {
        let buf_area = view::BufArea::new(&x.cache, area);
        let bg_default = buf_area.last_some().background;
        Self { buf_area, bg_default }
    }
}
impl view::DiffView for HighlightDiffView {
    fn get(&self, col: usize, row: usize) -> view::ViewElemDiff {
        match self.buf_area.get(col, row) {
            Some(style) => {
                let fg = conv(style.foreground);
                let bg = conv(style.background);
                (None, Some(fg), Some(bg))
            },
            None => {
                let bg = conv(self.bg_default);
                (None, None, Some(bg))
            },
        }
    }
}
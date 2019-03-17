use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Style, Color};
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use crate::BufElem;
use crate::view;
use crate::screen;

use lazy_static::lazy_static;
lazy_static! {
    static ref ts: ThemeSet = ThemeSet::load_defaults();
    static ref ps: SyntaxSet = SyntaxSet::load_defaults_newlines();
}

pub struct Highlighter {
    cache: Vec<Vec<Style>>, // L(buf) == L(cache)
    highlighter: HighlightLines<'static>,
}
impl Highlighter {
    pub fn new(n_rows: usize) -> Self {
        let syntax = ps.find_syntax_by_extension("rs").unwrap();
        Self {
            cache: vec![vec![]; n_rows],
            highlighter: HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]),
        }
    }
    pub fn cache_insert_new_line(&mut self, row: usize) {
        self.cache.insert(row, vec![]);
    }
    pub fn cache_remove_line(&mut self, row: usize) {
        self.cache.remove(row);
    }
    fn restruct_cache(&mut self, row: usize, n_deleted: usize, n_inserted: usize) {
        panic!()
    }
    // tmp
    pub fn clear_cache(&mut self, n_rows: usize) {
        self.cache = vec![vec![]; n_rows];
    }
    fn update_highlight_line(&mut self, row: usize, buf: &[Vec<BufElem>]) {
        if !self.cache[row].is_empty() {
            return;
        }
        let mut s = String::new();
        for e in &buf[row] {
            let c = match *e {
                BufElem::Char(c) => c,
                BufElem::Eol => '\n'
            };
            s.push(c);
        }

        let highlight_result = self.highlighter.highlight(&s, &ps);

        let mut v = vec![];
        for (style, s) in highlight_result {
            for _ in 0 .. s.len() {
                v.push(style);
            }
        }
        self.cache[row] = v;
    }
    pub fn update_cache(&mut self, row_range: std::ops::Range<usize>, buf: &[Vec<BufElem>]) {
        for row in row_range {
            self.update_highlight_line(row, &buf);
        }
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
    pub fn new(x: &Highlighter, area: view::Area) -> Self {
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
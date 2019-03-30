use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Style, Color};
use crate::read_buffer::BufElem;
use crate::view;
use crate::screen;
use crate::theme;

use lazy_static::lazy_static;
lazy_static! {
    static ref ps: SyntaxSet = SyntaxSet::load_defaults_newlines();
}

pub struct Highlighter {
    cache: Vec<Vec<Style>>, // L(buf) == L(cache)
    highlighter: HighlightLines<'static>,
}
impl Highlighter {
    pub fn new(n_rows: usize, ext: Option<&str>) -> Self {
        let syntax = ps.find_syntax_by_extension(ext.unwrap_or("rs")).unwrap_or(ps.find_syntax_plain_text());
        Self {
            cache: vec![vec![]; n_rows],
            highlighter: HighlightLines::new(syntax, theme::default()),
        }
    }

    // diff update is not implemeneted at the moment.
    // unlike search, highlighting needs a parse state rather than the indivisual line data.
    fn cache_insert_new_line(&mut self, row: usize) {
        self.cache.insert(row, vec![]);
    }
    fn cache_remove_line(&mut self, row: usize) {
        self.cache.remove(row);
    }
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

pub struct HighlightDiffViewRef<'a> {
    back: &'a Highlighter,
    bg_default: Color,
}
impl <'a> HighlightDiffViewRef<'a> {
    pub fn new(x: &'a Highlighter) -> Self {
        Self {
            back: x,
            bg_default: theme::default().settings.background.unwrap(),
        }
    }
}
impl <'a> view::View for HighlightDiffViewRef<'a> {
    fn get(&self, col: usize, row: usize) -> view::ViewElem {
        match self.back.cache.get(row).and_then(|x| x.get(col)) {
            Some(style) => {
                let fg = style.foreground.into();
                let bg = style.background.into();
                (None, Some(fg), Some(bg))
            },
            None => {
                let bg = self.bg_default.into();
                (None, None, Some(bg))
            },
        }
    }
}

pub struct HighlightDiffView {
    buf_area: view::BufArea<Style>,
    bg_default: Color,
}
impl HighlightDiffView {
    pub fn new(x: &Highlighter, area: view::Area) -> Self {
        let buf_area = view::BufArea::new(&x.cache, area);
        let bg_default = theme::default().settings.background.unwrap();
        Self { buf_area, bg_default }
    }
}
impl view::View for HighlightDiffView {
    fn get(&self, col: usize, row: usize) -> view::ViewElem {
        match self.buf_area.get(col, row) {
            Some(style) => {
                let fg = style.foreground.into();
                let bg = style.background.into();
                (None, Some(fg), Some(bg))
            },
            None => {
                let bg = self.bg_default.into();
                (None, None, Some(bg))
            },
        }
    }
}
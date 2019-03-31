mod file_parser;
mod line_parser;
mod trie;

use crate::read_buffer::{BufElem, ReadBuffer};
use crate::view;
use crate::message_box::MessageBox;

#[derive(Debug, PartialEq)]
pub enum SnippetElem {
    TabStop(String, usize),
    Str(String)
}

pub struct Snippet {
    prefix: String,
    body: Vec<SnippetElem>,
    description: String,
}

pub struct SnippetRepo {
    rb: ReadBuffer,
    current_matches: Vec<Snippet>,
}
impl SnippetRepo {
    pub fn new(ext: Option<&str>, message_box: MessageBox) -> Self {
        Self {
            rb: ReadBuffer::new(vec![vec![BufElem::Eol]], message_box),
            current_matches: vec![]
        }
    }
    pub fn set_searcher(&mut self, s: Option<Vec<char>>) {}
    pub fn current_matches(&self) -> &Vec<Snippet> { &self.current_matches }
}

struct SnippetView<'a> {
    x: &'a SnippetRepo,
}
impl <'a> view::View for SnippetView<'a> {
    fn get(&self, col: usize, row: usize) -> view::ViewElem {
        unimplemented!();
    }
}

struct SnippetViewGen<'a> {
    x: &'a SnippetRepo,
}
impl <'a> view::ViewGen for SnippetViewGen<'a> {
    fn gen(&self, area: view::Area) -> Box<view::View> {
        let view = view::ToView::new(&self.x.rb.buf);

        let add_cursor = view::AddCursor::new(self.x.rb.cursor);
        let view = view::OverlayView::new(view, add_cursor);

        let view = view::TranslateView::new(
            view,
            area.col as i32 - self.x.rb.window.col() as i32,
            area.row as i32 - self.x.rb.window.row() as i32,
        );

        let view = view::CloneView::new(view, area);
        Box::new(view)
    }
}
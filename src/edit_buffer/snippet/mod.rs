mod file_parser;
mod line_parser;
mod trie;

use crate::read_buffer::{BufElem, ReadBuffer};
use crate::view;
use crate::message_box::MessageBox;
use self::trie::Trie;

#[derive(Debug, PartialEq, Clone)]
pub enum SnippetElem {
    TabStop(String, usize),
    Str(String)
}

#[derive(Clone)]
pub struct Snippet {
    prefix: String,
    pub body: Vec<Vec<SnippetElem>>,
    description: String,
}

pub struct SnippetRepo {
    trie: Trie<Snippet>,
    pub rb: ReadBuffer,
    current_matches: Vec<Snippet>,
    message_box: MessageBox,
}
impl SnippetRepo {
    pub fn new(ext: Option<&str>, message_box: MessageBox) -> Self {
        Self {
            trie: Trie::new(),
            rb: ReadBuffer::new(vec![vec![BufElem::Eol]], message_box.clone()),
            current_matches: vec![],
            message_box: message_box,
        }
    }
    fn construct_rb(snippets: &[Snippet]) -> Vec<Vec<BufElem>> {
        let mut v = vec![];
        v
    }
    pub fn set_searcher(&mut self, s: Vec<char>) {
        let new_list = if s.is_empty() {
            vec![]
        } else {
            self.trie.get_node(&s).map(|node| {
                let mut res = vec![];
                for (k, vv) in node.list_values() {
                    for v in vv {
                        res.push(v)
                    }
                }
                res
            }).unwrap_or(vec![])
        };
        self.rb = ReadBuffer::new(Self::construct_rb(&new_list), self.message_box.clone());
        self.current_matches = new_list;
    }
    pub fn current_matches(&self) -> &Vec<Snippet> {
        &self.current_matches
    }
    pub fn current_snippet(&self) -> &Snippet {
        let pos = self.rb.cursor.row;
        &self.current_matches[pos]
    }
}

// struct SnippetView<'a> {
//     x: &'a SnippetRepo,
// }
// impl <'a> view::View for SnippetView<'a> {
//     fn get(&self, col: usize, row: usize) -> view::ViewElem {
//         unimplemented!();
//     }
// }

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
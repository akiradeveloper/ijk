mod file_parser;
mod line_parser;
mod trie;

use crate::read_buffer::{BufElem, ReadBuffer};
use crate::view;
use crate::message_box::MessageBox;
use self::trie::Trie;

const TESTDATA: &'static str = r#"{
    "for": {
        "prefix": "for",
        "body": [
        "for ${1:x} in ${2:xs} {",
        "    ${0:unimplemented!()}",
        "}"
        ],
        "description": "for loop"
    },
    "format": {
        "prefix": "format",
        "body": "format!(${1:format}, $2)",
        "description": "format!"
    },
    "assert": {
        "prefix": "assert",
        "body": "assert!(${0:true})",
        "description": "assert!"
    }
}"#;

#[derive(Debug, PartialEq, Clone)]
pub enum SnippetElem {
    TabStop(String, usize),
    Str(String)
}

#[derive(Clone, Debug)]
pub struct Snippet {
    pub prefix: String,
    pub body: Vec<Vec<SnippetElem>>,
    pub description: String,
}

pub struct SnippetRepo {
    trie: Trie<Snippet>,
    pub rb: ReadBuffer,
    current_matches: Vec<Snippet>,
    message_box: MessageBox,
}

use self::file_parser::{File, Unit};
use serde_json;
impl SnippetRepo {
    pub fn new(ext: Option<&str>, message_box: MessageBox) -> Self {
        let mut trie = Trie::new();
        let f: File = serde_json::from_str(&TESTDATA).unwrap();
        match f {
            File(units) => {
                for unit in units.values() {
                    match file_parser::convert(unit) {
                        None => {
                            // dbg!(&unit);
                        },
                        Some(snippet) => {
                            // dbg!(&snippet);
                            let k: Vec<char> = snippet.prefix.chars().collect();
                            trie.insert(&k, snippet)
                        }
                    }
                }
            }
        }

        Self {
            trie,
            rb: ReadBuffer::new(vec![vec![BufElem::Eol]], message_box.clone()),
            current_matches: vec![],
            message_box,
        }
    }
    fn construct_rb(snippets: &[Snippet]) -> Vec<Vec<BufElem>> {
        if snippets.is_empty() {
            return vec![vec![BufElem::Eol]]
        }

        let mut v = vec![];
        for snippet in snippets {
            let mut line = vec![];
            line.push(BufElem::Char('['));
            for c in snippet.prefix.chars() {
                line.push(BufElem::Char(c))
            }
            line.push(BufElem::Char(']'));
            line.push(BufElem::Char(' '));
            for c in snippet.description.chars() {
                line.push(BufElem::Char(c))
            }
            line.push(BufElem::Eol);
            
            v.push(line)
        }
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
        // dbg!(&new_list);
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

use crate::screen::Color;
pub struct AddColor {}
impl view::View for AddColor {
    fn get(&self, _: usize, _: usize) -> view::ViewElem {
        (None, Some(Color::Black), Some(Color::White))
    }
}

pub struct SnippetViewGen<'a> {
    x: &'a mut SnippetRepo,
}
impl <'a> SnippetViewGen<'a> {
    pub fn new(x: &'a mut SnippetRepo) -> Self {
        Self { x }
    }
}
impl <'a> view::ViewGen for SnippetViewGen<'a> {
    fn gen(&mut self, area: view::Area) -> Box<view::View> {
        self.x.rb.stabilize_cursor();
        self.x.rb.adjust_window(area.width, area.height);
        self.x.rb.update_cache();

        let view = view::ToView::new(&self.x.rb.buf);
        let view = view::OverlayView::new(
            view,
            AddColor {}
        );

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
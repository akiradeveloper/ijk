mod file_parser;
mod line_parser;
mod trie;

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

pub struct SnippetRepo {}
impl SnippetRepo {
    pub fn new(ext: Option<&str>) -> Self {
        Self {}
    }
    pub fn set_searcher(&mut self, s: &[char]) {}
    pub fn list_matches(&self) -> Vec<Snippet> { vec![] }
}
mod line_parser;
mod trie;

#[derive(Debug, PartialEq)]
pub enum SnippetElem {
    TabStop(String, usize),
    Str(String)
}
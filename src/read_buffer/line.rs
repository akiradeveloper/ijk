use crate::BufElem;

pub struct Line<'a> {
    line: &'a [BufElem]
}
impl <'a> Line<'a> {
    pub fn new(line: &'a [BufElem]) -> Self {
        Self { line }
    }
    pub fn first_non_space_index(&self) -> usize {
        self.line.iter().position(|c| c != &BufElem::Char(' ') && c != &BufElem::Char('\t')).unwrap()
    }
}
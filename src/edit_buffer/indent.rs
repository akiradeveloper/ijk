use crate::BufElem;

pub struct AutoIndent<'a> {
    pub line_predecessors: &'a [BufElem]
}
impl <'a> AutoIndent<'a> {
    fn current_indent(&self) -> Vec<BufElem> {
        let mut v = vec![];
        for e in self.line_predecessors {
            if *e == BufElem::Char(' ') || *e == BufElem::Char('\t') {
                v.push(e.clone());
            } else {
                break;
            }
        }
        v
    }
    fn extra_next_indent(&self) -> Vec<BufElem> {
        if self.line_predecessors.is_empty() {
            vec![]
        } else {
            let last = self.line_predecessors.last().cloned().unwrap();
            let choices = ['{', '[', '(', ':'];
            if choices.iter().any(|c| last == BufElem::Char(*c)) {
                // tmp (rust only)
                vec![BufElem::Char(' '); 4]
            } else {
                vec![]
            }
        }
    }
    pub fn next_indent(&self) -> Vec<BufElem> {
        let mut v = self.current_indent();
        v.append(&mut self.extra_next_indent());
        v
    }
}
#[test]
fn test_auto_indent() {
    use crate::BufElem::*;
    let line = [Char(' '), Char(' '), Char('a'), Char('{')];
    let ai = AutoIndent { line_predecessors: &line };
    assert_eq!(ai.next_indent(), vec![Char(' '); 6]);
}
use super::BufElem;

#[derive(Clone, Copy)]
pub enum IndentType {
    Spaces(usize),
    Tab,
}

pub fn into_bufelems(x: IndentType) -> Vec<BufElem> {
    match x {
        IndentType::Spaces(n) => vec![BufElem::Char(' '); n],
        IndentType::Tab => vec![BufElem::Char('\t')],
    }
}

pub struct AutoIndent {
    indent_type: IndentType,
    line_predecessors: Vec<BufElem>
}
impl AutoIndent {
    pub fn new(line_predecessors: &[BufElem], indent_type: IndentType) -> Self {
        Self {
            indent_type,
            line_predecessors: line_predecessors.to_vec()
        }
    }
    pub fn current_indent(&self) -> Vec<BufElem> {
        let mut v = vec![];
        for e in &self.line_predecessors {
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
                into_bufelems(self.indent_type)
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
    use super::BufElem::*;
    let line = [Char(' '), Char(' '), Char('a'), Char('{')];
    let ai = AutoIndent::new(&line, IndentType::Spaces(4));
    assert_eq!(ai.next_indent(), vec![Char(' '); 6]);
}
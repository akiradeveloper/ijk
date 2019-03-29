extern crate combine;
use combine::attempt;
use combine::parser::Parser;
use combine::parser::choice::choice;
use combine::parser::repeat::{many, many1};
use combine::parser::sequence::{between, with};
use combine::parser::char::{string, digit, alpha_num, newline, crlf, tab, space};
use combine::parser::item::{any, token, satisfy, none_of};
use combine::parser::combinator::not_followed_by;

// Simplified vscode snippet:
// tabstop = $num | ${num} | ${num:placeholder}
// elem = tabstop | str
// line = elem+

#[derive(Debug, PartialEq)]
pub enum SnippetElem {
    TabStop(String, usize),
    Str(String)
}

pub struct LineParser {}
impl LineParser {
    pub fn new() -> Self {
        Self {}
    }
    pub fn parse(&mut self, s: &str) -> Option<Vec<SnippetElem>> {
        use self::SnippetElem::*;

        let num_p = many1(digit()).map(|n: String| n.parse::<usize>().unwrap());
        let placeholder_p = satisfy(|c| c != '{' && c != '}' && c != '$');

        let tabstop_p0 = num_p.clone().map(|n: usize| TabStop("".to_owned(), n));
        let tabstop_p1 = num_p.clone().skip(token(':')).and(many1::<String, _>(placeholder_p.clone())).map(|(n, s)| TabStop(s, n));
        // $n
        let p0 = string("$").with(tabstop_p0.clone());
        // ${n}
        let p1 = string("$").with(between(token('{'),token('}'),tabstop_p0.clone()));
        // ${n:s}
        let p2 = string("$").with(between(token('{'),token('}'),tabstop_p1.clone()));
        let tabstop_p = attempt(p0).or(attempt(p1)).or(p2);

        let char_p = placeholder_p.clone().or(token('{')).or(token('}'));
        let str_p = many1(char_p).map(|s| Str(s));

        let elem_p = attempt(tabstop_p).or(str_p);

        let mut line_p = many1::<Vec<SnippetElem>, _>(elem_p).skip(not_followed_by(any())); // must consume all
        line_p.parse(s).ok().map(|x| x.0)
    }
}

#[test]
fn test_line_parser() {
    use self::SnippetElem::*;

    let mut parser = self::LineParser::new();
    assert_eq!(parser.parse("for (const ${2:element} of ${1:array}) {").unwrap(), vec![Str("for (const ".to_owned()),TabStop("element".to_owned(),2),Str(" of ".to_owned()),TabStop("array".to_owned(),1),Str(") {".to_owned())]);
    assert_eq!(parser.parse("fn a(&self) -> Vec<String>").unwrap(), vec![Str("fn a(&self) -> Vec<String>".to_owned())]);
    assert_eq!(parser.parse("a =+> b * <<-^ c").unwrap(), vec![Str("a =+> b * <<-^ c".to_owned())]);
    assert_eq!(parser.parse("$0ab c").unwrap(), vec![TabStop("".to_owned(),0),Str("ab c".to_owned())]);
    assert_eq!(parser.parse("${13}abc").unwrap(), vec![TabStop("".to_owned(),13),Str("abc".to_owned())]);
    assert_eq!(parser.parse("abc$13[]").unwrap(), vec![Str("abc".to_owned()),TabStop("".to_owned(),13),Str("[]".to_owned())]);
    assert_eq!(parser.parse("abc${0}").unwrap(), vec![Str("abc".to_owned()),TabStop("".to_owned(),0)]);
    assert_eq!(parser.parse("abc{${0:hoge}}").unwrap(), vec![Str("abc{".to_owned()),TabStop("hoge".to_owned(),0),Str("}".to_owned())]); 
    assert_eq!(parser.parse("abc{${0:$1}}"), None); // nested placeholder should be an error
}
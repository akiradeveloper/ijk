extern crate combine;
use combine::attempt;
use combine::parser::Parser;
use combine::parser::repeat::{many, many1};
use combine::parser::sequence::{between, with};
use combine::parser::char::{string, digit, alpha_num, newline, crlf, tab, space};
use combine::parser::item::{any, token, satisfy, none_of};

#[derive(Debug)]
enum SnippetElem {
    TabStop(String, usize),
    Str(String)
}

#[test]
fn text_parser_experiment() {
    use self::SnippetElem::*;
    let int_p = many1(digit()).map(|n: String| n.parse::<usize>().unwrap()); // digit -> char, many1 -> Extend<char>, String = Extend<char>
    let placeholder_p = alpha_num().or(space()).or(tab()).or(token(';')).or(token('|')).or(token('(')).or(token(')'));
    let char_p = placeholder_p.clone().or(token('}')).or(token('{'));
    let tabstop_p0 = int_p.clone().map(|n: usize| TabStop("".to_owned(), n));
    let tabstop_p1 = int_p.clone().skip(token(':')).and(many1::<String, _>(placeholder_p.clone())).map(|(n, s)| TabStop(s, n));

    let p0 = string("$").with(tabstop_p0.clone());
    let p1 = string("$").with(between(token('{'),token('}'),tabstop_p0.clone()));
    let p2 = string("$").with(between(token('{'),token('}'),tabstop_p1.clone()));

    let tabstop_p = attempt(p0).or(attempt(p1)).or(p2);
    let str_p = many1(char_p).map(|s| Str(s));

    let mut elem_p = attempt(tabstop_p).or(str_p);

    dbg!(elem_p.parse("$10"));
    dbg!(elem_p.parse("${10}"));
    dbg!(elem_p.parse("${10:unimplemented();}"));
    dbg!(elem_p.parse("for {"));

    let mut line_p = many1::<Vec<SnippetElem>, _>(elem_p);
    dbg!(line_p.parse("$0abc"));
    dbg!(line_p.parse("${0}abc"));
    dbg!(line_p.parse("abc$0"));
    dbg!(line_p.parse("abc${0}"));
    dbg!(line_p.parse("abc{${0:hoge}}"));
}
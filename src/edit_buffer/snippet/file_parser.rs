use serde_derive::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
pub struct Line(String);

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Body {
    Single(Line),
    Array(Vec<Line>),
}

#[derive(Deserialize, Debug)]
pub struct Unit {
    prefix: String,
    body: Body,
    description: String,
}

#[derive(Deserialize, Debug)]
pub struct File(HashMap<String, Unit>);

use super::{Snippet, SnippetElem};
pub fn convert(unit: &Unit) -> Option<Snippet> {
    let body = convert_body(&unit.body);
    if body.is_none() {
        return None
    }
    let body = body.unwrap();
    Some(Snippet {
        prefix: unit.prefix.clone(),
        body: body,
        description: unit.description.clone(),
    })
}
fn convert_body(body: &Body) -> Option<Vec<Vec<SnippetElem>>> {
    match body {
        Body::Single(line) => convert_body_line(line).map(|v| vec![v]),
        Body::Array(lines) => { 
            let mut converted = lines.iter().map(|line| convert_body_line(line));
            if converted.any(|x| x.is_none()) {
                None
            } else {
                Some(converted.map(|x| x.unwrap()).collect())
            }
        }
    }
}
fn convert_body_line(line: &Line) -> Option<Vec<SnippetElem>> {
    match line {
        Line(s) => {
            let mut lp = super::line_parser::LineParser::new();
            lp.parse(&s)
        }
    }
}

#[test]
fn test_parse_file() {
    let data = r#"{
        "for": {
            "prefix": "for",
            "body": [
            "for (const ${2:x} of ${1:xs}) {",
            "\t${0:unimplemented!())",
            "}"
            ],
            "description": "For Loop"
        },
        "assert": {
            "prefix": "assert",
            "body": "assert!($0)",
            "description": "never use this shit"
        }
    }"#;

    let f: File = serde_json::from_str(&data).unwrap();
    dbg!(f);
}

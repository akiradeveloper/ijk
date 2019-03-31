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

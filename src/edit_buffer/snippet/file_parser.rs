use serde_derive::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
struct Line(String);

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum Body {
    Single(Line),
    Array(Vec<Line>),
}

#[derive(Deserialize, Debug)]
struct Unit {
    prefix: String,
    body: Body,
    description: String,
}

#[derive(Deserialize, Debug)]
struct File(HashMap<String, Unit>);

#[test]
fn test_parse_file() {
    let data = r#"{
        "For_Loop": {
            "prefix": "for",
            "body": [
            "for (const ${2:element} of ${1:array}) {",
            "\t$0",
            "}"
            ],
            "description": "For Loop"
        },
        "Assert": {
            "prefix": "as",
            "body": "assert!($0)",
            "description": "never use this shit"
        }
    }"#;

    let f: File = serde_json::from_str(&data).unwrap();
    dbg!(f);
}

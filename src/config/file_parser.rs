extern crate toml;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
struct ConfigFile {
    lang: Option<BTreeMap<String, LangConfig>>,
}

#[derive(Debug, Deserialize)]
struct LangConfig {
    indent: Option<usize>,
}

#[test]
fn test_file_parser() {
    let data = r#"
    [lang.rust]
    indent = 4
    [lang.ruby]
    indent = 2
    "#;

    let config: ConfigFile = toml::from_str(&data).unwrap();
    dbg!(config);
}
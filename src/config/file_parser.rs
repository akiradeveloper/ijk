extern crate toml;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
pub struct FileToml {
    pub lang: Option<BTreeMap<String, LangToml>>,
}

#[derive(Debug, Deserialize)]
pub struct LangToml {
    pub extensions: Option<Vec<String>>,
    pub filenames: Option<Vec<String>>,
    pub indent: Option<usize>,
}

#[test]
fn test_file_parser() {
    let data = r#"
    [lang.rust]
    extensions = ["rs"]
    indent = 4
    [lang.ruby]
    extensions = ["rb", "erb"]
    filenames = ["Rakefile"]
    indent = 2
    "#;

    let config: FileToml = toml::from_str(&data).unwrap();
    dbg!(config);
}
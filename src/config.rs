use std::path::{Path, PathBuf};
use crate::edit_buffer::config::Config;
use std::collections::HashMap;

#[derive(Clone, Copy, Hash, Eq, PartialEq)]
enum Lang {
    Rust,
    Ruby,
    Make,
    Toml,
}
use self::Lang::*;

const LANG_TO_STR: &[(Lang, &str)] = &[
    (Rust, "rust"),
    (Ruby, "ruby"),
    (Make, "make"),
    (Toml, "toml"),
];

const EXT_TO_LANG: &[(&str, Lang)] = &[
    ("rs", Rust),
    ("rb", Ruby),
    ("mk", Make),
    ("toml", Toml)
];

const FILENAME_TO_LANG: &[(&str, Lang)] = &[
    ("Makefile", Make),
    ("Rakefile", Ruby)
];

use crate::edit_buffer::indent::IndentType::*;

const DEFAULT_CONFIGS: &'static [(Lang, Config)] = &[
    (Rust, Config { indent_type: Spaces(4), snippet: None }),
    (Ruby, Config { indent_type: Spaces(2), snippet: None }),
];

const FALLBACK_CONFIG: Config = Config {
    indent_type: Tab,
    snippet: None,
};

use lazy_static::lazy_static;
lazy_static! {
    static ref REPO: ConfigRepo = create_config_repo();
}

struct ConfigRepo {
    configs: HashMap<Lang, Config>,
}
impl ConfigRepo {
    fn get_config(&self, lang: Lang) -> Config {
        self.configs.get(&lang).cloned().unwrap_or(FALLBACK_CONFIG)
    }
}

fn infer_lang(path: &Path) -> Option<Lang> {
    let filename0: Option<&str> = path.file_name().map(|x| x.to_str().unwrap());
    let found0: Option<Lang> = filename0.and_then(|filename| FILENAME_TO_LANG.iter().find(|p| p.0 == filename).map(|x| x.1));
    if found0.is_some() {
        return found0
    }

    let ext0: Option<&str> = path.extension().map(|x| x.to_str().unwrap());
    let found0: Option<Lang> = ext0.and_then(|ext| EXT_TO_LANG.iter().find(|p| p.0 == ext).map(|x| x.1));
    if found0.is_some() {
        return found0
    }

    None
}
fn list_snippet_files() -> Vec<(Lang, PathBuf)> {
    let home_dir = std::env::home_dir().unwrap();
    let snippet_dir = home_dir.join(".ijk").join("snippets");
    let mut v = vec![];
    for entry in std::fs::read_dir(&snippet_dir).unwrap() {
        let path = entry.unwrap().path();
        let ext0 = path.extension();
        if ext0.is_none() {
            continue;
        }
        let ext = ext0.unwrap();
        if ext != "json" {
            continue;
        }
        let fn_no_ext = path.file_stem().unwrap();
        let found = LANG_TO_STR.iter().find(|x| x.1 == fn_no_ext).map(|x| x.0);
        if found.is_none() {
            continue;
        }
        let found = found.unwrap();
        v.push((found, std::fs::canonicalize(path).unwrap()))
    }
    v
}
fn create_config_repo() -> ConfigRepo {
    let current_dir = std::env::current_dir();
    if current_dir.is_err() {
        return ConfigRepo { configs: HashMap::new() }
    }
    let current_dir = current_dir.unwrap();
    let ijk_file_path = current_dir.join(".ijk.toml");
    unimplemented!()
}
mod file_parser;
mod builder;

use std::path::{Path, PathBuf};
use crate::edit_buffer::config::Config;
use std::collections::HashMap;
use self::builder::Builder;

use crate::edit_buffer::indent::IndentType::*;

const FALLBACK_CONFIG: Config = Config {
    indent_type: Tab,
    snippet: None,
};

use lazy_static::lazy_static;
lazy_static! {
    static ref SINGLETON: ConfigRepo = create_config_repo();
}

pub type Lang = String;

pub struct ConfigRepo {
    configs: HashMap<Lang, builder::LangConfig>,
    snippets: HashMap<Lang, PathBuf>,
    extensions: HashMap<String, Lang>,
    filenames: HashMap<String, Lang>,
}
impl ConfigRepo {
    fn get_config(&self, lang: &str) -> Config {
        let fallback = Config {
            indent_type: Tab,
            snippet: None,
        };
        let indent_type = match self.configs.get(lang) {
            Some(lc) => match lc.indent {
                None => Tab,
                Some(n) => if n > 0 {
                    Spaces(n)
                } else {
                    Tab
                }
            },
            None => Tab
        };
        Config {
            indent_type,
            snippet: self.snippets.get(lang).cloned()
        }
    }
    fn infer_lang(&self, path: &Path) -> Option<Lang> {
        let filename0: Option<&str> = path.file_name().map(|x| x.to_str().unwrap());
        let found0: Option<String> = filename0.and_then(|filename| self.filenames.get(filename)).cloned();
        if found0.is_some() {
            return found0
        }

        let ext0: Option<&str> = path.extension().map(|x| x.to_str().unwrap());
        let found0: Option<String> = ext0.and_then(|ext| self.extensions.get(ext)).cloned();
        if found0.is_some() {
            return found0
        }

        None
    }
}

fn list_snippet_files() -> HashMap<Lang, PathBuf> {
    let home_dir = std::env::home_dir().unwrap();
    let snippet_dir = home_dir.join(".ijk").join("snippets");
    let mut res = HashMap::new();
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

        let fn_no_ext = path.file_stem().unwrap().to_str().unwrap().to_owned();
        res.insert(fn_no_ext, std::fs::canonicalize(path).unwrap());
    }
    res
}

fn create_config_repo() -> ConfigRepo {
    let current_dir = std::env::current_dir().unwrap();

    let mut builder = Builder::new();

    let default_config = include_str!("default.toml");
    let default_config = toml::from_str(&default_config).unwrap();
    builder.add_config_file(default_config);

    let ijk_config_path = current_dir.join(".ijk.toml");
    for s in std::fs::read_to_string(ijk_config_path) {
        let config = toml::from_str(&s).unwrap();
        builder.add_config_file(config);
    }
    
    ConfigRepo {
        configs: builder.configs,
        snippets: list_snippet_files(),
        filenames: builder.filenames,
        extensions: builder.extensions,
    }
}
use super::file_parser::{FileToml, LangToml};
use std::collections::HashMap;
use super::Lang;

pub struct LangConfig {
    pub indent: Option<usize>,
}

pub struct Builder {
    pub configs: HashMap<Lang, LangConfig>,
    pub filenames: HashMap<String, Lang>,
    pub extensions: HashMap<String, Lang>,
}
impl Builder {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
            filenames: HashMap::new(),
            extensions: HashMap::new(),
        }
    }
    fn add_lang_config(&mut self, lang: String, config: LangToml) {
        for filename in config.filenames.unwrap_or_default() {
            self.filenames.insert(filename, lang.clone());
        }
        for ext in config.extensions.unwrap_or_default() {
            self.extensions.insert(ext, lang.clone());
        }

    }
    pub fn add_config_file(&mut self, config: FileToml) {
        for m in config.lang {
            for (lang, lang_config) in m {
                self.add_lang_config(lang, lang_config)
            }
        }
    }
}
use super::indent::IndentType;
use std::path::PathBuf;

#[derive(Clone)]
pub struct Config {
    pub indent_type: IndentType,
    pub snippet: Option<PathBuf>,
}
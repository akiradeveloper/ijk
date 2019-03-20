use syntect::highlighting::{Color, Theme, ThemeSet};

use lazy_static::lazy_static;
lazy_static! {
    static ref ts: ThemeSet = ThemeSet::load_defaults();
}

pub fn default() -> &'static Theme {
    &ts.themes["base16-ocean.dark"]
}
use std::fs;
use termion::event::Key::*;
use std::path;

fn to_term_key(s: &str) -> termion::event::Key {
    match s {
        "EOL" => Char('\n'),
        c if c.starts_with('C') => Ctrl(c.chars().nth(2).unwrap()),
        c => Char(c.chars().nth(0).unwrap()),
        _ => panic!(), // other keys are not necessary in benchmark
    }
}
pub fn read_keys_file(path: &path::Path) -> Vec<Result<termion::event::Key, std::io::Error>> {
    let mut v = vec![];
    let s = fs::read_to_string(path).unwrap();
    for line in s.lines() {
        v.push(Ok(to_term_key(line)))
    }
    v.push(Ok(Ctrl('z')));
    v
}
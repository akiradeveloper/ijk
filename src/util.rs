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

struct IntervalIterator<I>{
    iter: I,
    interval_ms: u32,
}
impl <I> IntervalIterator<I> {
    pub fn new(iter: I, interval_ms: u32) -> Self {
        Self { iter, interval_ms }
    }
}
impl <I> Iterator for IntervalIterator<I> where I: Iterator {
    type Item = I::Item;
    fn next(&mut self) -> std::option::Option<<Self as std::iter::Iterator>::Item> {
        std::thread::sleep_ms(self.interval_ms);
        self.iter.next()
    }
}

#[test]
fn test_interval_iterator() {
    let v = vec![1,2,3];
    let intv_iter = IntervalIterator::new(v.into_iter(), 1000);
    for i in intv_iter {
        println!("{}", i);
    }
}
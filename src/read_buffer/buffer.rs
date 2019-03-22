use super::{BufElem, Cursor};
use std::io::Write;

type Buf = Vec<Vec<BufElem>>;

pub fn write_to_file<W: Write>(mut out: W, buf: &Buf) {
    // TODO trim the eols from the back
    for i in 0..buf.len() {
        for j in 0..buf[i].len() {
            let e = &buf[i][j];
            match *e {
                BufElem::Char(c) => write!(out, "{}", c).unwrap(),
                BufElem::Eol => writeln!(out).unwrap(),
            }
        }
    }
}

fn convert_to_bufelems(cs: Vec<char>) -> Vec<BufElem> {
    let mut r = vec![];
    for c in cs {
        r.push(BufElem::Char(c));
    }
    r.push(BufElem::Eol);
    r
}
pub fn read_from_string(s: Option<String>) -> Buf {
    s.map(|s| {
        if s.is_empty() {
            vec![vec![BufElem::Eol]]
        } else {
            s.lines()
             .map(|line| convert_to_bufelems(line.chars().collect()))
             .collect()
        }
    }).unwrap_or(vec![vec![BufElem::Eol]])
}
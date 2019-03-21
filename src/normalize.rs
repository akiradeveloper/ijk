use crate::{BufElem, Cursor};
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
pub fn read_from_string(s: Option<&str>) -> Buf {
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

pub fn normalize_cursor(cursor: Cursor, buf: &Buf) -> Cursor {
    let mut cursor = cursor;

    let first_non_empty_row_rev = buf.iter().rev().position(|line| line.len() > 1).unwrap_or(buf.len() - 1);
    let max_row = buf.len() - 1 - first_non_empty_row_rev;
    if cursor.row > max_row {
        cursor.row = max_row;
    }

    let line = &buf[cursor.row];
    assert!(!line.is_empty());
    let first_unspace_index = line.iter().position(|c| c != &BufElem::Char(' ') && c != &BufElem::Char('\t')).unwrap();
    if cursor.col < first_unspace_index {
        cursor.col = first_unspace_index;
    }

    if cursor.col > buf[cursor.row].len() - 1 {
        cursor.col = buf[cursor.row].len() - 1;
    }

    cursor
}
pub struct Result {
    pub n_delete: usize,
    pub n_insert: usize,
}

#[derive(PartialEq, Debug)]
enum AffectRange {
    Empty,
    Mid(usize),
    EndEol(usize),
}
fn affect_range_of(buf: &[BufElem]) -> AffectRange {
    if buf.is_empty() {
        return AffectRange::Empty;
    }
    let mut n = 1;
    for e in buf {
        if *e == BufElem::Eol {
            n += 1;
        }
    }
    if *buf.last().unwrap() == BufElem::Eol {
        n -= 1;
        AffectRange::EndEol(n)
    } else {
        AffectRange::Mid(n)
    }
}
#[test]
fn test_affect_range() {
    use self::AffectRange::*;
    use crate::BufElem::*;
    assert_eq!(affect_range_of(&[]), Empty);
    assert_eq!(affect_range_of(&[Char(' ')]), Mid(1));
    assert_eq!(affect_range_of(&[Char(' '),BufElem::Eol]), EndEol(1));
    assert_eq!(affect_range_of(&[Char(' '),BufElem::Eol,Char('a')]), Mid(2));
    assert_eq!(affect_range_of(&[Char(' '),BufElem::Eol,Char('a'),Eol]), EndEol(2));
}

pub fn calc_n_lines_affected(deleted: &[BufElem], inserted: &[BufElem]) -> (usize, usize) {
    use self::AffectRange::*;
    let (n_delete, n_insert) = match (affect_range_of(deleted), affect_range_of(inserted)) {
        (Empty, Empty) => (0, 0),
        (Empty, Mid(n)) => (1, n),
        (Empty, EndEol(n)) => (1, n+1),
        (Mid(n), Empty) => (n, 1),
        (Mid(n), Mid(m)) => (n, m),
        (Mid(n), EndEol(m)) => (n, m+1),
        (EndEol(n), Empty) => (n+1, 1),
        (EndEol(n), Mid(m)) => (n+1, m),
        (EndEol(n), EndEol(m)) => (n+1, m+1),
    };
    Result { n_delete, n_insert }
}
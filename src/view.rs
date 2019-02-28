use crate::BufElem;
use crate::screen::Color;
use crate::edit_buffer::Cursor;
use crate::edit_buffer::CursorRange;

#[derive(PartialEq, Clone)]
pub struct ViewRegion {
    col: usize,
    row: usize,
    pub width: usize,
    pub height: usize,
}
type ViewElem = (char, Color, Color);
type ViewElemDiff = (Option<char>, Option<Color>, Option<Color>);

pub trait ViewGen {
    fn apply(&mut self, region: &ViewRegion) -> Box<View>;
}

pub trait View {
    fn get(&self, col: usize, row: usize) -> ViewElem;
}

pub trait DiffView {
    fn get(&self, col: usize, row: usize) -> ViewElemDiff;
}

struct ToView <'a> {
    x: &'a [Vec<BufElem>]
}

impl <'a> View for ToView<'a> {
    fn get(&self, col: usize, row: usize) -> ViewElem {
        let e: &BufElem = &self.x[row][col];
        let c = match *e {
            BufElem::Char(c) => c,
            BufElem::Eol => ' '
        };
        (c, Color::White, Color::Black)
    }
}

struct OverlayView<V, D> {
    v: V,
    d: D,
}

impl <V, D> OverlayView<V, D> where V: View, D: DiffView {
    fn new(v: V, d: D) -> Self {
        Self { v, d }
    }
}

impl <V, D> View for OverlayView<V, D> where V: View, D: DiffView {
    fn get(&self, col: usize, row: usize) -> ViewElem {
        let (v0, v1, v2) = self.v.get(col, row);
        let (d0, d1, d2) = self.d.get(col, row);
        (d0.unwrap_or(v0), d1.unwrap_or(v1), d2.unwrap_or(v2))
    }
}

struct VisualRangeDiffView {
    range: CursorRange,
}
impl DiffView for VisualRangeDiffView {
    fn get(&self, col: usize, row: usize) -> ViewElemDiff {
        let as_cursor = Cursor { row, col };
        let in_visual_range = self.range.start <= as_cursor && as_cursor < self.range.end;
        if in_visual_range {
            (None, None, Some(Color::Blue))
        } else {
            (None, None, None)
        }
    }
}

struct FilterView<V> {
    x: V,
    col: usize,
    row: usize,
    width: usize,
    height: usize,
}
impl <V> View for FilterView<V> where V: View {
    fn get(&self, col: usize, row: usize) -> ViewElem {
        unimplemented!()
    }
}

struct TestDiffView {}
impl DiffView for TestDiffView {
    fn get(&self, col: usize, row: usize) -> ViewElemDiff { 
        (Some('a'), Some(Color::Red), None)
    }
}

#[test]
fn test_view_overlay() {
    let buf = vec![vec![BufElem::Eol]];
    let v0 = ToView { x: &buf };
    let d0 = TestDiffView {};
    let v1 = OverlayView { v: v0, d: d0 };

    let view: Box<dyn View> = Box::new(v1);
    let reg = ViewRegion {
        col: 0,
        row: 0,
        width: 1,
        height: 1,
    };
    let e = view.get(0,0);
    assert_eq!(e, ('a', Color::Red, Color::Black));
}
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
    fn get_cursor_pos(&self) -> Option<Cursor>;
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
    fn get_cursor_pos(&self) -> Option<Cursor> { None }
}

struct AddCursor<V> {
    x: V,
    cursor: Cursor,
}
impl <V> View for AddCursor<V> where V: View {
    fn get(&self, col: usize, row: usize) -> ViewElem { self.x.get(col, row) }
    fn get_cursor_pos(&self) -> Option<Cursor> {
        Some(self.cursor)
    }
}

struct TranslateView<V> {
    x: V,
    diff_col: i32,
    diff_row: i32,
}
impl <V> View for TranslateView<V> where V: View {
    fn get(&self, col: usize, row: usize) -> ViewElem {
        let c = (col as i32 - self.diff_col) as usize;
        let r = (row as i32 - self.diff_row) as usize;
        self.x.get(c, r)
    }
    fn get_cursor_pos(&self) -> Option<Cursor> { 
        self.x.get_cursor_pos().map(|cur|
            Cursor {
                row: (cur.row as i32 - self.diff_row) as usize,
                col: (cur.col as i32 - self.diff_col) as usize,
            }
        )
    }
}

struct MergeVertical<V1,V2> {
    a: V1,
    b: V2,
    offset_row: usize,
}
impl <V1,V2> View for MergeVertical<V1,V2> where V1: View, V2: View {
    fn get(&self, col: usize, row: usize) -> ViewElem {
        if row < self.offset_row {
            self.a.get(col, row)
        } else {
            self.b.get(col, row)
        }
    }
    fn get_cursor_pos(&self) -> Option<Cursor> {
        self.a.get_cursor_pos().or(self.b.get_cursor_pos())
    }
}

struct MergeHorizontal<V1,V2> {
    a: V1,
    b: V2,
    offset_col: usize,
}
impl <V1,V2> View for MergeHorizontal<V1,V2> where V1: View, V2: View {
    fn get(&self, col: usize, row: usize) -> ViewElem {
        if col < self.offset_col {
            self.a.get(col, row)
        } else {
            self.b.get(col, row)
        }
    }
    fn get_cursor_pos(&self) -> Option<Cursor> {
        self.a.get_cursor_pos().or(self.b.get_cursor_pos())
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
    fn get_cursor_pos(&self) -> Option<Cursor> { self.v.get_cursor_pos() }
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
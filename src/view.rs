use crate::BufElem;
use crate::screen::Color;
use crate::Cursor;
use crate::edit_buffer::CursorRange;

#[derive(PartialEq, Clone, Copy)]
pub struct Area {
    pub col: usize,
    pub row: usize,
    pub width: usize,
    pub height: usize,
}
impl Area {
    pub fn split_horizontal(&self, left_width: usize) -> (Area, Area) {
        let left = Self {
            col: self.col,
            row: self.row,
            width: left_width,
            height: self.height,
        };
        let right = Self {
            col: self.col + left_width,
            row: self.row,
            width: self.width - left_width,
            height: self.height,
        };
        (left, right)
    }
    pub fn split_vertical(&self, top_height: usize) -> (Area, Area) {
        let top = Self {
            col: self.col,
            row: self.row,
            width: self.width,
            height: top_height,
        };
        let bottom = Self {
            col: self.col,
            row: self.row + top_height,
            width: self.width,
            height: self.height - top_height,
        };
        (top, bottom)
    }
}

pub type ViewElem = (char, Color, Color);
pub type ViewElemDiff = (Option<char>, Option<Color>, Option<Color>);

pub trait ViewGen {
    fn gen(&self, region: Area) -> Box<View>;
}

pub trait View {
    fn get(&self, col: usize, row: usize) -> ViewElem;
    fn get_cursor_pos(&self) -> Option<Cursor>;
}

pub struct NullView {}
impl View for NullView {
    fn get(&self, col: usize, row: usize) -> ViewElem {
        (' ', Color::Black, Color::Black)
    }
    fn get_cursor_pos(&self) -> Option<Cursor> {
        None
    }
}
pub struct NullViewGen {}
impl ViewGen for NullViewGen {
    fn gen(&self, _: Area) -> Box<View> {
        Box::new(NullView {})
    }
}

pub trait DiffView {
    fn get(&self, col: usize, row: usize) -> ViewElemDiff;
}

pub struct ToView {
    x: Vec<Vec<BufElem>>
}
impl View for ToView {
    fn get(&self, col: usize, row: usize) -> ViewElem {
        if row >= self.x.len() || col >= self.x[row].len() {
            return (' ', Color::Black, Color::Black)
        }
        let e: &BufElem = &self.x[row][col];
        let c = match *e {
            BufElem::Char(c) => c,
            BufElem::Eol => ' '
        };
        (c, Color::White, Color::Black)
    }
    fn get_cursor_pos(&self) -> Option<Cursor> { None }
}
impl ToView {
    pub fn new(x: Vec<Vec<BufElem>>) -> Self {
        Self { x }
    }
}

pub struct AddCursor<V> {
    x: V,
    cursor: Option<Cursor>,
}
impl <V> View for AddCursor<V> where V: View {
    fn get(&self, col: usize, row: usize) -> ViewElem { self.x.get(col, row) }
    fn get_cursor_pos(&self) -> Option<Cursor> {
        self.cursor
    }
}
impl <V> AddCursor<V> {
    pub fn new(x: V, cursor: Option<Cursor>) -> Self {
        Self { x, cursor }
    }
}

pub struct TranslateView<V> {
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
                row: (cur.row as i32 + self.diff_row) as usize,
                col: (cur.col as i32 + self.diff_col) as usize,
            }
        )
    }
}
impl <V> TranslateView<V> {
    pub fn new(x: V, diff_col: i32, diff_row: i32) -> Self {
        Self { x, diff_col, diff_row }
    }
}

pub struct MergeVertical<V1,V2> {
    pub top: V1,
    pub bottom: V2,
    pub row_offset: usize,
}
impl <V1,V2> View for MergeVertical<V1,V2> where V1: View, V2: View {
    fn get(&self, col: usize, row: usize) -> ViewElem {
        if row < self.row_offset {
            self.top.get(col, row)
        } else {
            self.bottom.get(col, row)
        }
    }
    fn get_cursor_pos(&self) -> Option<Cursor> {
        self.top.get_cursor_pos().or(self.bottom.get_cursor_pos())
    }
}

pub struct MergeHorizontal<V1,V2> {
    pub left: V1,
    pub right: V2,
    pub col_offset: usize,
}
impl <V1,V2> View for MergeHorizontal<V1,V2> where V1: View, V2: View {
    fn get(&self, col: usize, row: usize) -> ViewElem {
        if col < self.col_offset {
            self.left.get(col, row)
        } else {
            self.right.get(col, row)
        }
    }
    fn get_cursor_pos(&self) -> Option<Cursor> {
        self.left.get_cursor_pos().or(self.right.get_cursor_pos())
    }
}

pub struct LineNumber {
    pub from: usize,
    pub to: usize,
}
impl View for LineNumber {
    fn get(&self, col: usize, row: usize) -> ViewElem {
        let n = self.from + row;
        let c = if n <= self.to {
            let line: Vec<char> = format!("{0:>5} ", n).chars().collect();
            line[col]
        } else {
            ' '
        };
        (c, Color::White, Color::Black)
    }
    fn get_cursor_pos(&self) -> Option<Cursor> { None }
}
#[test]
fn test_lineno() {
    let view = LineNumber { from: 15, to: 15 };
    for (i, &c) in [' ', ' ', ' ', '1', '5', ' '].iter().enumerate() {
        assert_eq!(view.get(i,0).0, c);
    }
}

pub struct SearchBar {
    s: Vec<char>
}
impl SearchBar {
    pub fn new(s: &str) -> Self {
        let mut v = vec!['/'];
        for c in s.chars() {
            v.push(c);
        }
        Self { s: v }
    }
}
impl View for SearchBar {
    fn get(&self, col: usize, row: usize) -> ViewElem {
        if row == 0 {
            let c = if 0 <= col && col < self.s.len() {
                self.s[col]
            } else {
                ' '
            };
            (c, Color::White, Color::Black)
        } else {
            (' ', Color::White, Color::Black)
        }
    }
    fn get_cursor_pos(&self) -> Option<Cursor> { None }
}

pub struct OverlayView<V, D> {
    v: V,
    d: D,
}
impl <V, D> OverlayView<V, D> where V: View, D: DiffView {
    pub fn new(v: V, d: D) -> Self {
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

pub struct VisualRangeDiffView {
    range: Option<CursorRange>, // doubtful design to have option here
}
impl DiffView for VisualRangeDiffView {
    fn get(&self, col: usize, row: usize) -> ViewElemDiff {
        let as_cursor = Cursor { row, col };
        let in_visual_range = self.range.map(|r| r.start <= as_cursor && as_cursor < r.end).unwrap_or(false);
        if in_visual_range {
            (None, None, Some(Color::Blue))
        } else {
            (None, None, None)
        }
    }
}
impl VisualRangeDiffView {
    pub fn new(range: Option<CursorRange>) -> Self {
        Self { range }
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
    let v0 = ToView { x: buf };
    let d0 = TestDiffView {};
    let v1 = OverlayView { v: v0, d: d0 };

    let view: Box<dyn View> = Box::new(v1);
    let reg = Area {
        col: 0,
        row: 0,
        width: 1,
        height: 1,
    };
    let e = view.get(0,0);
    assert_eq!(e, ('a', Color::Red, Color::Black));
}
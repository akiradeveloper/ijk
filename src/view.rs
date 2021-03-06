extern crate flame;

use crate::screen::Color;
use crate::read_buffer::{BufElem, Cursor};

impl From<syntect::highlighting::Color> for Color {
    fn from(c: syntect::highlighting::Color) -> Color {
        Color::Rgb(c.r, c.g, c.b)
    }
}

pub fn default_fg() -> Color {
    crate::theme::default().settings.foreground.unwrap().into()
}

pub fn default_bg() -> Color {
    crate::theme::default().settings.background.unwrap().into()
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub struct Area {
    pub col: usize,
    pub row: usize,
    pub width: usize,
    pub height: usize,
}
impl Area {
    pub fn contains(&self, col: usize, row: usize) -> bool {
        if col < self.col {
            return false
        }
        if row < self.row {
            return false
        }
        if col >= self.col + self.width {
            return false
        }
        if row >= self.row + self.height {
            return false
        }

        true
    }
    pub fn contains_area(&self, other: &Self) -> bool {
        if other.col < self.col {
            return false
        }
        if other.row < self.row {
            return false
        }
        if other.col + other.width > self.col + self.width {
            return false
        }
        if other.row + other.height > self.row + self.height {
            return false
        }

        true
    }
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

pub struct BufArea<'a, T> {
    backing: &'a [Vec<T>],
}
impl <'a, T> BufArea<'a, T> {
    pub fn new(backing: &'a [Vec<T>]) -> Self {
        Self { backing }
    }
    fn get(&self, col: usize, row: usize) -> Option<&T> {
        if row > self.backing.len() - 1 || col > self.backing[row].len() - 1 {
            None
        } else {
            Some(&self.backing[row][col])
        }
    }
    pub fn map(self, f: fn(&T) -> ViewElem) -> Map<'a, T> {
        Map { backing: self, f }
    }
}
pub struct Map<'a, T> {
    backing: BufArea<'a, T>,
    f: fn(&T) -> ViewElem,
}
impl <'a, T> View for Map<'a, T> {
    fn get(&self, col: usize, row: usize) -> ViewElem {
        match self.backing.get(col, row) {
            None => (None, None, None),
            Some(t) => (self.f)(t)
        }
    }
}


// pub struct BufArea<T> {
//     copy: Vec<Vec<T>>,
//     area: Area,
// }
// impl <T: Clone> BufArea<T> {
//     pub fn new(orig: &[Vec<T>], area: Area) -> Self {
//         let mut v = vec![];
//         for i in 0..area.height {
//             let row = area.row + i;
//             if row > orig.len() - 1 {
//                 break;
//             }
//             let mut vv = vec![];
//             for j in 0..area.width {
//                 let col = area.col + j;
//                 if col > orig[row].len() - 1 {
//                     break;
//                 }
//                 vv.push(orig[row][col].clone());
//             }
//             v.push(vv);
//         }
//         // orig and area should have some overwrap
//         assert!(!v.is_empty());
//         Self {
//             copy: v,
//             area: area,
//         }
//     }
//     pub fn get(&self, col: usize, row: usize) -> Option<&T> {
//         let copy_row = row - self.area.row;
//         let copy_col = col - self.area.col;
//         if copy_row > self.copy.len() - 1 || copy_col > self.copy[copy_row].len() - 1 {
//             None
//         } else {
//             Some(&self.copy[copy_row][copy_col])
//         }
//     }
//     #[deprecated]
//     pub fn last_some(&self) -> &T {
//         let row = self.copy.len() - 1;
//         let col = self.copy[row].len() - 1;
//         &self.copy[row][col]
//     }
// }

pub type ViewElem = (Option<char>, Option<Color>, Option<Color>);

pub trait ViewGen {
    fn gen(&mut self, region: Area) -> Box<View>;
}

pub trait View {
    fn get(&self, col: usize, row: usize) -> ViewElem {
        (None, None, None)
    }
    fn get_cursor_pos(&self) -> Option<Cursor> {
        None
    }
}

impl <V: View + ?Sized> View for Box<V> {
    fn get(&self, col: usize, row: usize) -> ViewElem {
        (**self).get(col, row)
    }
    fn get_cursor_pos(&self) -> Option<Cursor> {
        (**self).get_cursor_pos()
    }
}

pub struct NullView {}
impl View for NullView {
    fn get(&self, col: usize, row: usize) -> ViewElem {
        (Some(' '), Some(default_fg()), Some(default_bg()))
    }
    fn get_cursor_pos(&self) -> Option<Cursor> {
        Some(Cursor { row:0, col:0 })
    }
}
pub struct NullViewGen {}
impl ViewGen for NullViewGen {
    fn gen(&mut self, _: Area) -> Box<View> {
        Box::new(NullView {})
    }
}

pub struct EnableView<V> {
    backing: V,
    enabled: bool,
}
impl <V: View> EnableView<V> {
    pub fn new(backing: V, enabled: bool) -> Self {
        Self { backing, enabled }
    }
}
impl <V: View> View for EnableView<V> {
    fn get(&self, col: usize, row: usize) -> ViewElem {
        if self.enabled {
            self.backing.get(col, row)
        } else {
            (None, None, None)
        }
    }
    fn get_cursor_pos(&self) -> Option<Cursor> {
        if self.enabled {
            self.backing.get_cursor_pos()
        } else {
            None
        }
    }
}

pub struct CloneView {
    owned: Vec<Vec<ViewElem>>,
    cursor: Option<Cursor>,
    area: Area,
}
impl CloneView {
    pub fn new<V: View>(orig: V, area: Area) -> Self {
        let mut v = vec![];
        for i in 0..area.height {
            let mut vv = vec![];
            let row = area.row + i;
            for j in 0..area.width {
                let col = area.col + j;
                vv.push(orig.get(col, row))
            }
            v.push(vv);
        }
        // orig and area should have some overwrap
        assert!(!v.is_empty());
        Self {
            owned: v,
            cursor: orig.get_cursor_pos(),
            area: area,
        }
    }
}
impl View for CloneView {
    fn get(&self, col: usize, row: usize) -> ViewElem {
        if !self.area.contains(col, row) {
            (None, None, None)
        } else {
            let i = row - self.area.row;
            let j = col - self.area.col;
            self.owned[i][j]
        }
    }
    fn get_cursor_pos(&self) -> Option<Cursor> {
        self.cursor
    }
}

pub struct ToView<'a> {
    pub back: &'a[Vec<BufElem>],
}
impl <'a> ToView<'a> {
    pub fn new(back: &'a[Vec<BufElem>]) -> Self {
        Self { back }
    }
}
impl <'a> View for ToView<'a> {
    fn get(&self, col: usize, row: usize) -> ViewElem {
        if row > self.back.len() - 1 || col > self.back[row].len() - 1 {
            (Some(' '), Some(default_fg()), Some(default_bg()))
        } else {
            let e = &self.back[row][col];
            let c = match *e {
                BufElem::Char(c) => c,
                BufElem::Eol => ' ',
            };
            (Some(c), Some(default_fg()), Some(default_bg()))
        }
    }
    fn get_cursor_pos(&self) -> Option<Cursor> { None }
}

// #[deprecated]
// pub struct ToView {
//     buf_area: BufArea<BufElem>,
// }
// impl ToView {
//     pub fn new(orig: &[Vec<BufElem>], area: Area) -> Self {
//         let _flame_guard = flame::start_guard("clone area buf");
//         Self {
//             buf_area: BufArea::new(orig, area)
//         }
//     }
// }
// impl View for ToView {
//     fn get(&self, col: usize, row: usize) -> ViewElem {
//         match self.buf_area.get(col, row) {
//             Some(e) => {
//                 let c = match *e {
//                     BufElem::Char(c) => c,
//                     BufElem::Eol => ' ',
//                 };
//                 (Some(c), Some(default_fg()), Some(default_bg()))
//             },
//             None => (Some(' '), Some(default_fg()), Some(default_bg()))
//         }
//     }
// }

pub struct BgColor {
    bg: Color,
}
impl View for BgColor {
    fn get(&self, col: usize, row: usize) -> ViewElem {
        (Some(' '), Some(self.bg), Some(self.bg))
    }
}

pub const LINE_NUMBER_W: usize = 7;
pub struct LineNumber {
    pub from: usize,
    pub to: usize,
}
impl View for LineNumber {
    fn get(&self, col: usize, row: usize) -> ViewElem {
        let n = self.from + row;
        let c = if n <= self.to {
            let line: Vec<char> = format!("{0:>5}  ", n).chars().collect();
            line[col]
        } else {
            ' '
        };
        let bg = if col < 6 {
            default_bg()
            // Color::Black
        } else {
            default_bg()
        };
        (Some(c), Some(Color::White), Some(bg))
    }
    fn get_cursor_pos(&self) -> Option<Cursor> {
        None
    }
}
#[test]
fn test_lineno() {
    let view = LineNumber { from: 15, to: 15 };
    for (i, &c) in [' ', ' ', ' ', '1', '5', ' '].iter().enumerate() {
        assert_eq!(view.get(i, 0).0, Some(c));
    }
}

pub struct AddCursor {
    cursor: Cursor,
}
impl AddCursor {
    pub fn new(cursor: Cursor) -> Self {
        Self { cursor }
    }
}
impl View for AddCursor {
    fn get_cursor_pos(&self) -> Option<Cursor> {
        Some(self.cursor)
    }
}

// #[deprecated]
// pub struct AddCursor<V> {
//     x: V,
//     cursor: Option<Cursor>,
// }
// impl<V> View for AddCursor<V>
// where
//     V: View,
// {
//     fn get(&self, col: usize, row: usize) -> ViewElem {
//         self.x.get(col, row)
//     }
//     fn get_cursor_pos(&self) -> Option<Cursor> {
//         self.cursor
//     }
// }
// impl<V> AddCursor<V> {
//     pub fn new(x: V, cursor: Option<Cursor>) -> Self {
//         Self { x, cursor }
//     }
// }

pub struct TranslateView<V> {
    x: V,
    diff_col: i32,
    diff_row: i32,
}
impl<V> View for TranslateView<V>
where
    V: View,
{
    fn get(&self, col: usize, row: usize) -> ViewElem {
        let c = (col as i32 - self.diff_col) as usize;
        let r = (row as i32 - self.diff_row) as usize;
        self.x.get(c, r)
    }
    fn get_cursor_pos(&self) -> Option<Cursor> {
        self.x.get_cursor_pos().map(|cur| Cursor {
            row: (cur.row as i32 + self.diff_row) as usize,
            col: (cur.col as i32 + self.diff_col) as usize,
        })
    }
}
impl<V> TranslateView<V> {
    pub fn new(x: V, diff_col: i32, diff_row: i32) -> Self {
        Self {
            x,
            diff_col,
            diff_row,
        }
    }
}

pub struct MergeVertical<V1, V2> {
    pub top: V1,
    pub bottom: V2,
    pub row_offset: usize,
}
impl<V1, V2> View for MergeVertical<V1, V2>
where
    V1: View,
    V2: View,
{
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

pub struct MergeHorizontal<V1, V2> {
    pub left: V1,
    pub right: V2,
    pub col_offset: usize,
}
impl<V1, V2> View for MergeHorizontal<V1, V2>
where
    V1: View,
    V2: View,
{
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

pub struct OverlayView<V, D> {
    v: V,
    d: D,
}
impl<V, D> OverlayView<V, D>
where
    V: View,
    D: View,
{
    pub fn new(v: V, d: D) -> Self {
        Self { v, d }
    }
}
impl<V, D> View for OverlayView<V, D>
where
    V: View,
    D: View,
{
    fn get(&self, col: usize, row: usize) -> ViewElem {
        let (v0, v1, v2) = self.v.get(col, row);
        let (d0, d1, d2) = self.d.get(col, row);
        (d0.or(v0), d1.or(v1), d2.or(v2))
    }
    fn get_cursor_pos(&self) -> Option<Cursor> {
        self.d.get_cursor_pos().or(self.v.get_cursor_pos())
    }
}

#[cfg(test)]
struct TestDiffView {}
#[cfg(test)]
impl View for TestDiffView {
    fn get(&self, col: usize, row: usize) -> ViewElem {
        (Some('a'), Some(Color::Red), None)
    }
}

#[test]
fn test_view_overlay() {
    let buf = vec![vec![BufElem::Eol]];
    let area = Area {
        col: 0,
        row: 0,
        width: 1,
        height: 1,
    };
    let v0 = ToView::new(&buf);
    let d0 = TestDiffView {};
    let v1 = OverlayView { v: v0, d: d0 };

    let view: Box<dyn View> = Box::new(v1);

    let e = view.get(0, 0);
    assert_eq!(e, (Some('a'), Some(Color::Red), Some(default_bg())));
}

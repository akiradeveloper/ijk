use std::rc::Rc;
use std::cell::RefCell;
use super::edit_buffer::{self, EditBuffer};
use super::controller;
use super::view;
use super::navigator::{self, Navigator};
use super::read_buffer::ReadBuffer;
use std::path::{self, Path, PathBuf};
use std::fs;
use crate::BufElem;
use crate::screen::Color;

enum Entry {
    Parent(path::PathBuf),
    File(path::PathBuf),
    Dir(path::PathBuf),
}

pub struct Directory {
    pub rb: ReadBuffer,
    path: path::PathBuf,
    entries: Vec<Entry>,
    navigator: Rc<RefCell<Navigator>>,
}
impl Directory {
    pub fn open(path: &Path, navigator: Rc<RefCell<Navigator>>) -> Self {
        let mut r = Self {
            path: fs::canonicalize(path).unwrap(),
            entries: vec![],
            rb: ReadBuffer::new(vec![]),
            navigator: navigator,
        };
        r.refresh();
        r
    }
    pub fn refresh(&mut self) {
        self.entries.clear();
        for p in self.path.parent() {
            self.entries.push(Entry::Parent(fs::canonicalize(p).unwrap()))
        }
        for entry in self.path.read_dir().unwrap() {
            let p = fs::canonicalize(entry.unwrap().path()).unwrap();
            let e = if p.is_file() {
                Entry::File(p)
            } else {
                Entry::Dir(p)
            };
            self.entries.push(e);
        }
        let mut v = vec![];
        for e in &self.entries {
            let mut vv = vec![];
            match e.clone() {
                Entry::Parent(_) => {
                    vv.push(BufElem::Char('.'));
                    vv.push(BufElem::Char('.'));
                },
                Entry::File(path) => {
                    for c in path.file_name().unwrap().to_str().unwrap().chars() {
                        vv.push(BufElem::Char(c));
                    }
                },
                Entry::Dir(path) => {
                    for c in path.file_name().unwrap().to_str().unwrap().chars() {
                        vv.push(BufElem::Char(c));
                    }
                    vv.push(BufElem::Char('/'));
                },
            }
            vv.push(BufElem::Eol);
            v.push(vv);
        }
        self.rb = ReadBuffer::new(v);
    }

    pub fn eff_cursor_up(&mut self, _: Key) {
        self.rb.cursor_up();
    }
    pub fn eff_cursor_down(&mut self, _: Key) {
        self.rb.cursor_down();
    }
    pub fn eff_select(&mut self, _: Key) {
        let i = self.rb.cursor.row;
        let entry = &self.entries[i];
        let page: Rc<navigator::Page> = match entry.clone() {
            Entry::Parent(path) => {
                let dir = Rc::new(RefCell::new(self::Directory::open(&path, self.navigator.clone())));
                Rc::new(self::Page::new(dir, path.clone()))
            },
            Entry::Dir(path) => {
                let dir = Rc::new(RefCell::new(self::Directory::open(&path, self.navigator.clone())));
                Rc::new(self::Page::new(dir, path.clone()))
            },
            Entry::File(path) => {
                let x = Rc::new(RefCell::new(EditBuffer::open(Some(&path))));
                Rc::new(edit_buffer::Page::new(x))
            },
        };
        self.navigator.borrow_mut().push(page);
    }
    pub fn eff_go_down(&mut self, _: Key) {
        let i = self.rb.cursor.row;
        let entry = &self.entries[i];
        match entry.clone() {
            Entry::Dir(path) => {
                let dir = Rc::new(RefCell::new(self::Directory::open(&path, self.navigator.clone())));
                let new_dir = Rc::new(self::Page::new(dir, path.clone()));
                self.navigator.borrow_mut().pop_and_push(new_dir);
            },
            _ => {}
        };
    }
    pub fn eff_go_up(&mut self, _: Key) {
        for e in &self.entries {
            match e {
                Entry::Parent(path) => {
                    let dir = Rc::new(RefCell::new(self::Directory::open(&path, self.navigator.clone())));
                    let new_dir = Rc::new(self::Page::new(dir, path.clone()));
                    self.navigator.borrow_mut().pop_and_push(new_dir);
                },
                _ => {},
            }
        }
    }
}

use crate::controller::Effect;
use crate::def_effect;
use crate::Key;

def_effect!(CursorUp, Directory, eff_cursor_up);
def_effect!(CursorDown, Directory, eff_cursor_down);
def_effect!(Select, Directory, eff_select);
def_effect!(GoDown, Directory, eff_go_down);
def_effect!(GoUp, Directory, eff_go_up);

pub fn mk_controller(x: Rc<RefCell<Directory>>) -> controller::ControllerFSM {
    use crate::Key::*;
    let mut g = controller::GraphImpl::new();
    g.add_edge("init", "init", Char('k'), Rc::new(CursorUp(x.clone())));
    g.add_edge("init", "init", Char('j'), Rc::new(CursorDown(x.clone())));
    g.add_edge("init", "init", Char('\n'), Rc::new(Select(x.clone())));
    g.add_edge("init", "init", Char('l'), Rc::new(GoDown(x.clone())));
    g.add_edge("init", "init", Char('h'), Rc::new(GoUp(x.clone())));
    controller::ControllerFSM::new("init", Box::new(g))
}
struct AddColor {
    x: Rc<RefCell<Directory>>,
}
impl AddColor {
    fn new(x: Rc<RefCell<Directory>>) -> Self {
        Self { x }
    }
}
impl view::DiffView for AddColor {
    fn get(&self, _: usize, row: usize) -> view::ViewElemDiff {
        if row > self.x.borrow().entries.len() - 1 {
            return (None, None, None)
        }
        match self.x.borrow().entries[row] {
            Entry::File(_) => (None, None, None),
            Entry::Dir(_) => (None, Some(Color::LightRed), None),
            Entry::Parent(_) => (None, Some(Color::LightRed), None),
        }
    }
}

struct ViewGen {
    x: Rc<RefCell<Directory>>,
}
impl ViewGen {
    pub fn new(x: Rc<RefCell<Directory>>) -> Self {
        Self {
            x,
         }
    }
}
impl view::ViewGen for ViewGen {
    fn gen(&self, region: view::Area) -> Box<view::View> {
        self.x.borrow_mut().rb.stabilize();
        self.x.borrow_mut().rb.adjust_window(region.width, region.height);
        self.x.borrow_mut().rb.update_search_results();

        let (lineno_area, dir_area) = region.split_horizontal(view::LINE_NUMBER_W);
        let dir_view = view::CutBuffer::new(&self.x.borrow().rb.buf, self.x.borrow().rb.current_window());
        let add_color = AddColor::new(self.x.clone());
        let dir_view = view::OverlayView::new(dir_view, add_color);
        let dir_view = view::AddCursor::new(
            dir_view,
            Some(self.x.borrow().rb.cursor), // tmp: the cursor is always visible
        );
        let dir_view = view::TranslateView::new(
            dir_view,
            dir_area.col as i32 - self.x.borrow().rb.window.col() as i32,
            dir_area.row as i32 - self.x.borrow().rb.window.row() as i32,
        );

        let lineno_range = self.x.borrow().rb.lineno_range();
        let lineno_view = view::LineNumber {
            from: lineno_range.start+1,
            to: lineno_range.end,
        };

        let view = view::MergeHorizontal {
            left: lineno_view,
            right: dir_view,
            col_offset: dir_area.col,
        };
        Box::new(view)
    }
}
pub struct Page {
    controller: Box<controller::Controller>,
    view_gen: Box<view::ViewGen>,
    x: Rc<RefCell<Directory>>,
    // WA:
    // to call eff_select() we need to take the borrow_mut() of the directory
    // and neither borrow() nor borrow_mut() should not be called again under
    // the path. (although this is too strict.)
    // if id() is implemented in a way it borrows the `path` from the directory
    // this violates the borrow rules in runtime.
    path: PathBuf,
}
impl Page {
    pub fn new(x: Rc<RefCell<Directory>>, path: PathBuf) -> Self {
        Self {
            controller: Box::new(mk_controller(x.clone())),
            view_gen: Box::new(ViewGen::new(x.clone())),
            x: x,
            path: path,
        }
    }
}
impl navigator::Page for Page {
    fn controller(&self) -> &Box<controller::Controller> {
        &self.controller
    }
    fn view_gen(&self) -> &Box<view::ViewGen> {
        &self.view_gen
    }
    fn desc(&self) -> String {
        format!("[DIRECTORY] {}", self.path.to_str().unwrap().to_owned())
    }
    fn kind(&self) -> navigator::PageKind {
        navigator::PageKind::Directory
    }
    fn id(&self) -> String {
        // should not call self.x.borrow() here
        self.path.to_str().unwrap().to_owned()
    }
}
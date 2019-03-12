use std::rc::Rc;
use std::cell::RefCell;
use super::controller;
use super::view;
use super::navigator;
use super::read_buffer::ReadBuffer;
use std::path;
use std::fs;
use crate::BufElem;

enum Entry {
    Parent(path::PathBuf),
    File(path::PathBuf),
    Dir(path::PathBuf),
}

pub struct Directory {
    pub rb: ReadBuffer,
    path: path::PathBuf,
    entries: Vec<Entry>,
}
impl Directory {
    pub fn open(path: &path::Path) -> Self {
        let mut r = Self {
            path: fs::canonicalize(path).unwrap(),
            entries: vec![],
            rb: ReadBuffer::new(vec![]),
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
}

use crate::controller::Effect;
use crate::Key;
macro_rules! def_effect {
    ($eff_name:ident, $t:ty, $fun_name:ident) => {
        struct $eff_name(Rc<RefCell<$t>>);
        impl Effect for $eff_name {
            fn run(&self, k: Key) {
                self.0.borrow_mut().$fun_name(k);
            }
        }
    };
}
def_effect!(CursorUp, Directory, eff_cursor_up);
def_effect!(CursorDown, Directory, eff_cursor_down);

pub fn mk_controller(x: Rc<RefCell<Directory>>) -> controller::ControllerFSM {
    use crate::Key::*;
    let mut g = controller::GraphImpl::new();
    g.add_edge("init", "init", Char('k'), Rc::new(CursorUp(x.clone())));
    g.add_edge("init", "init", Char('j'), Rc::new(CursorDown(x.clone())));
    controller::ControllerFSM {
        cur: "init".to_owned(),
        g: Box::new(g),
    }
}
struct ViewGen {
    x: Rc<RefCell<Directory>>,
    old_region: view::ViewRegion,
}
impl ViewGen {
    pub fn new(x: Rc<RefCell<Directory>>) -> Self {
        Self {
            x,
            old_region: view::ViewRegion {
                col: 0,
                row: 0,
                width: 0,
                height: 0,
            },
         }
    }
}
impl view::ViewGen for ViewGen {
    fn gen(&mut self, region: view::ViewRegion) -> Box<view::View> {
        self.x.borrow_mut().rb.stabilize();
        if self.old_region != region {
            self.x.borrow_mut().rb.resize_window(region.width - 6, region.height - 1);
            self.old_region = region;
        }
        self.x.borrow_mut().rb.adjust_window();
        self.x.borrow_mut().rb.update_search_results();

        let dir_area = region;
        let dir_view = view::ToView::new(self.x.borrow().rb.buf.clone());
        let dir_view = view::AddCursor::new(
            dir_view,
            Some(self.x.borrow().rb.cursor), // tmp: the cursor is always visible
        );
        let dir_view = view::TranslateView::new(
            dir_view,
            dir_area.col as i32 - self.x.borrow().rb.filter.col() as i32,
            dir_area.row as i32 - self.x.borrow().rb.filter.row() as i32,
        );

        let view = dir_view;
        Box::new(view)
    }
}
pub struct Page {
    controller: Rc<RefCell<controller::Controller>>,
    view_gen: Rc<RefCell<view::ViewGen>>,
    x: Rc<RefCell<Directory>>,
}
impl Page {
    pub fn new(x: Rc<RefCell<Directory>>) -> Self {
        Self {
            controller: Rc::new(RefCell::new(mk_controller(x.clone()))),
            view_gen: Rc::new(RefCell::new(ViewGen::new(x.clone()))),
            x: x,
        }
    }
}
impl navigator::Page for Page {
    fn controller(&self) -> Rc<RefCell<controller::Controller>> {
        self.controller.clone()
    }
    fn view_gen(&self) -> Rc<RefCell<view::ViewGen>> {
        self.view_gen.clone()
    }
    fn desc(&self) -> String {
        "directory".to_owned() // tmp
    }
    fn kind(&self) -> navigator::PageKind {
        navigator::PageKind::Directory
    }
    fn id(&self) -> String {
        "bbb".to_owned()
    }
}
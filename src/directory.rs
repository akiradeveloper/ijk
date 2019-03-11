use std::rc::Rc;
use std::cell::RefCell;
use super::controller;
use super::view;
use super::navigator;
use super::read_buffer::ReadBuffer;
use std::path;
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
            path: path.to_owned(),
            entries: vec![],
            rb: ReadBuffer::new(vec![]),
        };
        r.refresh();
        r
    }
    pub fn refresh(&mut self) {
        self.entries.clear();
        for p in self.path.parent() {
            self.entries.push(Entry::Parent(p.to_owned()))
        }
        for entry in self.path.read_dir().unwrap() {
            let p = entry.unwrap().path();
            let e = if p.is_file() {
                Entry::File(p)
            } else {
                Entry::Dir(p)
            };
            self.entries.push(e);
        }
    }
}
pub fn mk_controller(x: Rc<RefCell<Directory>>) -> controller::ControllerFSM {
    unimplemented!()
}
struct ViewGen {

}
impl ViewGen {
    pub fn new(x: Rc<RefCell<Directory>>) -> Self {
        Self {}
    }
}
impl view::ViewGen for ViewGen {
    fn gen(&mut self, region: view::ViewRegion) -> Box<view::View> {
        unimplemented!()
    }
}
pub struct Page {

}
impl Page {
    pub fn new(x: Rc<RefCell<Directory>>) -> Self {
        Self {}
    }
}
impl navigator::Page for Page {
    fn controller(&self) -> Rc<RefCell<controller::Controller>> {
        unimplemented!()
    }
    fn view_gen(&self) -> Rc<RefCell<view::ViewGen>> {
        unimplemented!()
    }
    fn desc(&self) -> String {
        unimplemented!()
    }
}
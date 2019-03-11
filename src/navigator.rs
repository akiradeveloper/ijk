use super::controller;
use super::view;
use super::read_buffer;
use crate::BufElem;
use std::collections::VecDeque;
use std::rc::Rc;
use std::cell::RefCell;

pub trait Page {
    fn controller(&self) -> Rc<RefCell<controller::Controller>>;
    fn view_gen(&self) -> Rc<RefCell<view::ViewGen>>;
    fn desc(&self) -> String;
}

pub struct Navigator {
    pub controller: Rc<RefCell<controller::Controller>>,
    pub view_gen: Rc<RefCell<view::ViewGen>>,
    list: VecDeque<Box<Page>>,
    rb: read_buffer::ReadBuffer,
}
impl Navigator {
    pub fn new() -> Self {
        Self {
            controller: Rc::new(RefCell::new(controller::NullController {})),
            view_gen: Rc::new(RefCell::new(view::NullViewGen {})),
            list: VecDeque::new(),
            rb: read_buffer::ReadBuffer::new(vec![]),
        }
    }
    fn refresh_buffer(&mut self) {
        let mut v = vec![];
        for e in &self.list {
            let mut vv = vec![];
            for c in e.desc().chars() {
                vv.push(BufElem::Char(c));
            }
            vv.push(BufElem::Eol);
            v.push(vv);
        }
        self.rb = read_buffer::ReadBuffer::new(v);
    }
    pub fn set(&mut self, controller: Rc<RefCell<controller::Controller>>, view_gen: Rc<RefCell<view::ViewGen>>) {
        self.controller = controller;
        self.view_gen = view_gen;
    }
    fn select(&mut self, i: usize) {
        // TODO move i-th to the top
        self.refresh_buffer();
        self.set(self.list[i].controller(), self.list[i].view_gen());
    }
    fn delete(&mut self, i: usize) {
        self.refresh_buffer();
    }
    pub fn push(&mut self, page: Box<Page>) {
        self.list.push_front(page);
        self.select(0);
    }
    pub fn pop(&mut self) {
        self.refresh_buffer();
    }
}

pub fn mk_controller(x: Rc<RefCell<Navigator>>) -> controller::ControllerFSM {
    let mut g = controller::GraphImpl::new();
    // TODO
    controller::ControllerFSM {
        cur: "init".to_owned(),
        g: Box::new(g),
    }
}
pub struct ViewGen {
    x: Rc<RefCell<Navigator>>,
    old_region: view::ViewRegion,
}
impl ViewGen {
    pub fn new(x: Rc<RefCell<Navigator>>) -> Self {
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

        let navi_area = region;
        let navi_view = view::ToView::new(self.x.borrow().rb.buf.clone());
        let navi_view = view::TranslateView::new(
            navi_view,
            navi_area.col as i32 - self.x.borrow().rb.filter.col() as i32,
            navi_area.row as i32 - self.x.borrow().rb.filter.row() as i32,
        );

        let view = navi_view;
        Box::new(view)
    }
}
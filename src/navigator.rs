use super::controller;
use super::view;
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
}
impl Navigator {
    pub fn new() -> Self {
        Self {
            controller: Rc::new(RefCell::new(controller::NullController {})),
            view_gen: Rc::new(RefCell::new(view::NullViewGen {})),
            list: VecDeque::new(),
        }
    }
    fn set(&mut self, controller: Rc<RefCell<controller::Controller>>, view_gen: Rc<RefCell<view::ViewGen>>) {
        self.controller = controller;
        self.view_gen = view_gen;
    }
    fn select(&mut self, i: usize) {
        self.set(self.list[i].controller(), self.list[i].view_gen())
    }
    fn delete(&mut self, i: usize) {

    }
    pub fn push(&mut self, page: Box<Page>) {
        self.list.push_front(page);
        self.select(0)
    }
    pub fn pop(&mut self) {

    }
}
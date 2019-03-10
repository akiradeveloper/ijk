use crate::controller;
use crate::view;
use std::rc::Rc;
use std::cell::RefCell;
trait Page {
    fn controller(&self) -> Rc<RefCell<controller::Controller>>;
    fn view_gen(&self) -> Rc<RefCell<view::ViewGen>>;
    fn desc(&self) -> String;
}

struct Navigator {
    list: Vec<Box<Page>>,
    controller: Rc<RefCell<controller::Controller>>,
    view_gen: Rc<RefCell<view::ViewGen>>,
}
impl Navigator {
    fn new(init_page: Box<Page>) -> Self {
        Self {
            controller: init_page.controller(),
            view_gen: init_page.view_gen(),
            list: vec![init_page],
        }
    }
    fn set(&mut self, controller: Rc<RefCell<controller::Controller>>, view_gen: Rc<RefCell<view::ViewGen>>) {
        self.controller = controller;
        self.view_gen = view_gen;
    }
    fn select(&mut self, i: usize) {

    }
    fn push(&mut self, page: Box<Page>) {

    }
    fn pop(&mut self) {

    }
    fn delete(&mut self, i: usize) {

    }
}
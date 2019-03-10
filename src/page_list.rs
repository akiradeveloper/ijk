use crate::controller;
use crate::view;
use std::rc::Rc;
use std::cell::RefCell;
trait Page {
    fn controller(&self) -> Rc<RefCell<controller::Controller>>;
    fn view_gen(&self) -> Rc<RefCell<view::ViewGen>>;
    fn desc(&self) -> String;
}

struct PageList {
    list: Vec<Box<Page>>,
}
impl PageList {
    fn new(init_page: Box<Page>) -> Self {
        Self {
            list: vec![init_page],
        }
    }
    fn peek(&self) -> &Box<Page> {
        self.list.last().unwrap()
    }
    fn push(&mut self, page: Box<Page>) {

    }
    fn pop(&mut self) {

    }
    fn delete(&mut self, at: usize) {

    }
}
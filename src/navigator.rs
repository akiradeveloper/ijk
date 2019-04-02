use super::controller::{self, PageState};
use super::view;
use super::read_buffer::{self, BufElem, ReadBuffer};
use std::rc::Rc;
use std::cell::RefCell;
use crate::message_box::MessageBox;
use crate::read_buffer::{INIT, SEARCH, JUMP};

#[derive(PartialEq)]
pub enum PageKind {
    Buffer,
    Directory,
    Navigator,
}

pub trait Page {
    fn controller(&self) -> &Box<controller::Controller>;
    fn view_gen(&mut self) -> &mut Box<view::ViewGen>;
    fn kind(&self) -> PageKind;
    fn id(&self) -> String;
    fn status(&self) -> String;
    fn message(&self) -> MessageBox;
}

pub struct Navigator {
    needs_refresh: bool,
    current: Option<Rc<RefCell<Page>>>,
    list: Vec<Rc<RefCell<Page>>>,
    rb: ReadBuffer,
    state: PageState,
    message_box: MessageBox,
}
impl Navigator {
    pub fn new() -> Self {
        let state = PageState::new(INIT.to_owned());
        let message_box = MessageBox::new();
        Self {
            needs_refresh: false,
            current: None,
            list: vec![],
            rb: read_buffer::ReadBuffer::new(vec![], state.clone(), message_box.clone()),
            state,
            message_box,
        }
    }
    pub fn current_page(&self) -> Rc<RefCell<Page>> {
        self.current.clone().unwrap().clone()
    }
    fn update_cache(&mut self) {

    }
    fn refresh_buffer(&mut self) {
        let mut v = vec![];
        for e in &self.list {
            let mut vv = vec![];
            for c in e.borrow().status().chars() {
                vv.push(BufElem::Char(c));
            }
            vv.push(BufElem::Eol);
            v.push(vv);
        }
        self.rb = read_buffer::ReadBuffer::new(v, self.state.clone(), self.message_box.clone());
    }
    pub fn set(&mut self, page: Rc<RefCell<Page>>) {
        self.needs_refresh = true;
        self.current = Some(page);
    }
    fn select(&mut self, i: usize) {
        let e = self.list.remove(i);
        self.list.insert(0, e);
        self.set(self.list[0].clone());
    }
    fn delete(&mut self, i: usize) {
        self.list.remove(i);
        self.needs_refresh = true;
    }
    pub fn pop(&mut self) {
        self.list.remove(0);
        self.select(0);
    }
    pub fn push(&mut self, page: Rc<RefCell<Page>>) {
        let pos0 = self.list.iter().position(|e| e.borrow().id() == page.borrow().id());
        match pos0 {
            Some(i) => {
                self.select(i);
            },
            None => {
                self.list.insert(0, page);
                self.select(0);
            }
        }
    }
    pub fn pop_and_push(&mut self, e: Rc<RefCell<Page>>) {
        self.list.remove(0);
        self.list.insert(0, e);
        self.select(0);
    }
    pub fn eff_cursor_up(&mut self, _: Key) -> String {
        self.rb.cursor_up();
        INIT.to_owned()
    }
    pub fn eff_cursor_down(&mut self, _: Key) -> String {
        self.rb.cursor_down();
        INIT.to_owned()
    }
    pub fn eff_select(&mut self, _: Key) -> String {
        self.select(self.rb.cursor.row);
        INIT.to_owned()
    }
    pub fn eff_select_cur_directory(&mut self, _: Key) -> String {
        for i in self.list.iter().position(|e| e.borrow().kind() == PageKind::Directory) {
            self.select(i);
        }
        INIT.to_owned()
    }
    pub fn eff_select_cur_buffer(&mut self, _: Key) -> String {
        for i in self.list.iter().position(|e| e.borrow().kind() == PageKind::Buffer) {
            self.select(i);
        }
        INIT.to_owned()
    }
    pub fn eff_close_selected(&mut self, _: Key) -> String {
        self.delete(self.rb.cursor.row);
        INIT.to_owned()
    }
}

use crate::controller::Effect;
use crate::def_effect;
use crate::Key;

def_effect!(CursorUp, Navigator, eff_cursor_up);
def_effect!(CursorDown, Navigator, eff_cursor_down);
def_effect!(Select, Navigator, eff_select);
def_effect!(SelectCurDirectory, Navigator, eff_select_cur_directory);
def_effect!(SelectCurBuffer, Navigator, eff_select_cur_buffer);
def_effect!(CloseSelected, Navigator, eff_close_selected);

use crate::shared::AsRefMut;
pub fn mk_controller(x: Rc<RefCell<Navigator>>) -> controller::ControllerFSM {
    use crate::Key::*;
    let mut g = controller::Graph::new();
    read_buffer::add_edges(&mut g, x.clone().map(|x| &mut x.rb));

    g.add_edge(INIT, Char('\n'), Rc::new(Select(x.clone())));
    g.add_edge(INIT, Char('h'), Rc::new(SelectCurDirectory(x.clone())));
    g.add_edge(INIT, Char('l'), Rc::new(SelectCurBuffer(x.clone())));
    g.add_edge(INIT, Char('d'), Rc::new(CloseSelected(x.clone())));
    controller::ControllerFSM::new(INIT, Box::new(g))
}
pub struct ViewGen {
    x: Rc<RefCell<Navigator>>,
}
impl ViewGen {
    pub fn new(x: Rc<RefCell<Navigator>>) -> Self {
        Self {
            x,
         }
    }
}
fn gen_impl(x: &mut Navigator, region: view::Area) -> Box<view::View> {
    // refreshing the buffer content is delayed because
    // calling status of a page when it is mutablly borrowed is not safe.
    // (e.g. directory opens a file and it kicks refreshing)
    if x.needs_refresh {
        x.refresh_buffer();
        x.needs_refresh = false;
    }
    x.rb.stabilize_cursor();
    x.rb.adjust_window(region.width, region.height);
    x.update_cache();

    let (lineno_area, navi_area) = region.split_horizontal(view::LINE_NUMBER_W);
    let navi_view = view::ToView::new(&x.rb.buf);

    let add_cursor = view::AddCursor::new(x.rb.cursor);
    let navi_view = view::OverlayView::new(navi_view, add_cursor);

    let navi_view = view::TranslateView::new(
        navi_view,
        navi_area.col as i32 - x.rb.window.col() as i32,
        navi_area.row as i32 - x.rb.window.row() as i32,
    );

    let lineno_range = x.rb.lineno_range();
    let lineno_view = view::LineNumber {
        from: lineno_range.start+1,
        to: lineno_range.end,
    };

    let view = view::MergeHorizontal {
        left: lineno_view,
        right: navi_view,
        col_offset: navi_area.col,
    };

    let view = view::CloneView::new(view, region);
    Box::new(view)
}
impl view::ViewGen for ViewGen {
    fn gen(&mut self, region: view::Area) -> Box<view::View> {
        gen_impl(&mut self.x.borrow_mut(), region)
    }
}

pub struct NavigatorPage {
    controller: Box<controller::Controller>,
    view_gen: Box<view::ViewGen>,
    x: Rc<RefCell<Navigator>>,
}
impl NavigatorPage {
    pub fn new(x: Rc<RefCell<Navigator>>) -> Self {
        Self {
            controller: Box::new(mk_controller(x.clone())),
            view_gen: Box::new(ViewGen::new(x.clone())),
            x: x,
        }
    }
}
impl Page for NavigatorPage {
    fn controller(&self) -> &Box<controller::Controller> {
        &self.controller
    }
    fn view_gen(&mut self) -> &mut Box<view::ViewGen> {
        &mut self.view_gen
    }
    fn status(&self) -> String {
        let state: &str = match self.x.borrow().state.get().as_str() {
            read_buffer::INIT => "*",
            read_buffer::SEARCH => "/",
            _ => "*",
        };
        format!("[Navigator -{}-]", state)
    }
    fn kind(&self) -> PageKind {
        PageKind::Navigator
    }
    fn id(&self) -> String {
        "navigator".to_owned()
    }
    fn message(&self) -> MessageBox {
        self.x.borrow().message_box.clone()
    }
}
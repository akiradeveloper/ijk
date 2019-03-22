use super::controller::{self, Controller};
use super::view::{self, View, Area};
use super::read_buffer::{self, ReadBuffer};
use crate::read_buffer::{BufElem, Cursor};
use std::rc::Rc;
use std::cell::RefCell;
use crate::screen::Color;
use crate::message_box::MessageBox;

#[derive(PartialEq)]
pub enum PageKind {
    Buffer,
    Directory,
    Help,
    Other,
}

pub trait Page {
    fn controller(&self) -> &Box<controller::Controller>;
    fn view_gen(&self) -> &Box<view::ViewGen>;
    fn kind(&self) -> PageKind;
    fn id(&self) -> String;
    fn status(&self) -> String;
    fn message(&self) -> MessageBox;
}

struct HelpPage {
    controller: Box<controller::Controller>,
    view_gen: Box<view::ViewGen>,
}

impl Page for HelpPage {
    fn controller(&self) -> &Box<controller::Controller> {
        &self.controller
    }
    fn view_gen(&self) -> &Box<view::ViewGen> {
        &self.view_gen
    }
    fn kind(&self) -> PageKind {
        PageKind::Help
    }
    fn id(&self) -> String {
        "help".to_owned()
    }
    fn status(&self) -> String {
        "[Help]".to_owned()
    }
    fn message(&self) -> MessageBox {
        MessageBox::new()
    }
}

pub struct HelpController {}
impl Controller for HelpController {
    fn receive(&self, k: Key) {}
}

pub struct HelpView {}
impl View for HelpView {
    fn get(&self, col: usize, row: usize) -> view::ViewElem {
        (' ', Color::Black, Color::Black)
    }
    fn get_cursor_pos(&self) -> Option<Cursor> {
        None
    }
}
pub struct HelpViewGen {}
impl view::ViewGen for HelpViewGen {
    fn gen(&self, _: Area) -> Box<View> {
        Box::new(HelpView {})
    }
}

const INIT: &str = "Normal";

pub struct Navigator {
    pub current: Rc<Page>,
    list: Vec<Rc<Page>>,
    rb: ReadBuffer,
    state: String,
    message_box: MessageBox,
}
impl Navigator {
    pub fn new() -> Self {
        let help_page = Rc::new(HelpPage {
            controller: Box::new(HelpController {}),
            view_gen: Box::new(HelpViewGen {}),
        });
        let message_box = MessageBox::new();
        let mut r = Self {
            current: help_page.clone(),
            list: vec![help_page],
            rb: read_buffer::ReadBuffer::new(vec![], message_box.clone()), // not valid
            state: INIT.to_owned(),
            message_box,
        };
        r.refresh_buffer();
        r
    }
    fn update_cache(&mut self) {

    }
    fn refresh_buffer(&mut self) {
        let mut v = vec![];
        for e in &self.list {
            let mut vv = vec![];
            for c in e.status().chars() {
                vv.push(BufElem::Char(c));
            }
            vv.push(BufElem::Eol);
            v.push(vv);
        }
        self.rb = read_buffer::ReadBuffer::new(v, self.message_box.clone());
    }
    pub fn set(&mut self, page: Rc<Page>) {
        self.refresh_buffer();
        self.current = page;
    }
    fn select(&mut self, i: usize) {
        let e = self.list.remove(i);
        self.list.insert(0, e);
        self.set(self.list[0].clone());
    }
    fn delete(&mut self, i: usize) {
        if self.list[i].kind() == PageKind::Help {
            return;
        }
        self.list.remove(i);
        self.refresh_buffer()
    }
    pub fn push(&mut self, page: Rc<Page>) {
        let pos0 = self.list.iter().position(|e| e.id() == page.id());
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
    pub fn pop_and_push(&mut self, e: Rc<Page>) {
        self.list.remove(0);
        self.list.insert(0, e);
        self.select(0);
    }
    pub fn pop(&mut self) {
        self.list.remove(0);
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
        for i in self.list.iter().position(|e| e.kind() == PageKind::Directory) {
            self.select(i);
        }
        INIT.to_owned()
    }
    pub fn eff_select_cur_buffer(&mut self, _: Key) -> String {
        for i in self.list.iter().position(|e| e.kind() == PageKind::Buffer) {
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

pub fn mk_controller(x: Rc<RefCell<Navigator>>) -> controller::ControllerFSM {
    use crate::Key::*;
    let mut g = controller::GraphImpl::new();
    g.add_edge(INIT, Char('k'), Rc::new(CursorUp(x.clone())));
    g.add_edge(INIT, Char('j'), Rc::new(CursorDown(x.clone())));
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
impl view::ViewGen for ViewGen {
    fn gen(&self, region: view::Area) -> Box<view::View> {
        self.x.borrow_mut().rb.stabilize_cursor();
        self.x.borrow_mut().rb.adjust_window(region.width, region.height);
        self.x.borrow_mut().update_cache();

        let (lineno_area, navi_area) = region.split_horizontal(view::LINE_NUMBER_W);
        let navi_view = view::ToView::new(&self.x.borrow().rb.buf, self.x.borrow().rb.current_window());
        let navi_view = view::AddCursor::new(
            navi_view,
            Some(self.x.borrow().rb.cursor), // tmp: the cursor is always visible
        );
        let navi_view = view::TranslateView::new(
            navi_view,
            navi_area.col as i32 - self.x.borrow().rb.window.col() as i32,
            navi_area.row as i32 - self.x.borrow().rb.window.row() as i32,
        );

        let lineno_range = self.x.borrow().rb.lineno_range();
        let lineno_view = view::LineNumber {
            from: lineno_range.start+1,
            to: lineno_range.end,
        };

        let view = view::MergeHorizontal {
            left: lineno_view,
            right: navi_view,
            col_offset: navi_area.col,
        };

        Box::new(view)
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
    fn view_gen(&self) -> &Box<view::ViewGen> {
        &self.view_gen
    }
    fn status(&self) -> String {
        "[Navigator]".to_owned()
    }
    fn kind(&self) -> PageKind {
        PageKind::Other
    }
    fn id(&self) -> String {
        "navigator".to_owned()
    }
    fn message(&self) -> MessageBox {
        self.x.borrow().message_box.clone()
    }
}
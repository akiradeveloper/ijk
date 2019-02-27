use termion::event::Key as TermKey;
use crate::Key as Key;
use crate::controller::KeyController;
use termion::clear;
use termion::cursor;
use termion::color;
use termion::event::Event;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use std::cell::RefCell;
use std::rc::Rc;
use std::{thread, time};
use crate::Key::*;
use crate::edit_buffer as EB;
use crate::screen::*;
use crate::BufElem;

pub struct Editor {
    cur_ctrl: Rc<RefCell<EB::Controller>>,
    eb: Rc<RefCell<EB::EditBuffer>>, // tmp instead of view
}

impl Editor {
    pub fn new(ctrl: Rc<RefCell<EB::Controller>>, edit_buffer: Rc<RefCell<EB::EditBuffer>>) -> Self {
        Self {
            cur_ctrl: ctrl,
            eb: edit_buffer,
        }
    }
    // fn draw() {}
    pub fn run(&mut self) {
        // let stdin = stdin();
        let stdin = termion::async_stdin();

        let (term_w, term_h) = termion::terminal_size().unwrap();
        // let (term_w, term_h) = (15, 15);
        let mut screen = Screen::new(term_w as usize, term_h as usize);
        let mut vfilter = crate::visibility_filter::VisibilityFilter::new(self.eb.borrow().cursor);
        let window_col: u16 = 0; let window_row: u16 = 0;
        // let window_col: u16 = 5; let window_row: u16 = 5;

        vfilter.resize(term_w as usize, term_h as usize);

        let mut keys = stdin.keys();

        loop {
            // draw
            vfilter.adjust(self.eb.borrow().cursor);

            let drawable = vfilter.apply(&self.eb.borrow());

            screen.clear();
            for row in 0 .. drawable.buf.len() {
                let line = &drawable.buf[row];
                for col in 0 .. line.len() {
                    let e = drawable.buf[row][col].clone();
                    let as_cursor = EB::Cursor { row: row, col: col };
                    let in_visual_range = drawable.selected.map(|vr| vr.start <= as_cursor && as_cursor < vr.end).unwrap_or(false);
                    let c0 = match e {
                        Some(BufElem::Char(c)) => Some(c),
                        Some(BufElem::Eol) => Some(' '),
                        None => None
                    };
                    let fg = Color::White;
                    let bg = if in_visual_range {
                        Color::Blue
                    } else {
                        Color::Black
                    };
                    for c in c0 {
                        screen.draw(col, row, c, Style(fg, bg));
                    }
                }
            }
            screen.move_cursor(drawable.cursor.col, drawable.cursor.row);
            screen.present();

            match keys.next() {
                Some(Ok(TermKey::Ctrl('z'))) => break,
                // Some(Ok(TermKey::Ctrl('b'))) => self.switch_to_current_buffer(),
                other_key => {
                    let kk = match other_key {
                        Some(Ok(TermKey::Ctrl('c'))) => Key::Esc,
                        Some(Ok(TermKey::Backspace)) => Key::Backspace,
                        Some(Ok(TermKey::Ctrl(c))) => Key::Ctrl(c),
                        Some(Ok(TermKey::Char(c))) => Key::Char(c),
                        _ => {
                            thread::sleep(time::Duration::from_millis(100));
                            continue
                        },
                    };
                    self.cur_ctrl.borrow_mut().receive(kk);
                }
            }
        }
    }
}
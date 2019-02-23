use std;
use std::cell::RefCell;
use std::io;
use std::io::{BufWriter, Write};

use termion;
use termion::color;
use termion::raw::{IntoRawMode, RawTerminal};
use termion::screen::AlternateScreen;

pub struct Screen {
    out: RefCell<AlternateScreen<RawTerminal<BufWriter<io::Stdout>>>>,
    buf: RefCell<Vec<(Style, char)>>,
    w: usize,
    h: usize,
    cursor_pos: (usize, usize),
    cursor_visible: bool
}

static EMPTY: char = ' ';
static DEFAULT: (Style, char) = (Style(Color::Black, Color::Black), EMPTY);

impl Screen {
    pub fn new(w: usize, h: usize) -> Self {
        let buf = std::iter::repeat(DEFAULT)
            .take(w as usize * h as usize)
            .collect();
        Screen {
            out: RefCell::new(AlternateScreen::from(
                BufWriter::with_capacity(1 << 14, io::stdout())
                    .into_raw_mode()
                    .unwrap(),
            )),
            buf: RefCell::new(buf),
            w: w,
            h: h,
            cursor_pos: (0, 0),
            cursor_visible: true
        }
    }

    pub fn clear(&self) {
        for cell in self.buf.borrow_mut().iter_mut() {
            *cell = DEFAULT;
        }
    }

    pub fn resize(&mut self, w: usize, h: usize) {
        self.w = w;
        self.h = h;
        self.buf
            .borrow_mut()
            .resize(w * h, DEFAULT);
    }

    pub fn present(&self) {
        let mut out = self.out.borrow_mut();
        let buf = self.buf.borrow();

        let mut last_style = DEFAULT.0;
        write!(out, "{}", last_style).unwrap();

        for y in 0..self.h {
            let mut x = 0;
            write!(out, "{}", termion::cursor::Goto(1, y as u16 + 1)).unwrap();
            for x in 0..self.w {
                let (style, ref c) = buf[y * self.w + x];
                if style != last_style {
                    write!(out, "{}", style).unwrap();
                    last_style = style;
                }
                write!(out, "{}", c).unwrap();
            }
        }

        if self.cursor_visible {
            let (cx, cy) = self.cursor_pos;
            write!(
                out,
                "{}{}",
                termion::cursor::Goto(1 + cx as u16, 1 + cy as u16),
                termion::cursor::Show,
            )
            .unwrap();
        }

        out.flush().unwrap();
    }

    pub fn draw(&self, x: usize, y: usize, c: char, style: Style) {
        if x < self.w && y < self.h {
            let mut buf = self.buf.borrow_mut();
            if x < self.w {
                buf[y * self.w + x] = (style, c);
            }
        }
    }

    pub fn hide_cursor(&mut self) {
        self.cursor_visible = false;
    }

    pub fn show_cursor(&mut self) {
        self.cursor_visible = true;
    }

    pub fn move_cursor(&mut self, x: usize, y: usize) {
        self.cursor_pos = (x, y);
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        write!(
            self.out.borrow_mut(),
            "{}{}{}",
            color::Fg(color::Reset),
            color::Bg(color::Reset),
            termion::clear::All,
        ).unwrap();
        self.show_cursor();
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Color {
    Black,
    Blue,
    Cyan,
    Green,
    LightBlack,
    LightBlue,
    LightCyan,
    LightGreen,
    LightMagenta,
    LightRed,
    LightWhite,
    LightYellow,
    Magenta,
    Red,
    Rgb(u8, u8, u8),
    White,
    Yellow,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Style(pub Color, pub Color); // Fg, Bg

impl std::fmt::Display for Style {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.0 {
            Color::Black => write!(f, "{}", color::Fg(color::Black)),
            Color::Blue => write!(f, "{}", color::Fg(color::Blue)),
            Color::Cyan => write!(f, "{}", color::Fg(color::Cyan)),
            Color::Green => write!(f, "{}", color::Fg(color::Green)),
            Color::LightBlack => write!(f, "{}", color::Fg(color::LightBlack)),
            Color::LightBlue => write!(f, "{}", color::Fg(color::LightBlue)),
            Color::LightCyan => write!(f, "{}", color::Fg(color::LightCyan)),
            Color::LightGreen => write!(f, "{}", color::Fg(color::LightGreen)),
            Color::LightMagenta => write!(f, "{}", color::Fg(color::LightMagenta)),
            Color::LightRed => write!(f, "{}", color::Fg(color::LightRed)),
            Color::LightWhite => write!(f, "{}", color::Fg(color::LightWhite)),
            Color::LightYellow => write!(f, "{}", color::Fg(color::LightYellow)),
            Color::Magenta => write!(f, "{}", color::Fg(color::Magenta)),
            Color::Red => write!(f, "{}", color::Fg(color::Red)),
            Color::Rgb(r, g, b) => write!(f, "{}", color::Fg(color::Rgb(r, g, b))),
            Color::White => write!(f, "{}", color::Fg(color::White)),
            Color::Yellow => write!(f, "{}", color::Fg(color::Yellow)),
        }?;

        match self.1 {
            Color::Black => write!(f, "{}", color::Bg(color::Black)),
            Color::Blue => write!(f, "{}", color::Bg(color::Blue)),
            Color::Cyan => write!(f, "{}", color::Bg(color::Cyan)),
            Color::Green => write!(f, "{}", color::Bg(color::Green)),
            Color::LightBlack => write!(f, "{}", color::Bg(color::LightBlack)),
            Color::LightBlue => write!(f, "{}", color::Bg(color::LightBlue)),
            Color::LightCyan => write!(f, "{}", color::Bg(color::LightCyan)),
            Color::LightGreen => write!(f, "{}", color::Bg(color::LightGreen)),
            Color::LightMagenta => write!(f, "{}", color::Bg(color::LightMagenta)),
            Color::LightRed => write!(f, "{}", color::Bg(color::LightRed)),
            Color::LightWhite => write!(f, "{}", color::Bg(color::LightWhite)),
            Color::LightYellow => write!(f, "{}", color::Bg(color::LightYellow)),
            Color::Magenta => write!(f, "{}", color::Bg(color::Magenta)),
            Color::Red => write!(f, "{}", color::Bg(color::Red)),
            Color::Rgb(r, g, b) => write!(f, "{}", color::Bg(color::Rgb(r, g, b))),
            Color::White => write!(f, "{}", color::Bg(color::White)),
            Color::Yellow => write!(f, "{}", color::Bg(color::Yellow)),
        }
    }
}
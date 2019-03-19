// https://github.com/hatoo/Accepted

use std::io::{Read, Write};
use std::process;
use std::process::Command;
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;

use crate::BufElem;

lazy_static! {
    pub static ref SINGLETON: Clipboard = Clipboard::new();
}

#[derive(Clone)]
pub enum Type {
    Range(Vec<BufElem>),
    Line(Vec<BufElem>),
}

fn to_str(x: &[BufElem]) -> String {
    let mut s = String::new();
    for e in x {
        let c = match *e {
            BufElem::Char(c) => c,
            BufElem::Eol => '\n',
        };
        s.push(c)
    }
    s
}
fn from_str(x: &str) -> Vec<BufElem> {
    let mut v = vec![];
    for c in x.chars() {
        let e = match c {
            '\n' => BufElem::Eol,
            c => BufElem::Char(c),
        };
        v.push(e)
    }
    v
}

struct ClipboardImpl {
    x: Option<Type>,
}
impl ClipboardImpl {
    fn new() -> Self {
        Self { x: None }
    }
    fn copy(&mut self, x: Type) {
        let v = match &x {
            Type::Range(a) => a,
            Type::Line(a) => a,
        };
        clipboard_copy(&to_str(&v));
        self.x = Some(x.clone())
    }
    fn paste(&mut self) -> Option<Type> {
        self.x.clone()
    }
}

pub struct Clipboard {
    imp: Arc<Mutex<ClipboardImpl>>,
}
impl Clipboard {
    pub fn new() -> Self {
        Self {
            imp: Arc::new(Mutex::new(ClipboardImpl::new()))
        }
    }
    pub fn copy(&self, x: Type) {
        self.imp.lock().unwrap().copy(x);
    }
    pub fn paste(&self) -> Option<Type> {
        self.imp.lock().unwrap().paste()
    }
}

fn clipboard_copy(s: &str) -> bool {
    if let Ok(mut p) = Command::new("pbcopy")
        .stdin(process::Stdio::piped())
        .spawn()
        .or_else(|_| {
            Command::new("win32yank")
                .arg("-i")
                .stdin(process::Stdio::piped())
                .spawn()
        })
        .or_else(|_| {
            Command::new("win32yank.exe")
                .arg("-i")
                .stdin(process::Stdio::piped())
                .spawn()
        })
        .or_else(|_| {
            Command::new("xsel")
                .arg("-bi")
                .stdin(process::Stdio::piped())
                .spawn()
        })
        .or_else(|_| {
            Command::new("xclip")
                .arg("-i")
                .stdin(process::Stdio::piped())
                .spawn()
        })
    {
        if let Some(mut stdin) = p.stdin.take() {
            write!(stdin, "{}", s).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
            return true;
        }
    }
    false
}

fn clipboard_paste() -> Option<String> {
    if let Ok(mut p) = Command::new("pbpaste")
        .stdout(process::Stdio::piped())
        .spawn()
        .or_else(|_| {
            Command::new("win32yank")
                .arg("-o")
                .stdout(process::Stdio::piped())
                .spawn()
        })
        .or_else(|_| {
            Command::new("win32yank.exe")
                .arg("-o")
                .stdout(process::Stdio::piped())
                .spawn()
        })
        .or_else(|_| {
            Command::new("xsel")
                .arg("-bo")
                .stdout(process::Stdio::piped())
                .spawn()
        })
        .or_else(|_| {
            Command::new("xclip")
                .arg("-o")
                .stdout(process::Stdio::piped())
                .spawn()
        })
    {
        if let Some(mut stdout) = p.stdout.take() {
            let mut buf = String::new();
            stdout.read_to_string(&mut buf).ok()?;
            return Some(buf);
        }
    }
    None
}
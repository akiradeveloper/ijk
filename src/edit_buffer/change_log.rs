use crate::{BufElem, Cursor};
use super::undo_buffer::UndoBuffer;
use std::time::Instant;

#[derive(Clone)]
pub struct ChangeLog {
    clock: Instant,
    pub at: Cursor,
    pub deleted: Vec<BufElem>,
    pub inserted: Vec<BufElem>,
}
impl ChangeLog {
    pub fn new(at: Cursor, deleted: Vec<BufElem>, inserted: Vec<BufElem>) -> Self {
        let clock = Instant::now();
        Self {
            clock: clock,
            at: at,
            deleted: deleted,
            inserted: inserted,
        }
    }
    pub fn swap(&self) -> Self {
        Self {
            clock: self.clock,
            at: self.at,
            deleted: self.inserted.clone(),
            inserted: self.deleted.clone(),
        }
    }
}

pub struct ChangeLogBuffer {
    buf: UndoBuffer<ChangeLog>,
}
impl ChangeLogBuffer {
    pub fn new() -> Self {
        Self {
            buf: UndoBuffer::new(20),
        }
    }
    pub fn clock(&self) -> Option<Instant> {
        self.buf.peek().map(|x| x.clock)
    }
    pub fn peek(&self) -> Option<&ChangeLog> {
        self.buf.peek()
    }
    pub fn push(&mut self, x: ChangeLog) {
        self.buf.push(x);
    }
    pub fn pop_undo(&mut self) -> Option<ChangeLog> {
        self.buf.pop_undo()
    }
    pub fn pop_redo(&mut self) -> Option<ChangeLog> {
        self.buf.pop_redo()
    }
}

use crate::{BufElem, Cursor};
use super::undo_buffer::UndoBuffer;
use std::time::Instant;

#[derive(Clone)]
pub struct ChangeLog {
    tick: Instant,
    pub at: Cursor,
    pub deleted: Vec<BufElem>,
    pub inserted: Vec<BufElem>,
}
impl ChangeLog {
    pub fn new(at: Cursor, deleted: Vec<BufElem>, inserted: Vec<BufElem>) -> Self {
        let tick = Instant::now();
        Self {
            tick: tick,
            at: at,
            deleted: deleted,
            inserted: inserted,
        }
    }
    pub fn swap(&self) -> Self {
        Self {
            tick: self.tick,
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
    pub fn tick(&self) -> Option<Instant> {
        self.buf.peek().map(|x| x.tick)
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
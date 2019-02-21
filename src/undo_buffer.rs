use std::collections::VecDeque;

/// save 1,2,3,4,5
/// undo 5,4
/// redo 4,5
/// 
/// undo_queue: [1,2,3]
/// redo_stack: [5,4]

pub struct UndoBuffer<T: Clone> {
    capacity: usize,
    undo_queue: VecDeque<T>,
    redo_stack: Vec<T>,
}

impl <T: Clone> UndoBuffer<T> {
    pub fn new(cap: usize) -> UndoBuffer<T> {
        UndoBuffer {
            capacity: cap,
            undo_queue: VecDeque::new(),
            redo_stack: Vec::new(),
        }
    }
    pub fn save(&mut self, x: T) {
        self.redo_stack.clear();
        self.undo_queue.push_back(x);
        while self.undo_queue.len() > self.capacity {
            self.undo_queue.pop_front();
        }
    }
    pub fn pop_undo(&mut self) -> Option<T> {
        let x0 = self.undo_queue.pop_back();
        for x in x0.clone() {
            self.redo_stack.push(x);
        }
        x0
    }
    pub fn pop_redo(&mut self) -> Option<T> {
        let x0 = self.redo_stack.pop();
        for x in x0.clone() {
            self.undo_queue.push_back(x);
        }
        x0
    }
}

#[test]
fn test_undo_buffer() {
    let mut ub = UndoBuffer::new(3);
    ub.save(1);
    ub.save(2);
    ub.save(3);
    ub.save(4); // [2,3,4]+[]
    assert!(ub.pop_redo().is_none());
    assert_eq!(ub.pop_undo(), Some(4)); // [2,3]+[4]
    assert_eq!(ub.pop_undo(), Some(3)); // [2]+[3,4]
    assert_eq!(ub.pop_redo(), Some(3)); // [2,3]+[4]
    assert_eq!(ub.pop_redo(), Some(4)); // [2,3,4]+[]
}
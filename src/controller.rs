use crate::Key;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::cell::RefCell;

pub trait Effect {
    fn run(&self, k: Key) -> String;
}

#[macro_export]
macro_rules! def_effect {
    ($eff_name:ident, $t:ty, $fun_name:ident) => {
        struct $eff_name(Rc<RefCell<$t>>);
        impl Effect for $eff_name {
            fn run(&self, k: Key) -> String {
                let next_state: String = self.0.borrow_mut().$fun_name(k);
                self.0.borrow_mut().state = next_state.clone();
                next_state
            }
        }
    };
}

struct Edge {
    matcher: Key,
    eff: Rc<Effect>,
}
impl Edge {
    fn matches(&self, k: &Key) -> bool {
        match self.matcher.clone() {
            Key::CharRange(a, b) => match *k {
                Key::Char(c) => a <= c && c <= b,
                _ => false,
            },
            Key::Otherwise => true,
            mhr => k.clone() == mhr,
        }
    }
}

pub struct Graph {
    edges: HashMap<String, Vec<Edge>>,
}
impl Graph {
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
        }
    }
    fn ensure_edge_vec(&mut self, from: &str) {
        if !self.edges.contains_key(from) {
            self.edges.insert(from.to_owned(), vec![]);
        }
    }
    pub fn add_edge(&mut self, from: &str, matcher: Key, eff: Rc<Effect>) {
        self.ensure_edge_vec(from);
        let v = self.edges.get_mut(from).unwrap();
        v.push(Edge {
            matcher: matcher,
            eff: eff,
        });
    }
    fn find_effect(&self, from: &str, k: &Key) -> Option<Rc<Effect>> {
        if !self.edges.contains_key(from) {
            return None;
        }
        let v = self.edges.get(from).unwrap();
        v.iter()
            .find(|e| e.matches(&k))
            .map(|x| x.eff.clone())
    }
}

pub trait Controller {
    fn receive(&self, k: Key);
}

pub struct NullController {}
impl Controller for NullController {
    fn receive(&self, k: Key) {}
}

pub struct ControllerFSM {
    cur: RefCell<String>,
    g: Box<Graph>,
}
impl ControllerFSM {
    pub fn new(s: &str, g: Box<Graph>) -> Self {
        Self {
            cur: RefCell::new(s.to_owned()),
            g: g,
        }
    }
}
impl Controller for ControllerFSM {
    fn receive(&self, k: Key) {
        let cur = self.cur.borrow().clone();
        let eff0 = self.g.find_effect(&cur, &k);
        match eff0 {
            Some(eff) => {
                let to = eff.run(k);
                *self.cur.borrow_mut() = to;
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Controller, ControllerFSM, Effect, Graph};
    use crate::Key;
    use std::cell::RefCell;
    use std::rc::Rc;
    // for test
    struct AppendY(Rc<RefCell<Vec<char>>>);
    impl Effect for AppendY {
        fn run(&self, k: Key) -> String {
            self.0.borrow_mut().push('y');
            "no".to_owned()
        }
    }
    struct AppendN(Rc<RefCell<Vec<char>>>);
    impl Effect for AppendN {
        fn run(&self, k: Key) -> String {
            self.0.borrow_mut().push('n');
            "yes".to_owned()
        }
    }

    #[test]
    fn test_controller() {
        use crate::Key::*;

        let buf = Rc::new(RefCell::new(vec![]));

        let append_y = AppendY(buf.clone());
        let mut g = Graph::new();
        g.add_edge("yes", Char('y'), Rc::new(append_y));

        let append_n = AppendN(buf.clone());
        g.add_edge("no", Char('n'), Rc::new(append_n));

        let ctrl = ControllerFSM::new("yes", Box::new(g));
        ctrl.receive(Char('y'));
        assert_eq!(*buf.borrow(), ['y']);
        ctrl.receive(Char('y'));
        assert_eq!(*buf.borrow(), ['y']);
        ctrl.receive(Char('n'));
        assert_eq!(*buf.borrow(), ['y', 'n']);
    }
}

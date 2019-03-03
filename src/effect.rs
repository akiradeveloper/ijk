use crate::Key;
use std::collections::HashMap;

trait Effect {
    fn run(&self) -> ();
}

struct Edge {
    matcher: Key,
    eff: Rc<Effect>,
    to: String,
}

trait Graph {
    fn find_effect(&self, from: &str, k: Key) -> Option<(Rc<Effect>, String)>;
}

struct GraphImpl {
    nodes: Vec<String>,
    edges: HashMap<String, Vec<Edge>>,
}
impl GraphImpl {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: HashMap::new(),
        }
    }
    fn add_node(&mut self, id: &str) {
    }
    fn add_edge(&mut self, from: &str, to: &str, eff: Rc<Effect>) { }
}
impl Graph for GraphImpl {
    fn find_effect(&self, from: &str, k: Key) -> Option<(Rc<Effect>, String)> {
        unimplemented!()
    }
}

struct ComposedGraph<G1,G2> {
    g1: G1,
    g2: G2,
}
impl <G1,G2> Graph for ComposedGraph<G1,G2> where G1: Graph, G2: Graph {
    fn find_effect(&self, from: &str, k: Key) -> Option<(Rc<Effect>, String)> {
        self.g1.find_effect(from.clone(), k.clone()).or(self.g2.find_effect(from, k))
    }
}

fn compose<G1: Graph,G2: Graph>(g1: G1, g2: G2) -> ComposedGraph<G1,G2> {
    ComposedGraph {
        g1, g2,
    }
}

struct Controller {
    cur: String,
    g: Box<Graph>,
}
impl Controller {
    fn receive(&mut self, k: Key) {
        let eff0 = self.g.find_effect(&self.cur, k);
        for (eff, to) in eff0 {
            eff.run();
            self.cur = to;
        }
    }
}

use std::rc::Rc;
use std::cell::RefCell;
// for test
struct AppendY(Rc<RefCell<Vec<char>>>);
impl Effect for AppendY {
    fn run(&self) -> () {
        self.0.borrow_mut().push('y')
    }
}
struct AppendN(Rc<RefCell<Vec<char>>>);
impl Effect for AppendN {
    fn run(&self) -> () {
        self.0.borrow_mut().push('n')
    }
}

#[test]
fn test_controller() {
    use crate::Key::*;

    let buf = Rc::new(RefCell::new(vec![]));
    let append_y = AppendY(buf.clone());
    let append_n = AppendN(buf.clone());
    let mut g = GraphImpl::new();
    g.add_node("yes");
    g.add_node("no");
    g.add_edge("yes", "no", Rc::new(append_y));
    g.add_edge("no", "yes", Rc::new(append_n));

    let mut ctrl = Controller {
        cur: "yes".to_owned(),
        g: Box::new(g),
    };
    ctrl.receive(Char('y'));
    assert_eq!(*buf.borrow(), ['y']);
    ctrl.receive(Char('y'));
    assert_eq!(*buf.borrow(), ['y']);
    ctrl.receive(Char('n'));
    assert_eq!(*buf.borrow(), ['y', 'n']);
}
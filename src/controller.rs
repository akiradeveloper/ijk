use crate::Key;
use std::collections::{HashSet, HashMap};

pub trait Effect {
    fn run(&self, k: Key) -> ();
}

struct Edge {
    matcher: Key,
    eff: Rc<Effect>,
    to: String,
}
impl Edge {
    fn matches(&self, k: &Key) -> bool {
        match self.matcher.clone() {
            Key::CharRange(a,b) => match *k {
                Key::Char(c) => a <= c && c <= b,
                _ => false
            },
            Key::Otherwise => true,
            mhr => k.clone() == mhr,
        }
    }
}

pub trait Graph {
    fn find_effect(&self, from: &str, k: &Key) -> Option<(Rc<Effect>, String)>;
}

pub struct GraphImpl {
    edges: HashMap<String, Vec<Edge>>,
}
impl GraphImpl {
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
    fn add_edge(&mut self, from: &str, to: &str, matcher: Key, eff: Rc<Effect>) { 
        self.ensure_edge_vec(from);
        let v = self.edges.get_mut(from).unwrap();
        v.push(Edge {
            matcher: matcher,
            eff: eff,
            to: to.to_owned(),
        });
    }
}
impl Graph for GraphImpl {
    fn find_effect(&self, from: &str, k: &Key) -> Option<(Rc<Effect>, String)> {
        if !self.edges.contains_key(from) {
            return None;
        }
        let v = self.edges.get(from).unwrap();
        v.iter().find(|e| e.matches(&k)).map(|x| (x.eff.clone(), x.to.clone()))
    }
}

struct ComposedGraph<G1,G2> {
    g1: G1,
    g2: G2,
}
impl <G1,G2> Graph for ComposedGraph<G1,G2> where G1: Graph, G2: Graph {
    fn find_effect(&self, from: &str, k: &Key) -> Option<(Rc<Effect>, String)> {
        self.g1.find_effect(from.clone(), k).or(self.g2.find_effect(from, k))
    }
}

fn compose<G1: Graph,G2: Graph>(g1: G1, g2: G2) -> ComposedGraph<G1,G2> {
    ComposedGraph {
        g1, g2,
    }
}

pub struct Controller {
    pub cur: String,
    pub g: Box<Graph>,
}
impl Controller {
    pub fn receive(&mut self, k: Key) {
        let eff0 = self.g.find_effect(&self.cur, &k);
        match eff0 {
            Some((eff, to)) => {
                eff.run(k);
                self.cur = to;
            },
            None => {}
        }
    }
}

use std::rc::Rc;
use std::cell::RefCell;
// for test
struct AppendY(Rc<RefCell<Vec<char>>>);
impl Effect for AppendY {
    fn run(&self, k: Key) -> () {
        self.0.borrow_mut().push('y')
    }
}
struct AppendN(Rc<RefCell<Vec<char>>>);
impl Effect for AppendN {
    fn run(&self, k: Key) -> () {
        self.0.borrow_mut().push('n')
    }
}

#[test]
fn test_controller() {
    use crate::Key::*;

    let buf = Rc::new(RefCell::new(vec![]));

    let append_y = AppendY(buf.clone());
    let mut g1 = GraphImpl::new();
    g1.add_edge("yes", "no", Char('y'), Rc::new(append_y));

    let append_n = AppendN(buf.clone());
    let mut g2 = GraphImpl::new();
    g2.add_edge("no", "yes", Char('n'), Rc::new(append_n));

    let g = compose(g1, g2);

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
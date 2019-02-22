use crate::*;

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
struct Trans {
    e: Edge,
    n: Node
}
#[derive(Debug)]
struct NodeImpl {
    name: String,
    possible_trans: Vec<Trans>,
}
impl NodeImpl {
    fn new(name: &str) -> NodeImpl {
        NodeImpl {
            name: name.to_owned(),
            possible_trans: vec![]
        }
    }
    fn add_trans(&mut self, e: Edge, n: &Node) {
        self.possible_trans.push(Trans{e:e, n:n.clone()});
    }
}
#[derive(Debug, Clone)]
pub struct Node {
    node_impl: Rc<RefCell<NodeImpl>>
}
impl Node {
    pub fn new(name: &str) -> Node {
        Node {
            node_impl: Rc::new(RefCell::new(NodeImpl::new(name)))
        }
    }
    pub fn name(&self) -> String {
        self.node_impl.borrow().name.clone()
    }
    pub fn add_trans(&self, e: Edge, n: &Node) {
        self.node_impl.borrow_mut().add_trans(e, n)
    }
    fn find_trans(&self, k: &Key) -> Option<Trans> {
        self.node_impl.borrow().possible_trans.iter().find(|tr| tr.e.matches(&k)).cloned() 
    }
}
#[derive(Debug, Clone)]
pub struct Edge {
    matcher: Key
}
impl Edge {
    pub fn new(matcher: Key) -> Edge {
        Edge {
            matcher: matcher
        }
    }
    fn matches(&self, k: &Key) -> bool {
        match self.matcher.clone() {
            Key::CharRange(a,b) => match k.clone() {
                Key::Char(c) => a <= c && c <= b,
                _ => false
            },
            Key::Otherwise => true,
            mhr => k.clone() == mhr,
        }
    }
}
#[derive(Debug)]
pub struct Parser {
    pub cur_node: Node,
    pub prev_node: Option<Node>,
    pub rec: VecDeque<Key>
}
impl Parser {
    pub fn new(init_node: &Node) -> Parser {
        Parser {
            cur_node: init_node.clone(),
            prev_node: None,
            rec: VecDeque::new()
        }
    }
    pub fn clear_rec(&mut self) {
        self.rec = VecDeque::new();
    }
    pub fn feed(&mut self, k: Key) {
        let trans0 = self.cur_node.find_trans(&k);
        let trans = trans0.unwrap(); // hope that user inputs are all perfect
        let cur_node = self.cur_node.clone();
        self.cur_node = trans.n;
        self.prev_node = Some(cur_node);
        self.rec.push_back(k);
    }
}
#[test]
fn test_key_eq() {
    let a = Key::Char('0');
    let b = Key::Char('7');
    assert_ne!(a, b);
}
#[test]
fn test_node() {
    use crate::Key::*;

    let a = Node::new("a");
    let b = Node::new("b");

    a.add_trans(Edge::new(Char('0')), &a);
    a.add_trans(Edge::new(CharRange('1','9')), &b);

    let tr = a.find_trans(&Char('7')).unwrap();
    assert_eq!(tr.n.name(), b.name());
}
#[test]
fn test_vi_command_mode() {
    use crate::Key::*;

    // make graph
    let init = Node::new("init");
    let num = Node::new("num");
    init.add_trans(Edge::new(Char('G')), &init);
    init.add_trans(Edge::new(Char('0')), &init);
    init.add_trans(Edge::new(CharRange('1','9')), &num);
    num.add_trans(Edge::new(CharRange('0','9')), &num);
    num.add_trans(Edge::new(Char('G')), &init);

    let mut parser = Parser::new(&init);
    parser.feed(Char('0'));
    parser.feed(Char('G'));
    assert_eq!(parser.cur_node.name(), "init");
    parser.clear_rec();

    parser.feed(Char('7'));
    assert_eq!(parser.cur_node.name(), "num");
    parser.feed(Char('0'));
    assert_eq!(parser.cur_node.name(), "num");
    parser.feed(Char('G'));
    assert_eq!(parser.cur_node.name(), "init");
    assert_eq!(parser.rec, [Char('7'),Char('0'),Char('G')]);
}

#[test]
fn test_otherwise() {
    use crate::Key::*;

    let init = Node::new("init");
    let other = Node::new("other");
    init.add_trans(Edge::new(Char('a')), &init);
    init.add_trans(Edge::new(Otherwise), &other);

    let mut parser = Parser::new(&init);
    parser.feed(Char('a'));
    assert_eq!(parser.cur_node.name(), "init");
    parser.feed(Char('b'));
    assert_eq!(parser.cur_node.name(), "other");
}
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::VecDeque;

#[derive(Debug, PartialEq, Clone)]
enum Key {
    Left,
    Right,
    Up,
    Down,
    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    Backspace,
    Esc,
    F(u8),
    Char(char), // termion passes space as Char(' ') and tab as Char('\t')
    Alt(char),
    Ctrl(char),

    CharRange(char,char), // only for matcher. inclusive like ...
}
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
struct Node {
    node_impl: Rc<RefCell<NodeImpl>>
}
impl Node {
    fn new(name: &str) -> Node {
        Node {
            node_impl: Rc::new(RefCell::new(NodeImpl::new(name)))
        }
    }
    fn name(&self) -> String {
        self.node_impl.borrow().name.clone()
    }
    fn add_trans(&self, e: Edge, n: &Node) {
        self.node_impl.borrow_mut().add_trans(e, n)
    }
    fn find_trans(&self, k: &Key) -> Option<Trans> {
        self.node_impl.borrow().possible_trans.iter().find(|tr| tr.e.matches(&k)).cloned() 
    }
}
#[derive(Debug, Clone)]
struct Edge {
    matcher: Key
}
impl Edge {
    fn new(matcher: Key) -> Edge {
        Edge {
            matcher: matcher
        }
    }
    fn matches(&self, key: &Key) -> bool {
        match self.matcher.clone() {
            Key::CharRange(a,b) => match key.clone() {
                Key::Char(c) => a <= c && c <= b,
                _ => false
            },
            k => k == self.matcher
        }
    }
}
#[derive(Debug)]
struct Parser {
    cur_node: Node,
    prev_node: Option<Node>,
    rec: VecDeque<Key>
}
impl Parser {
    fn new(init_node: &Node) -> Parser {
        Parser {
            cur_node: init_node.clone(),
            prev_node: None,
            rec: VecDeque::new()
        }
    }
    fn reset(&mut self, init_node: &Node) {
        self.cur_node = init_node.clone();
        self.prev_node = None;
        self.rec = VecDeque::new();
    }
    fn feed(&mut self, k: Key) {
        let trans0 = self.cur_node.find_trans(&k);
        let trans = trans0.unwrap(); // hope that user inputs are all perfect
        let cur_node = self.cur_node.clone();
        self.cur_node = trans.n;
        self.prev_node = Some(cur_node);
        self.rec.push_back(k);
    }
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

    parser.reset(&init);
    parser.feed(Char('7'));
    assert_eq!(parser.cur_node.name(), "num");
    parser.feed(Char('0'));
    assert_eq!(parser.cur_node.name(), "num");
    parser.feed(Char('G'));
    assert_eq!(parser.cur_node.name(), "init");
    assert_eq!(parser.rec, [Char('7'),Char('0'),Char('G')]);
}
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::VecDeque;

#[derive(PartialEq, Clone, Debug)]
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

    CharRange(char, char), // only for matcher. inclusive like ...
}
#[derive(Clone)]
struct Trans {
    e: Edge,
    n: Node
}
struct NodeImpl {
    name: String,
    possible_trans: Vec<Trans>,
}
impl NodeImpl {
    fn new(s: &str) -> NodeImpl {
        NodeImpl {
            name: s.to_owned(),
            possible_trans: vec![]
        }
    }
    fn add_trans(&mut self, e: Edge, n: &Node) {
        self.possible_trans.push(Trans{e:e, n:n.clone()});
    }
}
type Node = Rc<RefCell<NodeImpl>>;
#[derive(Clone)]
struct Edge {
    matcher: Key
}
impl Edge {
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
        let trans0 = self.cur_node.borrow().possible_trans.iter().find(|tr| tr.e.matches(&k)).cloned();
        let trans = trans0.unwrap(); // hope that user inputs are all perfect
        let cur_node = self.cur_node.clone();
        self.cur_node = trans.n;
        self.prev_node = Some(cur_node);
        self.rec.push_back(k);
    }
}
#[test]
fn test_vi_command_mode() {

}
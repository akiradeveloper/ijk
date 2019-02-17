use std::rc::Rc;
use std::cell::RefCell;
use std::collections::VecDeque;

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
    Fn(u8),
    Char(char),
    Alt(char),
    Ctrl(char),

    // only for matcher. inclusive like ...
    CharRange(char, char),
}
struct Trans {
    e: Edge, n: Node
}
struct NodeImpl {
    possible_trans: Vec<Trans>,
}
impl NodeImpl {
    fn add_trans(&mut self, e: Edge, n: &Node) {

    }
}
type Node = Rc<RefCell<NodeImpl>>;
struct Edge {
    matcher: Key
}
impl Edge {
    fn matches(&self, key: &Key) -> bool {
        false
    }
}
struct Parser {

}
impl Parser {

}
#[test]
fn test_vi_command_mode() {

}
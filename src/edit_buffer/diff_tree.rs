use crate::Key;
use crate::read_buffer::BufElem;
use super::indent;
use std::collections::HashMap;

pub enum ChildComponent {
    Fixed(Vec<BufElem>),
    Dynamic(Vec<BufElem>, usize), // placeholder, order
}

struct Node {
    is_placeholder: bool,
    buffer: Vec<BufElem>,
    children: Vec<NodeId>,
}
impl Node {
    fn new(placeholder: Vec<BufElem>) -> Self {
        Self {
            buffer: placeholder.clone(),
            is_placeholder: true,
            children: vec![],
        }
    }
    fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
    fn add_children(&mut self, children: Vec<NodeId>) {
        self.children = children
    }
}

type NodeId = usize;

pub struct DiffTree {
    pre_buffer: Vec<BufElem>,
    stack: Vec<NodeId>,
    nodes: HashMap<NodeId, Node>,
    next_node_id: NodeId,
}

impl DiffTree {
    pub fn new(pre_buffer: Vec<BufElem>) -> Self {
        let root = Node::new(vec![]);
        let mut nodes = HashMap::new();
        nodes.insert(0, root);
        Self {
            pre_buffer,
            next_node_id: 1,
            stack: vec![0],
            nodes,
        }
    }
    fn node(&self, i: NodeId) -> &Node {
        self.nodes.get(&i).unwrap()
    }
    fn cur_node_id(&self) -> NodeId {
        self.stack.last().cloned().unwrap()
    }
    fn cur_node(&mut self) -> &mut Node {
        let cur_id = self.stack.last().unwrap();
        self.nodes.get_mut(cur_id).unwrap()
    }
    fn next_node_id(&mut self) -> NodeId {
        let x = self.next_node_id;
        self.next_node_id += 1;
        x
    }

    // TODO rollback the buffer when starting a snippet
    // [a,b,c, ,f,o] -> [a,b,c, ,] + snippet

    pub fn add_children(&mut self, children: Vec<ChildComponent>) {
        let mut dynamics = vec![];
        let mut children_ids = vec![];
        for cc in children.into_iter() {
            let node_id = self.next_node_id();
            let placeholder = match cc {
                ChildComponent::Fixed(placeholder) => {
                    placeholder
                },
                ChildComponent::Dynamic(placeholder, order) => {
                    dynamics.push((order, node_id));
                    placeholder
                }
            };
            let n = Node::new(placeholder);
            self.nodes.insert(node_id, n);
            children_ids.push(node_id);
        }

        self.cur_node().add_children(children_ids);
        
        // assert!(!dynamics.is_empty());
        if !dynamics.is_empty() {
            self.stack.pop();
        }
        dynamics.sort_by_key(|pair| pair.0);
        for pair in dynamics.iter().rev() {
            self.stack.push(pair.1)
        }
    }
    // fn right_most_node_id(&self) -> NodeId {
    //     let mut cur = 0;
    //     while !self.node(cur).is_leaf() {
    //         let cur_node = self.node(cur);
    //         cur = *cur_node.children.last().unwrap();
    //     }
    //     cur
    // }
    pub fn flatten(&self) -> (Vec<BufElem>, usize) {
        self._flatten(self.cur_node_id())
    }
    fn _flatten(&self, cursor_pin: NodeId) -> (Vec<BufElem>, usize) {
        let mut buf = vec![];
        let mut cursor = 0;
        let mut stack = vec![0];
        while !stack.is_empty() {
            let cur_id = stack.pop().unwrap();
            let cur_node = self.node(cur_id);
            let is_placeholder = cur_node.is_placeholder;

            if cur_id == cursor_pin && is_placeholder {
                cursor = buf.len()
            }
            buf.append(&mut cur_node.buffer.clone());
            if cur_id == cursor_pin && !is_placeholder {
                cursor = buf.len()
            }
            
            for &child in cur_node.children.iter().rev() {
                stack.push(child);
            }
        }
        (buf, cursor)
    }
    fn before_change_buffer(&mut self) {
        if self.cur_node().is_placeholder {
            self.cur_node().buffer.clear();
            self.cur_node().is_placeholder = false;
        }
    }
    pub fn input(&mut self, k: Key) {
        assert!(self.stack.len() > 0);
        match k {
            Key::Char('\t') => {
                if self.stack.len() == 1 {
                    self.before_change_buffer();
                    self.cur_node().buffer.push(BufElem::Char('\t'))
                } else {
                    // go to the next tab stop
                    self.stack.pop();
                }
            },
            Key::Backspace => {
                self.before_change_buffer();
                self.cur_node().buffer.pop();
            },
            Key::Char('\n') => {
                self.before_change_buffer();

                // find the first eol from the current position backward
                let mut v1 = self.pre_buffer.clone();
                let mut v2 = {
                    let res = self.flatten();
                    let mut v = res.0;
                    v.split_off(res.1);
                    v
                };
                v1.append(&mut v2);
                let start_of_cur_line = if v1.is_empty() {
                    0
                } else {
                    let mut i = v1.len();
                    while v1[i-1] != BufElem::Eol {
                        i -= 1;
                        if i == 0 {
                            break;
                        }
                    }
                    i
                };
                let auto_indent = indent::AutoIndent {
                    line_predecessors: &v1[start_of_cur_line..v1.len()],
                };

                let mut v = vec![BufElem::Eol];
                v.append(&mut auto_indent.next_indent());
                self.cur_node().buffer.append(&mut v);
            },
            Key::Char(c) => {
                self.before_change_buffer();
                self.cur_node().buffer.push(BufElem::Char(c))
            },
            _ => {}
        }
    }
}

#[test]
fn test_only_root() {
    use crate::read_buffer::BufElem::*;
    let mut dt = DiffTree::new(vec![Char('a'),Eol,Char('a')]);
    assert_eq!(dt.flatten(), (vec![], 0));
    dt.input(Key::Backspace);
    assert_eq!(dt.flatten(), (vec![], 0));
    dt.input(Key::Char('a'));
    assert_eq!(dt.flatten(), (vec![Char('a')], 1));
    dt.input(Key::Backspace);
    assert_eq!(dt.flatten(), (vec![], 0));
    
    // FIXME this test is strongly bound to rust indent
    dt.input(Key::Char('{'));
    dt.input(Key::Char('\n'));
    assert_eq!(dt.flatten(), (vec![Char('{'),Eol,Char(' '),Char(' '),Char(' '),Char(' ')], 6));
}

#[test]
fn test_simple() {
    use crate::read_buffer::BufElem::*;
    let mut dt = DiffTree::new(vec![]);
    dt.add_children(vec![
        ChildComponent::Fixed(vec![BufElem::Char('a')]),
        ChildComponent::Dynamic(vec![BufElem::Char('b')],0)
    ]);
    assert_eq!(dt.flatten().0, vec![BufElem::Char('a'),BufElem::Char('b')]);
}
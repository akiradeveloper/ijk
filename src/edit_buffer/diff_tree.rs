use crate::Key;
use crate::read_buffer::BufElem;
use super::indent;
use std::collections::HashMap;

enum ChildComponent {
    Fixed(Vec<BufElem>),
    Dynamic(Vec<BufElem>, usize), // placeholder, order
}

struct Node {
    placeholder: Vec<BufElem>,
    buffer: Vec<BufElem>,
    children: Vec<NodeId>,
}
impl Node {
    fn new(placeholder: Vec<BufElem>) -> Self {
        Self {
            buffer: placeholder.clone(),
            placeholder,
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

struct DiffTree {
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
    fn cur_node(&mut self) -> &mut Node {
        let cur_id = self.stack.last().unwrap();
        self.nodes.get_mut(cur_id).unwrap()
    }
    fn next_node_id(&mut self) -> NodeId {
        let x = self.next_node_id; self.next_node_id += 1;
        x
    }
    fn add_children(&mut self, children: Vec<ChildComponent>) {
        let mut mutables = vec![];
        let mut children_ids = vec![];
        for (i, cc) in children.into_iter().enumerate() {
            let placeholder = match cc {
                ChildComponent::Fixed(placeholder) => {
                    placeholder
                },
                ChildComponent::Dynamic(placeholder, order) => {
                    mutables.push((order, i));
                    placeholder
                }
            };
            let n = Node::new(placeholder);
            let next_id = self.next_node_id();
            self.nodes.insert(next_id, n);
            children_ids.push(next_id);
        }
        
        if !mutables.is_empty() {
            self.stack.pop();
        }
        mutables.sort_by_key(|pair| pair.0);
        for pair in mutables.iter().rev() {
            self.stack.push(pair.1)
        }
        self.cur_node().add_children(children_ids);
    }
    fn right_most_node_id(&self) -> NodeId {
        let mut cur = 0;
        while !self.node(cur).is_leaf() {
            let cur_node = self.node(cur);
            cur = *cur_node.children.last().unwrap();
        }
        cur
    }
    fn flatten(&self, to: NodeId) -> (Vec<BufElem>, usize) {
        // tmp
        let node = self.nodes.get(&to).unwrap();
        (node.buffer.clone(), node.buffer.len())
    }
    pub fn input(&mut self, k: Key) {
        assert!(self.stack.len() > 0);
        match k {
            Key::Char('\t') => {
                if self.stack.len() == 1 {
                    self.cur_node().buffer.push(BufElem::Char('\t'))
                } else {
                    self.stack.pop();
                }
            },
            Key::Backspace => {
                self.cur_node().buffer.pop();
            },
            Key::Char('\n') => {
                let mut v1 = vec![];
                v1.append(&mut self.pre_buffer.clone());
                v1.append(&mut self.flatten(self.stack.last().cloned().unwrap()).0);
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
                self.cur_node().buffer.push(BufElem::Char(c))
            },
            _ => {}
        }
    }
}
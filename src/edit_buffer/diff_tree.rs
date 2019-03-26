use crate::Key;
use crate::read_buffer::BufElem;
use std::collections::HashMap;

enum SnippetComponent {
    Fixed(Vec<BufElem>),
    Dynamic(Vec<BufElem>, usize), // placeholder, order
}

struct Node {
    placeholder: Vec<BufElem>,
    buffer: Vec<BufElem>,
}
impl Node {
    fn new(placeholder: Vec<BufElem>) -> Self {
        Self {
            buffer: placeholder.clone(),
            placeholder,
        }
    }
}

type NodeId = usize;

struct DiffTree {
    stack: Vec<NodeId>,
    nodes: HashMap<NodeId, Node>,
    next_node_id: NodeId,
}

impl DiffTree {
    pub fn new() -> Self {
        let root = Node::new(vec![]);
        let mut nodes = HashMap::new();
        nodes.insert(0, root);
        Self {
            next_node_id: 1,
            stack: vec![0],
            nodes,
        }
    }
    fn next_node_id(&mut self) -> NodeId {
        let x = self.next_node_id;
        self.next_node_id += 1;
        x
    }
    fn start_snippet(snippet: Vec<SnippetComponent>) {

    }
    fn right_most_node_id(&self) -> NodeId {
        0 // tmp
    }
    fn flatten(&self, to: NodeId) -> (Vec<BufElem>, usize) {
        // tmp
        let node = self.nodes.get(&to).unwrap();
        (node.buffer.clone(), node.buffer.len())
    }
    pub fn input(&mut self, k: Key) {

    }
}
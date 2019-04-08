use crate::Key;
use crate::read_buffer::{Line, BufElem};
use super::{indent, IndentType};
use std::collections::HashMap;

#[derive(PartialEq, Clone)]
pub enum ChildComponent {
    Eol,
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
    fn current_word(&self) -> Vec<char> {
        if self.is_placeholder {
            return vec![]
        }
        if self.buffer.is_empty() {
            return vec![]
        }

        let col = self.buffer.len() - 1;
        let line = Line::new(&self.buffer);
        let r = line.word_range(col);
        if r.is_none() {
            return vec![]
        }
        let r = r.unwrap();

        let mut v = vec![];
        for i in r {
            if let BufElem::Char(c) = self.buffer[i] {
                v.push(c)
            }
        }
        v
    }
    fn rollback_current_word(&mut self) {
        let n = self.current_word().len();
        for _ in 0 .. n {
            self.buffer.pop();
        }
    }
}

type NodeId = usize;

pub struct DiffTree {
    pre_buffer: Vec<BufElem>,
    indent_type: IndentType,
    stack: Vec<NodeId>,
    nodes: HashMap<NodeId, Node>,
    next_node_id: NodeId,
}

impl DiffTree {
    pub fn new(pre_buffer: Vec<BufElem>, indent_type: IndentType) -> Self {
        let root = Node::new(vec![]);
        let mut nodes = HashMap::new();
        nodes.insert(0, root);
        Self {
            pre_buffer,
            indent_type,
            next_node_id: 1,
            stack: vec![0],
            nodes,
        }
    }
    pub fn current_word(&self) -> Vec<char> {
         self.node(self.cur_node_id()).current_word()
    }
    pub fn rollback_current_word(&mut self) {
         self.cur_node().rollback_current_word() 
    }
    fn node(&self, i: NodeId) -> &Node {
        self.nodes.get(&i).unwrap()
    }
    fn node_mut(&mut self, i: NodeId) -> &mut Node {
        self.nodes.get_mut(&i).unwrap()
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
        let auto_indent = self.current_auto_indent();

        let mut dynamics = vec![];
        let mut children_ids = vec![];
        for cc in children.iter() {
            let node_id = self.next_node_id();
            let placeholder: Vec<BufElem> = match cc.clone() {
                ChildComponent::Eol => {
                    vec![]
                },
                ChildComponent::Fixed(placeholder) => {
                    placeholder.clone()
                },
                ChildComponent::Dynamic(placeholder, order) => {
                    dynamics.push((order, node_id));
                    placeholder.clone()
                }
            };
            let n = Node::new(placeholder);
            self.nodes.insert(node_id, n);
            children_ids.push(node_id);

            if cc == &ChildComponent::Eol {
                let mut v = vec![BufElem::Eol];
                v.append(&mut auto_indent.current_indent());
                self.node_mut(node_id).buffer = v;
            }
        }

        self.cur_node().add_children(children_ids);
        
        // any snippet should have at least a dynamic
        // if the snippet doesn't have a placeholder,
        // it should complete with a placeholder at the end.
        assert!(!dynamics.is_empty());
        self.stack.pop();
        
        dynamics.sort_by_key(|pair| pair.0);
        for pair in dynamics.iter().rev() {
            self.stack.push(pair.1)
        }
        assert!(!self.stack.is_empty());
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
    fn current_auto_indent(&self) -> indent::AutoIndent {
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
        let auto_indent = indent::AutoIndent::new(
            &v1[start_of_cur_line..v1.len()],
            self.indent_type,
        );
        auto_indent
    }
    pub fn input(&mut self, k: Key) {
        assert!(self.stack.len() > 0);
        match k {
            Key::Char('\t') => {
                if self.stack.len() == 1 {
                    self.before_change_buffer();
                    self.cur_node().buffer.push(BufElem::Char('\t'))
                } else {
                    // MEMO: Idea toward the nested placeholder
                    // to implement the VSCode's the nested placeholder
                    // the Dynamic is nested and children are added in prior but they are not pushed into the stack
                    // here, if the current node is as is the placeholder the children are added to the stack
                    // and then pop the current node as in like addChildren()
                    
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
                let auto_indent = self.current_auto_indent();

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
    let mut dt = DiffTree::new(vec![Char('a'),Eol,Char('a')], IndentType::Spaces(4));
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
    let mut dt = DiffTree::new(vec![], IndentType::Spaces(4));
    dt.add_children(vec![
        ChildComponent::Fixed(vec![BufElem::Char('a')]),
        ChildComponent::Dynamic(vec![BufElem::Char('b')],0)
    ]);
    assert_eq!(dt.flatten().0, vec![BufElem::Char('a'),BufElem::Char('b')]);
}
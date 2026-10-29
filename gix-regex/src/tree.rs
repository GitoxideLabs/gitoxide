// Derived from posix-regex 0.1.4, upstream revision
// 063882aa6051b5075062d20d00b41ddc0fb8cd89.
// Copyright (c) 2018 jD91mZM2. Licensed under the MIT license; see LICENSE-MIT.

// Keep unchanged upstream formatting and option handling while compatibility fixes are developed.
#![allow(clippy::map_unwrap_or)]

use core::{
    fmt,
    ops::{Index, IndexMut},
};

use alloc::{boxed::Box, vec::Vec};

use crate::compile::{Range, Token};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(usize);
impl From<usize> for NodeId {
    fn from(id: usize) -> Self {
        NodeId(id)
    }
}
impl From<NodeId> for usize {
    fn from(id: NodeId) -> usize {
        id.0
    }
}

#[derive(Clone)]
pub struct Node {
    pub token: Token,
    pub range: Range,
    pub parent: Option<NodeId>,
    pub next_sibling: Option<NodeId>,
    pub child: Option<NodeId>,
}
impl Node {
    pub fn children<'a>(&self, arena: &'a Tree) -> NodeIter<'a> {
        NodeIter(arena, self.child)
    }
}
impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?} {:?}", self.token, self.range)?;
        Ok(())
    }
}

pub struct NodeIter<'a>(&'a Tree, Option<NodeId>);
impl Iterator for NodeIter<'_> {
    type Item = NodeId;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.1 {
            self.1 = self.0[next].next_sibling;
            Some(next)
        } else {
            None
        }
    }
}

pub struct Checkpoint {
    cursor: Option<NodeId>,
}

#[derive(Default)]
pub struct TreeBuilder {
    arena: Vec<Node>,
    parent: Option<NodeId>,
    cursor: Option<NodeId>,
}
impl TreeBuilder {
    fn insert(&mut self, token: Token, range: Range) -> NodeId {
        let id = NodeId::from(self.arena.len());
        self.arena.push(Node {
            token,
            range,
            parent: self.parent,
            next_sibling: None,
            child: None,
        });
        if let Some(prev) = self.cursor {
            self.arena[usize::from(prev)].next_sibling = Some(id);
        }
        if let Some(parent) = self.parent {
            self.arena[usize::from(parent)].child = self.arena[usize::from(parent)].child.or(Some(id));
        }
        id
    }
    pub fn leaf(&mut self, token: Token, range: Range) {
        self.cursor = Some(self.insert(token, range));
    }
    pub fn start_internal(&mut self, token: Token, range: Range) {
        self.parent = Some(self.insert(token, range));
        self.cursor = None;
    }
    pub fn finish_internal(&mut self) {
        self.cursor = self.parent;
        self.parent = self.parent.and_then(|parent| self.arena[usize::from(parent)].parent);
    }
    pub fn checkpoint(&self) -> Checkpoint {
        Checkpoint { cursor: self.cursor }
    }
    pub fn start_internal_at(&mut self, checkpoint: Checkpoint, token: Token, range: Range) {
        let parent = if let Some(from) = checkpoint.cursor {
            let id = NodeId::from(self.arena.len());
            self.arena.push(Node {
                token,
                range,
                parent: self.parent,
                next_sibling: None,
                child: self.arena[usize::from(from)].next_sibling,
            });
            self.arena[usize::from(from)].next_sibling = Some(id);
            id
        } else if let Some(parent) = self.parent {
            let id = NodeId::from(self.arena.len());
            self.arena.push(Node {
                token,
                range,
                parent: self.parent,
                next_sibling: None,
                child: self.arena[usize::from(parent)].child,
            });
            self.arena[usize::from(parent)].child = Some(id);
            id
        } else {
            let id = NodeId::from(self.arena.len());
            self.arena.push(Node {
                token,
                range,
                parent: None,
                next_sibling: None,
                child: self.cursor,
            });
            id
        };
        // Update parent
        let mut next = self.arena[usize::from(parent)].child;
        while let Some(node) = next {
            let node = &mut self.arena[usize::from(node)];
            next = node.next_sibling;
            node.parent = Some(parent);
        }
        self.parent = Some(parent);
        self.cursor = None;
    }
    pub fn finish(self) -> Tree {
        assert!(self.cursor.is_some(), "no item");
        let cursor = self.cursor.expect("finishing a tree requires an open root");

        Tree {
            arena: self.arena.into_boxed_slice(),
            root: cursor,
        }
    }
}

#[derive(Clone)]
pub struct Tree {
    pub arena: Box<[Node]>,
    pub root: NodeId,
}
impl Index<NodeId> for Tree {
    type Output = Node;
    fn index(&self, index: NodeId) -> &Node {
        &self.arena[usize::from(index)]
    }
}
impl IndexMut<NodeId> for Tree {
    fn index_mut(&mut self, index: NodeId) -> &mut Node {
        &mut self.arena[usize::from(index)]
    }
}
impl fmt::Debug for Tree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut next = Some(self.root);
        let mut nested: usize = 0;
        'outer: while let Some(id) = next {
            let node = &self[id];
            writeln!(f, "{:indent$}{:?}", "", node, indent = nested * 2)?;
            if node.child.is_some() {
                nested += 1;
                next = node.child;
            } else {
                let mut me = Some(node);
                while me.map(|me| me.next_sibling.is_none()).unwrap_or(false) {
                    match nested.checked_sub(1) {
                        Some(new) => nested = new,
                        None => break 'outer,
                    }
                    me = me
                        .expect("the preceding loop condition established a node")
                        .parent
                        .map(|id| &self[id]);
                }
                next = me.and_then(|me| me.next_sibling);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;

    use super::*;

    fn sanity_check(tree: &Tree) {
        let mut next = Some(tree.root);
        let mut parent = None;
        while let Some(id) = next {
            let node = &tree[id];
            assert_eq!(parent, node.parent);

            if let Some(child) = node.child {
                next = Some(child);
                parent = Some(id);
            } else {
                let mut node = Some(id);
                while node.map(|node| tree[node].next_sibling.is_none()).unwrap_or(false) {
                    node = tree[node.expect("the preceding loop condition established a node")].parent;
                }
                next = node.and_then(|node| tree[node].next_sibling);
                parent = node.and_then(|node| tree[node].parent);
            }
        }
    }

    #[test]
    fn simple_builder() {
        let mut builder = TreeBuilder::default();
        builder.start_internal(Token::Root, Range(1, Some(1)));
        builder.start_internal(Token::Alternative, Range(1, Some(1)));
        builder.leaf(Token::Start, Range(1, Some(1)));
        builder.start_internal(Token::Group(1), Range(1, Some(1)));
        builder.start_internal(Token::Alternative, Range(1, Some(1)));
        builder.leaf(Token::Any, Range(1, Some(1)));
        builder.finish_internal();
        builder.finish_internal();
        builder.finish_internal();
        builder.start_internal(Token::Alternative, Range(1, Some(1)));
        builder.leaf(Token::End, Range(1, Some(1)));
        builder.finish_internal();
        builder.finish_internal();

        let tree = builder.finish();
        sanity_check(&tree);

        assert_eq!(
            format!("{tree:?}"),
            "\
Root 1..1
  Alternative 1..1
    ^ 1..1
    Group(1) 1..1
      Alternative 1..1
        . 1..1
  Alternative 1..1
    $ 1..1
"
        );
    }
    #[test]
    fn builder_checkpoint() {
        let mut builder = TreeBuilder::default();
        builder.start_internal(Token::Root, Range(1, Some(1)));
        let mut alternation = builder.checkpoint();
        builder.leaf(Token::Start, Range(1, Some(1)));
        let group = builder.checkpoint();
        builder.start_internal(Token::Alternative, Range(1, Some(1)));
        builder.leaf(Token::Any, Range(1, Some(1)));
        builder.finish_internal();
        builder.start_internal_at(group, Token::Group(1), Range(1, Some(1)));
        builder.finish_internal();
        builder.start_internal_at(alternation, Token::Alternative, Range(1, Some(1)));
        builder.finish_internal();
        alternation = builder.checkpoint();
        builder.leaf(Token::End, Range(1, Some(1)));
        builder.start_internal_at(alternation, Token::Alternative, Range(1, Some(1)));
        builder.finish_internal();
        builder.finish_internal();

        let tree = builder.finish();
        sanity_check(&tree);

        assert_eq!(
            format!("{tree:?}"),
            "\
Root 1..1
  Alternative 1..1
    ^ 1..1
    Group(1) 1..1
      Alternative 1..1
        . 1..1
  Alternative 1..1
    $ 1..1
"
        );
    }
}

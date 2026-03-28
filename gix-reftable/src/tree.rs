/// A simple binary-search tree node.
#[derive(Debug)]
pub struct TreeNode<T> {
    key: T,
    left: Option<Box<TreeNode<T>>>,
    right: Option<Box<TreeNode<T>>>,
}

impl<T> TreeNode<T> {
    fn new(key: T) -> Self {
        Self {
            key,
            left: None,
            right: None,
        }
    }
}

/// Insert `key` into the tree rooted at `root` and return a mutable reference to the matched node.
pub fn tree_insert<'a, T, F>(root: &'a mut Option<Box<TreeNode<T>>>, key: T, compare: &F) -> &'a mut TreeNode<T>
where
    F: Fn(&T, &T) -> i32,
{
    match root {
        Some(node) => {
            let cmp = compare(&key, &node.key);
            match cmp.cmp(&0) {
                std::cmp::Ordering::Less => tree_insert(&mut node.left, key, compare),
                std::cmp::Ordering::Greater => tree_insert(&mut node.right, key, compare),
                std::cmp::Ordering::Equal => node,
            }
        }
        None => {
            *root = Some(Box::new(TreeNode::new(key)));
            root.as_deref_mut().expect("inserted")
        }
    }
}

/// Search `key` in the tree rooted at `root`.
pub fn tree_search<'a, T, F>(root: &'a Option<Box<TreeNode<T>>>, key: &T, compare: &F) -> Option<&'a TreeNode<T>>
where
    F: Fn(&T, &T) -> i32,
{
    let node = root.as_deref()?;
    let cmp = compare(key, &node.key);
    match cmp.cmp(&0) {
        std::cmp::Ordering::Less => tree_search(&node.left, key, compare),
        std::cmp::Ordering::Greater => tree_search(&node.right, key, compare),
        std::cmp::Ordering::Equal => Some(node),
    }
}

/// In-order walk of all keys.
pub fn infix_walk<T, F>(root: &Option<Box<TreeNode<T>>>, action: &mut F)
where
    F: FnMut(&T),
{
    let Some(node) = root.as_deref() else {
        return;
    };
    infix_walk(&node.left, action);
    action(&node.key);
    infix_walk(&node.right, action);
}

/// Release the tree.
pub fn tree_free<T>(root: &mut Option<Box<TreeNode<T>>>) {
    *root = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmp(a: &i32, b: &i32) -> i32 {
        a.cmp(b) as i32
    }

    #[test]
    fn insert_search_walk() {
        let mut root = None;
        tree_insert(&mut root, 3, &cmp);
        tree_insert(&mut root, 1, &cmp);
        tree_insert(&mut root, 2, &cmp);

        assert!(tree_search(&root, &1, &cmp).is_some());
        assert!(tree_search(&root, &4, &cmp).is_none());

        let mut out = Vec::new();
        infix_walk(&root, &mut |k| out.push(*k));
        assert_eq!(out, vec![1, 2, 3]);

        tree_free(&mut root);
        assert!(root.is_none());
    }
}

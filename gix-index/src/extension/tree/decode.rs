use gix_hash::ObjectId;

use crate::{extension::Tree, util::split_at_byte_exclusive};

/// A recursive data structure
pub fn decode(data: &[u8], object_hash: gix_hash::Kind, alloc_limit_bytes: Option<usize>) -> Option<Tree> {
    let (tree, data) = one_recursive(data, object_hash.len_in_bytes(), alloc_limit_bytes)?;
    data.is_empty().then_some(tree)
}

fn one_recursive(data: &[u8], hash_len: usize, alloc_limit_bytes: Option<usize>) -> Option<(Tree, &[u8])> {
    let (path, data) = split_at_byte_exclusive(data, 0)?;

    let (entry_count, data) = split_at_byte_exclusive(data, b' ')?;
    let num_entries: i32 = gix_utils::btoi::to_signed(entry_count).ok()?;

    let (subtree_count, data) = split_at_byte_exclusive(data, b'\n')?;
    let subtree_count: usize = gix_utils::btoi::to_unsigned(subtree_count).ok()?;
    if alloc_limit_bytes.is_some_and(|limit| subtree_count.saturating_mul(std::mem::size_of::<Tree>()) > limit) {
        return None;
    }

    let (id, mut data) = if num_entries >= 0 {
        let (hash, data) = data.split_at_checked(hash_len)?;
        (ObjectId::from_bytes_or_panic(hash), data)
    } else {
        (
            ObjectId::null(gix_hash::Kind::from_hex_len(hash_len * 2).expect("valid hex_len")),
            data,
        )
    };

    let mut subtrees = Vec::new();
    subtrees.try_reserve(subtree_count).ok()?;
    for _ in 0..subtree_count {
        let (tree, rest) = one_recursive(data, hash_len, alloc_limit_bytes)?;
        subtrees.push(tree);
        data = rest;
    }

    subtrees.sort_by(|a, b| a.name.cmp(&b.name));
    let num_trees = subtrees.len();
    subtrees.dedup_by(|a, b| a.name == b.name);
    if num_trees != subtrees.len() {
        return None;
    }

    Some((
        Tree {
            id,
            num_entries: num_entries.try_into().ok(),
            name: path.into(),
            children: subtrees,
        },
        data,
    ))
}

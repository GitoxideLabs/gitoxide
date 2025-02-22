use std::{borrow::BorrowMut, collections::VecDeque};

use gix_object::{tree::EntryRef, FindExt, TreeRefIter};

use crate::tree::visit::{ChangeId, Relation};
use crate::tree::{visit::Change, Error, State, TreeInfoTuple, Visit};

/// Calculate the changes that would need to be applied to `lhs` to get `rhs` using `objects` to obtain objects as needed for traversal.
/// `state` can be used between multiple calls to re-use memory.
///
/// * The `state` maybe owned or mutably borrowed to allow reuses allocated data structures through multiple runs.
/// * `delegate` will receive the computed changes, see the [`Visit`] trait for more information on what to expect.
///
/// # Notes
///
/// * `lhs` can be an empty tree to simulate what would happen if the left-hand side didn't exist.
/// * To obtain progress, implement it within the `delegate`.
/// * Tree entries are expected to be ordered using [`tree-entry-comparison`][git_cmp_c] (the same [in Rust][git_cmp_rs])
/// * it does a breadth first iteration as buffer space only fits two trees, the current one on the one we compare with.
/// * does not do rename tracking but attempts to reduce allocations to zero (so performance is mostly determined
///   by the delegate implementation which should be as specific as possible. Rename tracking can be computed on top of the changes
///   received by the `delegate`.
/// * cycle checking is not performed, but can be performed in the delegate which can return
///   [`tree::visit::Action::Cancel`](crate::tree::visit::Action::Cancel) to stop the traversal.
///
/// [git_cmp_c]: https://github.com/git/git/blob/ef8ce8f3d4344fd3af049c17eeba5cd20d98b69f/tree-diff.c#L72-L88
/// [git_cmp_rs]: https://github.com/GitoxideLabs/gitoxide/blob/795962b107d86f58b1f7c75006da256d19cc80ad/gix-object/src/tree/mod.rs#L263-L273
#[doc(alias = "diff_tree_to_tree", alias = "git2")]
pub fn diff<StateMut>(
    lhs: TreeRefIter<'_>,
    rhs: TreeRefIter<'_>,
    mut state: StateMut,
    objects: impl gix_object::Find,
    delegate: &mut impl Visit,
) -> Result<(), Error>
where
    StateMut: BorrowMut<State>,
{
    let state = state.borrow_mut();
    state.clear();
    let mut lhs_entries = peekable(lhs);
    let mut rhs_entries = peekable(rhs);
    let mut relation = None;
    let mut pop_path = false;

    loop {
        if pop_path {
            delegate.pop_path_component();
        }
        pop_path = true;

        match (lhs_entries.next(), rhs_entries.next()) {
            (None, None) => {
                match state.trees.pop_front() {
                    Some((None, Some(rhs), relation_to_propagate)) => {
                        delegate.pop_front_tracked_path_and_set_current();
                        relation = relation_to_propagate;
                        rhs_entries = peekable(objects.find_tree_iter(&rhs, &mut state.buf2)?);
                    }
                    Some((Some(lhs), Some(rhs), relation_to_propagate)) => {
                        delegate.pop_front_tracked_path_and_set_current();
                        lhs_entries = peekable(objects.find_tree_iter(&lhs, &mut state.buf1)?);
                        rhs_entries = peekable(objects.find_tree_iter(&rhs, &mut state.buf2)?);
                        relation = relation_to_propagate;
                    }
                    Some((Some(lhs), None, relation_to_propagate)) => {
                        delegate.pop_front_tracked_path_and_set_current();
                        lhs_entries = peekable(objects.find_tree_iter(&lhs, &mut state.buf1)?);
                        relation = relation_to_propagate;
                    }
                    Some((None, None, _)) => unreachable!("BUG: it makes no sense to fill the stack with empties"),
                    None => return Ok(()),
                }
                pop_path = false;
            }
            (Some(lhs), Some(rhs)) => {
                use std::cmp::Ordering::*;
                let (lhs, rhs) = (lhs?, rhs?);
                match compare(&lhs, &rhs) {
                    Equal => handle_lhs_and_rhs_with_equal_filenames(
                        lhs,
                        rhs,
                        &mut state.trees,
                        &mut state.change_id,
                        relation,
                        delegate,
                    )?,
                    Less => catchup_lhs_with_rhs(
                        &mut lhs_entries,
                        lhs,
                        rhs,
                        &mut state.trees,
                        &mut state.change_id,
                        relation,
                        delegate,
                    )?,
                    Greater => catchup_rhs_with_lhs(
                        &mut rhs_entries,
                        lhs,
                        rhs,
                        &mut state.trees,
                        &mut state.change_id,
                        relation,
                        delegate,
                    )?,
                }
            }
            (Some(lhs), None) => {
                let lhs = lhs?;
                delete_entry_schedule_recursion(lhs, &mut state.trees, &mut state.change_id, relation, delegate)?;
            }
            (None, Some(rhs)) => {
                let rhs = rhs?;
                add_entry_schedule_recursion(rhs, &mut state.trees, &mut state.change_id, relation, delegate)?;
            }
        }
    }
}

fn compare(a: &EntryRef<'_>, b: &EntryRef<'_>) -> std::cmp::Ordering {
    let common = a.filename.len().min(b.filename.len());
    a.filename[..common].cmp(&b.filename[..common]).then_with(|| {
        let a = a.filename.get(common).or_else(|| a.mode.is_tree().then_some(&b'/'));
        let b = b.filename.get(common).or_else(|| b.mode.is_tree().then_some(&b'/'));
        a.cmp(&b)
    })
}

fn delete_entry_schedule_recursion(
    entry: EntryRef<'_>,
    queue: &mut VecDeque<TreeInfoTuple>,
    change_id: &mut ChangeId,
    relation_to_propagate: Option<Relation>,
    delegate: &mut impl Visit,
) -> Result<(), Error> {
    delegate.push_path_component(entry.filename);
    let relation = relation_to_propagate.or_else(|| {
        entry.mode.is_tree().then(|| {
            *change_id += 1;
            Relation::Parent(*change_id)
        })
    });
    let is_cancelled = delegate
        .visit(Change::Deletion {
            entry_mode: entry.mode,
            oid: entry.oid.to_owned(),
            relation,
        })
        .cancelled();
    if is_cancelled {
        return Err(Error::Cancelled);
    }
    if entry.mode.is_tree() {
        delegate.pop_path_component();
        delegate.push_back_tracked_path_component(entry.filename);
        queue.push_back((Some(entry.oid.to_owned()), None, to_child(relation)));
    }
    Ok(())
}

fn add_entry_schedule_recursion(
    entry: EntryRef<'_>,
    queue: &mut VecDeque<TreeInfoTuple>,
    change_id: &mut ChangeId,
    relation_to_propagate: Option<Relation>,
    delegate: &mut impl Visit,
) -> Result<(), Error> {
    delegate.push_path_component(entry.filename);
    let relation = relation_to_propagate.or_else(|| {
        entry.mode.is_tree().then(|| {
            *change_id += 1;
            Relation::Parent(*change_id)
        })
    });
    if delegate
        .visit(Change::Addition {
            entry_mode: entry.mode,
            oid: entry.oid.to_owned(),
            relation,
        })
        .cancelled()
    {
        return Err(Error::Cancelled);
    }
    if entry.mode.is_tree() {
        delegate.pop_path_component();
        delegate.push_back_tracked_path_component(entry.filename);
        queue.push_back((None, Some(entry.oid.to_owned()), to_child(relation)));
    }
    Ok(())
}

fn catchup_rhs_with_lhs(
    rhs_entries: &mut IteratorType<TreeRefIter<'_>>,
    lhs: EntryRef<'_>,
    rhs: EntryRef<'_>,
    queue: &mut VecDeque<TreeInfoTuple>,
    change_id: &mut ChangeId,
    relation_to_propagate: Option<Relation>,
    delegate: &mut impl Visit,
) -> Result<(), Error> {
    use std::cmp::Ordering::*;
    add_entry_schedule_recursion(rhs, queue, change_id, relation_to_propagate, delegate)?;
    loop {
        match rhs_entries.peek() {
            Some(Ok(rhs)) => match compare(&lhs, rhs) {
                Equal => {
                    let rhs = rhs_entries.next().transpose()?.expect("the peeked item to be present");
                    delegate.pop_path_component();
                    handle_lhs_and_rhs_with_equal_filenames(
                        lhs,
                        rhs,
                        queue,
                        change_id,
                        relation_to_propagate,
                        delegate,
                    )?;
                    break;
                }
                Greater => {
                    let rhs = rhs_entries.next().transpose()?.expect("the peeked item to be present");
                    delegate.pop_path_component();
                    add_entry_schedule_recursion(rhs, queue, change_id, relation_to_propagate, delegate)?;
                }
                Less => {
                    delegate.pop_path_component();
                    delete_entry_schedule_recursion(lhs, queue, change_id, relation_to_propagate, delegate)?;
                    break;
                }
            },
            Some(Err(err)) => return Err(Error::EntriesDecode(err.to_owned())),
            None => {
                delegate.pop_path_component();
                delete_entry_schedule_recursion(lhs, queue, change_id, relation_to_propagate, delegate)?;
                break;
            }
        }
    }
    Ok(())
}

fn catchup_lhs_with_rhs(
    lhs_entries: &mut IteratorType<TreeRefIter<'_>>,
    lhs: EntryRef<'_>,
    rhs: EntryRef<'_>,
    queue: &mut VecDeque<TreeInfoTuple>,
    change_id: &mut ChangeId,
    relation_to_propagate: Option<Relation>,
    delegate: &mut impl Visit,
) -> Result<(), Error> {
    use std::cmp::Ordering::*;
    delete_entry_schedule_recursion(lhs, queue, change_id, relation_to_propagate, delegate)?;
    loop {
        match lhs_entries.peek() {
            Some(Ok(lhs)) => match compare(lhs, &rhs) {
                Equal => {
                    let lhs = lhs_entries.next().expect("the peeked item to be present")?;
                    delegate.pop_path_component();
                    handle_lhs_and_rhs_with_equal_filenames(
                        lhs,
                        rhs,
                        queue,
                        change_id,
                        relation_to_propagate,
                        delegate,
                    )?;
                    break;
                }
                Less => {
                    let lhs = lhs_entries.next().expect("the peeked item to be present")?;
                    delegate.pop_path_component();
                    delete_entry_schedule_recursion(lhs, queue, change_id, relation_to_propagate, delegate)?;
                }
                Greater => {
                    delegate.pop_path_component();
                    add_entry_schedule_recursion(rhs, queue, change_id, relation_to_propagate, delegate)?;
                    break;
                }
            },
            Some(Err(err)) => return Err(Error::EntriesDecode(err.to_owned())),
            None => {
                delegate.pop_path_component();
                add_entry_schedule_recursion(rhs, queue, change_id, relation_to_propagate, delegate)?;
                break;
            }
        }
    }
    Ok(())
}

fn handle_lhs_and_rhs_with_equal_filenames(
    lhs: EntryRef<'_>,
    rhs: EntryRef<'_>,
    queue: &mut VecDeque<TreeInfoTuple>,
    change_id: &mut ChangeId,
    relation_to_propagate: Option<Relation>,
    delegate: &mut impl Visit,
) -> Result<(), Error> {
    match (lhs.mode.is_tree(), rhs.mode.is_tree()) {
        (true, true) => {
            delegate.push_back_tracked_path_component(lhs.filename);
            if lhs.oid != rhs.oid
                && delegate
                    .visit(Change::Modification {
                        previous_entry_mode: lhs.mode,
                        previous_oid: lhs.oid.to_owned(),
                        entry_mode: rhs.mode,
                        oid: rhs.oid.to_owned(),
                    })
                    .cancelled()
            {
                return Err(Error::Cancelled);
            }
            queue.push_back((
                Some(lhs.oid.to_owned()),
                Some(rhs.oid.to_owned()),
                relation_to_propagate,
            ));
        }
        (_, true) => {
            delegate.push_back_tracked_path_component(lhs.filename);
            if delegate
                .visit(Change::Deletion {
                    entry_mode: lhs.mode,
                    oid: lhs.oid.to_owned(),
                    relation: None,
                })
                .cancelled()
            {
                return Err(Error::Cancelled);
            }

            let relation = relation_to_propagate.or_else(|| {
                *change_id += 1;
                Some(Relation::Parent(*change_id))
            });
            if delegate
                .visit(Change::Addition {
                    entry_mode: rhs.mode,
                    oid: rhs.oid.to_owned(),
                    relation,
                })
                .cancelled()
            {
                return Err(Error::Cancelled);
            }
            queue.push_back((None, Some(rhs.oid.to_owned()), to_child(relation)));
        }
        (true, _) => {
            delegate.push_back_tracked_path_component(lhs.filename);
            let relation = relation_to_propagate.or_else(|| {
                *change_id += 1;
                Some(Relation::Parent(*change_id))
            });
            if delegate
                .visit(Change::Deletion {
                    entry_mode: lhs.mode,
                    oid: lhs.oid.to_owned(),
                    relation,
                })
                .cancelled()
            {
                return Err(Error::Cancelled);
            }
            if delegate
                .visit(Change::Addition {
                    entry_mode: rhs.mode,
                    oid: rhs.oid.to_owned(),
                    relation: None,
                })
                .cancelled()
            {
                return Err(Error::Cancelled);
            }
            queue.push_back((Some(lhs.oid.to_owned()), None, to_child(relation)));
        }
        (false, false) => {
            delegate.push_path_component(lhs.filename);
            debug_assert!(lhs.mode.is_no_tree() && lhs.mode.is_no_tree());
            if (lhs.oid != rhs.oid || lhs.mode != rhs.mode)
                && delegate
                    .visit(Change::Modification {
                        previous_entry_mode: lhs.mode,
                        previous_oid: lhs.oid.to_owned(),
                        entry_mode: rhs.mode,
                        oid: rhs.oid.to_owned(),
                    })
                    .cancelled()
            {
                return Err(Error::Cancelled);
            }
        }
    }
    Ok(())
}

type IteratorType<I> = std::iter::Peekable<I>;

fn to_child(r: Option<Relation>) -> Option<Relation> {
    r.map(|r| match r {
        Relation::Parent(id) => Relation::ChildOfParent(id),
        Relation::ChildOfParent(id) => Relation::ChildOfParent(id),
    })
}

fn peekable<I: Iterator>(iter: I) -> IteratorType<I> {
    iter.peekable()
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use gix_object::tree::EntryKind;

    use super::*;

    #[test]
    fn compare_select_samples() {
        let null = gix_hash::ObjectId::null(gix_hash::Kind::Sha1);
        let actual = compare(
            &EntryRef {
                mode: EntryKind::Blob.into(),
                filename: "plumbing-cli.rs".into(),
                oid: &null,
            },
            &EntryRef {
                mode: EntryKind::Tree.into(),
                filename: "plumbing".into(),
                oid: &null,
            },
        );
        assert_eq!(actual, Ordering::Less);
        let actual = compare(
            &EntryRef {
                mode: EntryKind::Tree.into(),
                filename: "plumbing-cli.rs".into(),
                oid: &null,
            },
            &EntryRef {
                mode: EntryKind::Blob.into(),
                filename: "plumbing".into(),
                oid: &null,
            },
        );
        assert_eq!(actual, Ordering::Greater);
    }
}

use bstr::BStr;
use gix_error::{Result, ResultExt, message};
use gix_hash::{ObjectId, oid};

/// A fact needed to resolve a nontrivial three-way gitlink merge.
#[derive(Debug, Clone, Copy)]
pub enum Query<'a> {
    /// Are the ancestor, current, and other IDs all available as commits in this submodule?
    /// This is queried before walking history, so a missing tip remains a conflict even if another walk would fail.
    CommitsAvailable([&'a oid; 3]),
    /// Is `ancestor` reachable from `descendant`, including when they are equal?
    IsAncestor {
        /// The commit whose reachability is being checked.
        ancestor: &'a oid,
        /// The commit at which the history walk starts.
        descendant: &'a oid,
    },
}

/// Answer a history query for the submodule at the original ancestor/source path.
///
/// Return `None` if its repository or the requested fact is unavailable, and propagate errors encountered
/// while reading available history. Repository discovery, shallow boundaries, replacements, and graph caches
/// belong to the caller. IDs belong to that submodule's object database, not the superproject's.
pub type QueryFn<'a> = dyn FnMut(&BStr, Query<'_>) -> Result<Option<bool>> + 'a;

pub(crate) fn merge(
    path: &BStr,
    base_commit_id: &oid,
    current_commit_id: &oid,
    other_commit_id: &oid,
    context: &mut crate::tree::Context<'_>,
) -> Result<Option<ObjectId>> {
    let Some(query) = context.submodule.as_mut() else {
        return Ok(None);
    };
    let mut query =
        |request| query(path, request).or_raise(|| message!("Could not inspect history of submodule {path}"));
    if query(Query::CommitsAvailable([
        base_commit_id,
        current_commit_id,
        other_commit_id,
    ]))? != Some(true)
    {
        return Ok(None);
    }
    for descendant in [current_commit_id, other_commit_id] {
        if query(Query::IsAncestor {
            ancestor: base_commit_id,
            descendant,
        })? != Some(true)
        {
            return Ok(None);
        }
    }
    for (ancestor, descendant) in [
        (current_commit_id, other_commit_id),
        (other_commit_id, current_commit_id),
    ] {
        match query(Query::IsAncestor { ancestor, descendant })? {
            Some(true) => return Ok(Some(descendant.to_owned())),
            Some(false) => {}
            None => return Ok(None),
        }
    }
    Ok(None)
}

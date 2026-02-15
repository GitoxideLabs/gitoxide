use std::borrow::Cow;

use gix_object::FindExt;

use crate::{
    blob::builtin_driver,
    commit::{Error, Options},
};

/// Like [`tree()`](crate::tree()), but it takes only two commits, `our_commit` and `their_commit` to automatically
/// compute the merge-bases among them.
/// If there are multiple merge bases, these will be auto-merged into one, recursively, if
/// [`allow_missing_merge_base`](Options::allow_missing_merge_base) is `true`.
///
/// `labels` are names where [`current`](crate::blob::builtin_driver::text::Labels::current) is a name for `our_commit`
/// and [`other`](crate::blob::builtin_driver::text::Labels::other) is a name for `their_commit`.
/// If [`ancestor`](crate::blob::builtin_driver::text::Labels::ancestor) is unset, it will be set by us based on the
/// merge-bases of `our_commit` and `their_commit`.
///
/// The `graph` is used to find the merge-base between `our_commit` and `their_commit`, and can also act as cache
/// to speed up subsequent merge-base queries.
///
/// Use `abbreviate_hash(id)` to shorten the given `id` according to standard git shortening rules. It's used in case
/// the ancestor-label isn't explicitly set so that the merge base label becomes the shortened `id`.
/// Note that it's a dyn closure only to make it possible to recursively call this function in case of multiple merge-bases.
///
/// `write_object` is used only if it's allowed to merge multiple merge-bases into one, and if there
/// are multiple merge bases, and to write merged buffers as blobs.
///
/// ### Performance
///
/// Note that `objects` *should* have an object cache to greatly accelerate tree-retrieval.
///
/// ### Notes
///
/// When merging merge-bases recursively, the options are adjusted automatically to act like Git, i.e. merge binary
/// blobs and resolve with *ours*, while resorting to using the base/ancestor in case of unresolvable conflicts.
///
/// ### Deviation
///
/// * It's known that certain conflicts around symbolic links can be auto-resolved. We don't have an option for this
///   at all, yet, primarily as Git seems to not implement the *ours*/*theirs* choice in other places even though it
///   reasonably could. So we leave it to the caller to continue processing the returned tree at will.
#[allow(clippy::too_many_arguments)]
pub fn commit<'objects>(
    our_commit: gix_hash::ObjectId,
    their_commit: gix_hash::ObjectId,
    labels: builtin_driver::text::Labels<'_>,
    graph: &mut gix_revwalk::Graph<'_, '_, gix_revwalk::graph::Commit<gix_revision::merge_base::Flags>>,
    diff_resource_cache: &mut gix_diff::blob::Platform,
    blob_merge: &mut crate::blob::Platform,
    objects: &'objects (impl gix_object::FindObjectOrHeader + gix_object::Write),
    abbreviate_hash: &mut dyn FnMut(&gix_hash::oid) -> String,
    options: Options,
) -> Result<super::Outcome<'objects>, Error> {
    let merge_bases = gix_revision::merge_base(our_commit, &[their_commit], graph)?;
    let mut virtual_merge_bases = Vec::new();
    let mut state = gix_diff::tree::State::default();
    let mut commit_to_tree =
        |commit_id: gix_hash::ObjectId| objects.find_commit(&commit_id, &mut state.buf1).map(|c| c.tree());

    let (merge_base_tree_id, ancestor_name): (_, Cow<'_, str>) = match merge_bases.clone() {
        Some(base_commit) if base_commit.as_slice().len() == 1 => (
            commit_to_tree(*base_commit.first())?,
            abbreviate_hash(base_commit.first()).into(),
        ),
        Some(base_commits) => {
            let virtual_base_tree = if options.use_first_merge_base {
                commit_to_tree(*base_commits.first())?
            } else {
                let mut base_commits = base_commits.into_vec();
                let first = base_commits.pop().expect("at least two");
                let second = base_commits.pop().expect("at least one left");
                let out = crate::commit::virtual_merge_base(
                    first,
                    second,
                    base_commits,
                    graph,
                    diff_resource_cache,
                    blob_merge,
                    objects,
                    abbreviate_hash,
                    options.tree_merge.clone(),
                )?;
                virtual_merge_bases = out.virtual_merge_bases.into_vec();
                out.tree_id
            };
            (virtual_base_tree, "merged common ancestors".into())
        }
        None => {
            if options.allow_missing_merge_base {
                (gix_hash::ObjectId::empty_tree(our_commit.kind()), "empty tree".into())
            } else {
                return Err(Error::NoMergeBase {
                    our_commit_id: our_commit,
                    their_commit_id: their_commit,
                });
            }
        }
    };

    let mut labels = labels; // TODO(borrowchk): this re-assignment shouldn't be needed.
    if labels.ancestor.is_none() {
        labels.ancestor = Some(ancestor_name.as_ref().into());
    }

    let our_tree_id = objects.find_commit(&our_commit, &mut state.buf1)?.tree();
    let their_tree_id = objects.find_commit(&their_commit, &mut state.buf1)?.tree();

    let outcome = crate::tree(
        &merge_base_tree_id,
        &our_tree_id,
        &their_tree_id,
        labels,
        objects,
        |buf| objects.write_buf(gix_object::Kind::Blob, buf),
        &mut state,
        diff_resource_cache,
        blob_merge,
        options.tree_merge,
    )?;

    Ok(super::Outcome {
        tree_merge: outcome,
        merge_bases,
        merge_base_tree_id,
        virtual_merge_bases,
    })
}

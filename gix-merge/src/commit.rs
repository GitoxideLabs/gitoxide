/// The error returned by [`commit()`](crate::commit()).
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    MergeBase(#[from] gix_revision::merge_base::Error),
    #[error(transparent)]
    MergeTree(#[from] crate::tree::Error),
    #[error("No common ancestor between {our_commit_id} and {their_commit_id}")]
    NoMergeBase {
        /// The commit on our side that was to be merged.
        our_commit_id: gix_hash::ObjectId,
        /// The commit on their side that was to be merged.
        their_commit_id: gix_hash::ObjectId,
    },
    #[error("Could not find ancestor, our or their commit to extract tree from")]
    FindCommit(#[from] gix_object::find::existing_object::Error),
}

/// A way to configure [`commit()`](crate::commit()).
#[derive(Default, Debug, Copy, Clone)]
pub struct Options {
    /// If `true`, merging unrelated commits is allowed, with the merge-base being assumed as empty tree.
    pub allow_missing_merge_base: bool,
    /// Options to define how trees should be merged.
    pub tree_merge: crate::tree::Options,
    /// Options to define how to merge blobs.
    ///
    /// Note that these are temporarily overwritten if multiple merge-bases are merged into one.
    pub blob_merge: crate::blob::platform::merge::Options,
}

pub(super) mod function {
    use crate::commit::{Error, Options};
    use gix_object::FindExt;

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
    /// ### Performance
    ///
    /// Note that `objects` *should* have an object cache to greatly accelerate tree-retrieval.
    #[allow(clippy::too_many_arguments)]
    pub fn commit<'objects>(
        our_commit: gix_hash::ObjectId,
        their_commit: gix_hash::ObjectId,
        mut labels: crate::blob::builtin_driver::text::Labels<'_>,
        graph: &mut gix_revwalk::Graph<'_, '_, gix_revwalk::graph::Commit<gix_revision::merge_base::Flags>>,
        diff_resource_cache: &mut gix_diff::blob::Platform,
        blob_merge: &mut crate::blob::Platform,
        objects: &'objects impl gix_object::FindObjectOrHeader,
        options: Options,
    ) -> Result<crate::tree::Outcome<'objects>, Error> {
        let merge_bases_commit_ids = gix_revision::merge_base(our_commit, &[their_commit], graph)?;
        let (merge_base_commit_id, ancestor_name) = match merge_bases_commit_ids {
            Some(base_commit) if base_commit.len() == 1 => (base_commit[0], None),
            Some(base_commits) => {
                let virtual_base_tree = *base_commits.first().expect("TODO: merge multiple bases into one");
                (virtual_base_tree, Some("merged common ancestors".into()))
            }
            None => {
                if options.allow_missing_merge_base {
                    (
                        gix_hash::ObjectId::empty_tree(our_commit.kind()),
                        Some("empty tree".into()),
                    )
                } else {
                    return Err(Error::NoMergeBase {
                        our_commit_id: our_commit,
                        their_commit_id: their_commit,
                    });
                }
            }
        };
        if labels.ancestor.is_none() {
            labels.ancestor = ancestor_name;
        }

        let mut state = gix_diff::tree::State::default();
        let merge_base_tree_id = objects.find_commit(&merge_base_commit_id, &mut state.buf1)?.tree();
        let our_tree_id = objects.find_commit(&our_commit, &mut state.buf1)?.tree();
        let their_tree_id = objects.find_commit(&their_commit, &mut state.buf1)?.tree();

        Ok(crate::tree(
            &merge_base_tree_id,
            &our_tree_id,
            &their_tree_id,
            labels,
            objects,
            &mut state,
            diff_resource_cache,
            blob_merge,
            options.tree_merge,
        )?)
    }
}

/// The outcome produced by [`commit::merge_base()`](crate::commit::virtual_merge_base()).
pub struct Outcome {
    /// The commit ids of all the virtual merge bases we have produced in the process of recursively merging the merge-bases.
    /// As they have been written to the object database, they are still available until they are garbage collected.
    /// The last one is the most recently produced and the one returned as `commit_id`.
    /// This is never empty.
    pub virtual_merge_bases: Vec<gix_hash::ObjectId>,
    /// The id of the commit that was created to hold the merged tree.
    pub commit_id: gix_hash::ObjectId,
    /// The hash of the merged tree.
    pub tree_id: gix_hash::ObjectId,
}

/// The error returned by [`commit::merge_base()`](crate::commit::virtual_merge_base()).
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    MergeTree(#[from] crate::tree::Error),
    #[error("Failed to write tree for merged merge-base or virtual commit")]
    WriteObject(gix_object::write::Error),
    #[error("Failed to decode a commit needed to build a virtual merge-base")]
    DecodeCommit(#[from] gix_object::decode::Error),
    #[error(
        "Conflicts occurred when trying to resolve multiple merge-bases by merging them. This is most certainly a bug."
    )]
    VirtualMergeBaseConflict,
    #[error("Could not find commit to use as basis for a virtual commit")]
    FindCommit(#[from] gix_object::find::existing_object::Error),
}

pub(super) mod function {
    use std::convert::TryFrom;

    use gix_object::FindExt;

    use super::Error;
    use crate::{
        blob::builtin_driver,
        tree::{treat_as_unresolved, TreatAsUnresolved},
    };

    /// Create a single virtual merge-base by merging `first_commit`, `second_commit` and `others` into one.
    /// Note that `first_commit` and `second_commit` are expected to have been popped off `others`, so `first_commit`
    /// was the last provided merge-base of function that provides multiple merge-bases for a pair of commits.
    ///
    /// The parameters `graph`, `diff_resource_cache`, `blob_merge`, `objects`, `abbreviate_hash` and `options` are passed
    /// directly to [`tree()`](crate::tree()) for merging the trees of two merge-bases at a time.
    /// Note that most of `options` are overwritten to match the requirements of a merge-base merge.
    #[allow(clippy::too_many_arguments)]
    pub fn virtual_merge_base<'objects>(
        first_commit: gix_hash::ObjectId,
        second_commit: gix_hash::ObjectId,
        mut others: Vec<gix_hash::ObjectId>,
        graph: &mut gix_revwalk::Graph<'_, '_, gix_revwalk::graph::Commit<gix_revision::merge_base::Flags>>,
        diff_resource_cache: &mut gix_diff::blob::Platform,
        blob_merge: &mut crate::blob::Platform,
        objects: &'objects (impl gix_object::FindObjectOrHeader + gix_object::Write),
        abbreviate_hash: &mut dyn FnMut(&gix_hash::oid) -> String,
        mut options: crate::tree::Options,
    ) -> Result<super::Outcome, crate::commit::Error> {
        let mut merged_commit_id = first_commit;
        others.push(second_commit);

        options.tree_conflicts = Some(crate::tree::ResolveWith::Ancestor);
        options.blob_merge.is_virtual_ancestor = true;
        options.blob_merge.text.conflict = builtin_driver::text::Conflict::ResolveWithOurs;
        let favor_ancestor = Some(builtin_driver::binary::ResolveWith::Ancestor);
        options.blob_merge.resolve_binary_with = favor_ancestor;
        options.symlink_conflicts = favor_ancestor;
        let labels = builtin_driver::text::Labels {
            current: Some("Temporary merge branch 1".into()),
            other: Some("Temporary merge branch 2".into()),
            ancestor: None,
        };
        let mut virtual_merge_bases = Vec::new();
        let mut tree_id = None;
        while let Some(next_commit_id) = others.pop() {
            options.marker_size_multiplier += 1;
            let mut out = crate::commit(
                merged_commit_id,
                next_commit_id,
                labels,
                graph,
                diff_resource_cache,
                blob_merge,
                objects,
                abbreviate_hash,
                crate::commit::Options {
                    allow_missing_merge_base: false,
                    tree_merge: options.clone(),
                    use_first_merge_base: false,
                },
            )?;
            // This shouldn't happen, but if for some buggy reason it does, we rather bail.
            if out.tree_merge.has_unresolved_conflicts(TreatAsUnresolved {
                content_merge: treat_as_unresolved::ContentMerge::Markers,
                tree_merge: treat_as_unresolved::TreeMerge::Undecidable,
            }) {
                return Err(Error::VirtualMergeBaseConflict.into());
            }
            let merged_tree_id = out
                .tree_merge
                .tree
                .write(|tree| objects.write(tree))
                .map_err(Error::WriteObject)?;

            tree_id = Some(merged_tree_id);
            merged_commit_id = create_virtual_commit(objects, merged_commit_id, next_commit_id, merged_tree_id)?;

            virtual_merge_bases.extend(out.virtual_merge_bases);
            virtual_merge_bases.push(merged_commit_id);
        }

        Ok(super::Outcome {
            virtual_merge_bases,
            commit_id: merged_commit_id,
            tree_id: tree_id.map_or_else(
                || {
                    let mut buf = Vec::new();
                    objects.find_commit(&merged_commit_id, &mut buf).map(|c| c.tree())
                },
                Ok,
            )?,
        })
    }

    fn create_virtual_commit(
        objects: &(impl gix_object::Find + gix_object::Write),
        parent_a: gix_hash::ObjectId,
        parent_b: gix_hash::ObjectId,
        tree_id: gix_hash::ObjectId,
    ) -> Result<gix_hash::ObjectId, Error> {
        let mut buf = Vec::new();
        let commit_ref = objects.find_commit(&parent_a, &mut buf)?;
        let mut commit = gix_object::Commit::try_from(commit_ref)?;
        commit.parents = vec![parent_a, parent_b].into();
        commit.tree = tree_id;
        objects.write(&commit).map_err(Error::WriteObject)
    }
}

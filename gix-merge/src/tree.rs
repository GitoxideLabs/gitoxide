use gix_diff::Rewrites;

/// The error returned by [`tree()`](crate::tree()).
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Could not find ancestor, our or their tree to get started")]
    FindTree(#[from] gix_object::find::existing_object::Error),
    #[error("Could not find ancestor, our or their tree iterator to get started")]
    FindTreeIter(#[from] gix_object::find::existing_iter::Error),
    #[error("Failed to diff our side or their side")]
    DiffTree(#[from] gix_diff::tree_with_rewrites::Error),
    #[error("Could not apply merge result to base tree")]
    TreeEdit(#[from] gix_object::tree::editor::Error),
}

/// The outcome produced by [`tree()`](crate::tree()).
pub struct Outcome<'a> {
    /// The ready-made (but unwritten) tree if `conflicts` is empty, or the best-possible tree when facing `conflicts`.
    ///
    /// The tree may contain blobs with conflict markers, and will be missing directories or files that were conflicting
    /// without a resolution strategy.
    ///
    /// This means, if all of their changes were conflicting, this will be equivalent to our tree.
    pub tree: gix_object::tree::Editor<'a>,
    /// The set of conflicts we encountered. Can be empty to indicate there was no conflict.
    pub conflicts: Vec<Conflict>,
}

/// A description of a conflict (i.e. merge issue without an auto-resolution) as seen during a [tree-merge](crate::tree()).
pub struct Conflict;

/// A way to configure [`tree()`](crate::tree()).
#[derive(Default, Debug, Copy, Clone)]
pub struct Options {
    /// If *not* `None`, rename tracking will be performed when determining the changes of each side of the merge.
    pub rewrites: Option<Rewrites>,
    // TODO(Perf) add a flag to allow parallelizing the tree-diff itself.
}

pub(super) mod function {
    use crate::tree::{Error, Options, Outcome};
    use bstr::{BStr, BString};
    use gix_diff::tree::recorder::Location;
    use gix_diff::tree::visit::Relation;
    use gix_diff::tree_with_rewrites::Change;
    use gix_object::{tree, FindExt};
    use std::collections::btree_map::Entry;
    use std::collections::BTreeMap;
    use std::convert::Infallible;

    /// Perform a merge between `our_tree` and `their_tree`, using `base_tree` as merge-base.
    /// Note that `base_tree` can be an empty tree to indicate 'no common ancestor between the two sides'.
    ///
    /// `labels` are relevant for text-merges and will be shown in conflicts.
    /// `objects` provides access to trees when diffing them.
    /// `diff_state` is state used for diffing trees.
    /// `diff_resource_cache` is used for similarity checks.
    /// `blob_merge` is a pre-configured platform to merge any content.
    /// `options` are used to affect how the merge is performed.
    ///
    /// ### Performance
    ///
    /// Note that `objects` *should* have an object cache to greatly accelerate tree-retrieval.
    #[allow(clippy::too_many_arguments)]
    pub fn tree<'objects>(
        base_tree: &gix_hash::oid,
        our_tree: &gix_hash::oid,
        their_tree: &gix_hash::oid,
        _labels: crate::blob::builtin_driver::text::Labels<'_>,
        objects: &'objects impl gix_object::FindObjectOrHeader,
        diff_state: &mut gix_diff::tree::State,
        diff_resource_cache: &mut gix_diff::blob::Platform,
        _blob_merge: &mut crate::blob::Platform,
        options: Options,
    ) -> Result<Outcome<'objects>, Error> {
        let (mut base_buf, mut side_buf) = (Vec::new(), Vec::new());
        let ancestor_tree = objects.find_tree(base_tree, &mut base_buf)?;
        let our_tree = objects.find_tree_iter(our_tree, &mut side_buf)?;

        let mut editor = tree::Editor::new(ancestor_tree.to_owned(), objects, base_tree.kind());
        let ancestor_tree = gix_object::TreeRefIter::from_bytes(&base_buf);

        let mut ours = Vec::new();
        gix_diff::tree_with_rewrites(
            ancestor_tree,
            our_tree,
            diff_resource_cache,
            diff_state,
            objects,
            |change| -> Result<_, Infallible> {
                if may_track(change) {
                    ours.push(change.into_owned());
                }
                Ok(gix_diff::tree_with_rewrites::Action::Continue)
            },
            gix_diff::tree_with_rewrites::Options {
                location: Some(Location::Path),
                rewrites: options.rewrites,
            },
        )?;

        let mut rewrite_source_tree = TreeNode::default();

        for change in &ours {
            apply_change(&mut editor, change)?;
            rewrite_source_tree.track_ours_exclusive(change);
        }

        let their_tree = objects.find_tree_iter(their_tree, &mut side_buf)?;
        let mut theirs = Vec::new();
        gix_diff::tree_with_rewrites(
            ancestor_tree,
            their_tree,
            diff_resource_cache,
            diff_state,
            objects,
            |change| -> Result<_, Infallible> {
                if may_track(change) {
                    theirs.push(change.into_owned());
                }
                Ok(gix_diff::tree_with_rewrites::Action::Continue)
            },
            gix_diff::tree_with_rewrites::Options {
                location: Some(Location::Path),
                rewrites: options.rewrites,
            },
        )?;

        dbg!(&ours, &theirs, &rewrite_source_tree);

        let conflicts = Vec::new();
        for their in theirs {
            // `their` can be a tree, and it could be used to efficiently prune child-changes as these
            // trees are always rewrites with parent ids (of course we validate), so child-changes could be handled
            // quickly. However, for now the benefit of having these trees is to have them as part of the match-tree
            // on *our* side so that it's clear that we passed a renamed directory (by identity).
            if their.entry_mode().is_tree() {
                continue;
            }

            let maybe_conflict = rewrite_source_tree.check_conflict(&their);
            match maybe_conflict {
                None => {
                    apply_change(&mut editor, &their)?;
                }
                Some(candidate) => {
                    use to_components_bstring_ref as to_components;
                    dbg!(&candidate);
                    match candidate {
                        PossibleConflict::PassedRewrittenDirectory { .. } => {
                            todo!("rewritten directory changes the destination directory of their change by prefix")
                        }
                        PossibleConflict::TreeToNonTree { .. } => {
                            todo!("TreeToNonTree: This can never be reconciled unless we are told which tree to pick (also todo)")
                        }
                        PossibleConflict::NonTreeToTree { our_leaf_node, .. } => {
                            debug_assert!(our_leaf_node.children.is_empty(), "BUG: always a leaf node");
                            todo!("NonTreeToTree: This can never be reconciled unless we are told which tree to pick (also todo)")
                        }
                        PossibleConflict::Match { our_leaf_node } => {
                            debug_assert!(our_leaf_node.children.is_empty(), "BUG: always a leaf node");
                            let ours = our_leaf_node.change.expect("leaf nodes always have a change");
                            match (ours, &their) {
                                (
                                    Change::Modification {
                                        previous_id,
                                        id: our_id,
                                        location: our_location,
                                        entry_mode: our_entry_mode,
                                        ..
                                    },
                                    Change::Rewrite {
                                        source_id: their_source_id,
                                        id: their_id,
                                        location: their_location,
                                        entry_mode: their_entry_mode,
                                        ..
                                    },
                                ) => {
                                    assert_eq!(
                                        previous_id, their_source_id,
                                        "both refer to the same base, so should always match"
                                    );
                                    let renamed_without_change = their_source_id == their_id;
                                    let both_versions_same = our_id == their_id;
                                    if (renamed_without_change || both_versions_same)
                                        && our_entry_mode == their_entry_mode
                                    {
                                        editor.remove(to_components(our_location))?;
                                        editor.upsert(
                                            to_components(their_location),
                                            their_entry_mode.kind(),
                                            *our_id,
                                        )?;
                                    } else {
                                        todo!("needs blob merge, but figure out typechange logic")
                                    }
                                }
                                _ => todo!("all other cases, can probably be matched very well"),
                            }
                        }
                    }
                }
            }
        }

        Ok(Outcome {
            tree: editor,
            conflicts,
        })
    }

    /// Only keep leaf nodes, or trees that are the renamed.
    /// Doing so makes it easy to track renamed or rewritten or copied directories, and properly
    /// handle *their* changes that fall within them.
    fn may_track(change: gix_diff::tree_with_rewrites::ChangeRef<'_>) -> bool {
        !change.entry_mode().is_tree() || matches!(change.relation(), Some(Relation::Parent(_)))
    }

    /// Unconditionally apply `change` to `editor`.
    fn apply_change(editor: &mut tree::Editor<'_>, change: &Change) -> Result<(), gix_object::tree::editor::Error> {
        use to_components_bstring_ref as to_components;

        match change {
            Change::Addition {
                location,
                entry_mode,
                id,
                ..
            }
            | Change::Modification {
                location,
                entry_mode,
                id,
                ..
            } => editor.upsert(to_components(location), entry_mode.kind(), *id)?,
            Change::Deletion { location, .. } => editor.remove(to_components(location))?,
            Change::Rewrite {
                source_location,
                entry_mode,
                id,
                location,
                copy,
                ..
            } => {
                if !*copy {
                    editor.remove(to_components(source_location))?;
                }
                editor.upsert(to_components(location), entry_mode.kind(), *id)?
            }
        };
        Ok(())
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
    struct PathComponent<'a>(&'a BStr);

    /// A potential conflict that needs to be checked. It comes in several varieties and always happens
    /// if paths overlap in some way between *theirs* and *ours*.
    #[derive(Debug)]
    // TODO: remove this when all fields are used.
    #[allow(dead_code)]
    enum PossibleConflict<'a, 'change> {
        /// There are changes below *their* change.
        TreeToNonTree {
            /// The node at the end of *their* location, i.e. the node at `c` of path `a/b/c`, and there is `a/b/c/d`
            /// present in the tree (or more children).
            /// This always happens if `c` was a directory and turned into a non-directory, but can also happen if
            /// *their* change is a directory change.
            /// This also means `node` has children.
            our_node: &'a TreeNode<'change>,
        },
        /// A non-tree in *our* tree turned into a tree in *theirs* - this can be done with additions in *theirs*.
        NonTreeToTree {
            /// The last seen node at the end of the *our* portion of *their* path, i.e. the node at `a/b` when *their*
            /// path is `a/b/c`.
            our_leaf_node: &'a TreeNode<'change>,
        },
        /// A perfect match, i.e. *our* change at `a/b/c` corresponds to *their* change at the same path.
        Match {
            /// *our* node at *their* path, which also means it has no children, and that there is a change.
            our_leaf_node: &'a TreeNode<'change>,
        },
        /// *their* change at `a/b/c` passed `a/b` which is a change indicating a directory that was rewritten,
        /// with all its contents being renamed. However, *theirs* has been added *into* that renamed directory.
        PassedRewrittenDirectory { change: &'a Change },
    }

    impl<'a, 'change> PossibleConflict<'a, 'change> {
        fn non_tree_to_tree(node: &'a TreeNode<'change>) -> Option<Self> {
            // Defensively allow for empty trees, even though they shouldn't exist
            if node.change.expect("leafs are always changes").entry_mode().is_tree() {
                None
            } else {
                Some(PossibleConflict::NonTreeToTree { our_leaf_node: node })
            }
        }
    }

    /// Trees lead to other trees, or leafs (without children), and it can be represented by a renamed directory.
    #[derive(Debug, Default)]
    struct TreeNode<'a> {
        /// A mapping of path components to their children to quickly see if `theirs` in some way is potentially
        /// conflicting with `ours`.
        children: BTreeMap<PathComponent<'a>, TreeNode<'a>>,
        /// A change, which is always set if this is a leaf node (with no children), and if there are children and this
        /// is a rewritten tree.
        change: Option<&'a Change>,
    }

    impl<'a> TreeNode<'a> {
        fn is_leaf_node(&self) -> bool {
            self.children.is_empty()
        }
        /// Insert our `change` into a linked-tree, assuring that each `change` is non-conflicting
        /// with this tree structure, i.e. reach path is only seen once.
        fn track_ours_exclusive(&mut self, change: &'a Change) {
            // TODO(borrowchk): Would love to just 'walk down' but have to use recursive approach (depth-first) to be
            //                  able to build the tree at all :/.
            // TODO: turn this into a stack-based approach similar to what's done in `Editor` actually, is that possible at all
            //       with a tree-structure?
            fn make_tree<'a>(
                node: &mut TreeNode<'a>,
                mut path: impl Iterator<Item = &'a BStr>,
                next: Option<&'a BStr>,
                change: &'a Change,
            ) {
                let Some(component) = next.map(PathComponent) else {
                    return;
                };
                let next = path.next();
                match node.children.entry(component) {
                    Entry::Vacant(entry) => {
                        let mut new_node = TreeNode {
                            children: Default::default(),
                            change: next.is_none().then_some(change),
                        };
                        make_tree(&mut new_node, path, next, change);
                        entry.insert(new_node);
                    }
                    Entry::Occupied(mut entry) => make_tree(entry.get_mut(), path, next, change),
                }
            }

            let mut components = to_components(change.source_location());
            let next = components.next();
            make_tree(self, components, next, change);
        }

        /// Search the tree for `theirs` by [`source_location()`](Change::source_location())).
        fn check_conflict(&self, theirs: &'a Change) -> Option<PossibleConflict<'_, 'a>> {
            let components = to_components(theirs.source_location()).map(PathComponent);
            let mut cursor = self;
            let mut intermediate_change = None;
            for component in components {
                if cursor.change.is_some() {
                    intermediate_change = cursor.change;
                }
                match cursor.children.get(&component) {
                    // *their* change is outside *our* tree
                    None => {
                        let res = if cursor.is_leaf_node() {
                            PossibleConflict::non_tree_to_tree(cursor)
                        } else {
                            // a change somewhere else, i.e. `a/c` and we know `a/b/c` only.
                            intermediate_change.map(|change| PossibleConflict::PassedRewrittenDirectory { change })
                        };
                        return res;
                    }
                    Some(child) => {
                        cursor = child;
                    }
                }
            }

            if cursor.is_leaf_node() {
                PossibleConflict::Match { our_leaf_node: cursor }
            } else {
                PossibleConflict::TreeToNonTree { our_node: cursor }
            }
            .into()
        }
    }

    fn to_components_bstring_ref(rela_path: &BString) -> impl Iterator<Item = &BStr> {
        rela_path.split(|b| *b == b'/').map(Into::into)
    }

    fn to_components(rela_path: &BStr) -> impl Iterator<Item = &BStr> {
        rela_path.split(|b| *b == b'/').map(Into::into)
    }
}

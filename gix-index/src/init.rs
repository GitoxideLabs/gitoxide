///
pub mod from_tree {
    use gix_error::Result;
    use std::collections::VecDeque;

    use bstr::{BStr, BString, ByteSlice, ByteVec};
    use gix_error::{ErrorExt, bail, message};
    use gix_object::{tree, tree::EntryKind};
    use gix_traverse::tree::{Visit, depthfirst, visit::Action};

    use crate::{
        Entry, PathStorage, State, Version,
        entry::{Flags, Mode, Stage, Stat, stat},
    };

    /// Initialization
    impl State {
        /// Return a new and empty in-memory index assuming the given `object_hash`.
        pub fn new(object_hash: gix_hash::Kind) -> Self {
            State {
                object_hash,
                timestamp: filetime::FileTime::now(),
                version: Version::V2,
                entries: vec![],
                path_backing: vec![],
                is_sparse: false,
                tree: None,
                link: None,
                resolve_undo: None,
                untracked: None,
                fs_monitor: None,
                offset_table_at_decode_time: false,
                end_of_index_at_decode_time: false,
            }
        }

        /// Create an index [`State`] by traversing `tree` recursively, accessing sub-trees
        /// with `objects`.
        /// `validate` is used to determine which validations to perform on every path component we see.
        ///
        /// # Security
        ///
        /// This currently trusts tree shape beyond individual path-component validation, and is exploitable with
        /// malicious trees that contain file/directory conflicts like `a` and `a/x`. Such trees can produce an
        /// index [`State`] with inconsistent paths, which may panic or confuse downstream checkout/index consumers.
        ///
        /// A previous normalization pass tried to remove these conflicts after traversal, but only checked adjacent
        /// entries. That was incomplete because [`Entry::cmp_filepaths()`] can order unrelated paths between a file
        /// and its conflicting child, for example `a`, `a.`, `a/x`.
        ///
        /// **No extension data is currently produced**.
        pub fn from_tree<Find>(
            tree: &gix_hash::oid,
            objects: Find,
            validate: gix_validate::path::component::Options,
        ) -> Result<Self>
        where
            Find: gix_object::Find,
        {
            Self::from_tree_inner(tree, objects, validate, None)
        }

        /// Like [`Self::from_tree()`], but reuse worktree metadata from `previous` while collecting entries.
        ///
        /// Entries with the same path, object ID and mode retain their non-racy stat cache and assume-valid flag.
        /// `stat_options` determine whether cached timestamps are racy; racy stat is discarded so writing the new
        /// index cannot make an unverified worktree modification appear clean.
        /// Skip-worktree flags are retained by path, including descendants of expanded sparse-directory entries.
        /// Sparse-checkout patterns are not reevaluated. Conflicts, intent-to-add, removal and transient flags,
        /// and all extensions are discarded. `previous` is never changed and must have sorted entries.
        ///
        /// The path-validation limitations of [`Self::from_tree()`] also apply here.
        pub fn from_tree_with_index<Find>(
            tree: &gix_hash::oid,
            objects: Find,
            validate: gix_validate::path::component::Options,
            previous: &State,
            stat_options: stat::Options,
        ) -> Result<Self>
        where
            Find: gix_object::Find,
        {
            Self::from_tree_inner(tree, objects, validate, Some((previous, stat_options)))
        }

        fn from_tree_inner<Find>(
            tree: &gix_hash::oid,
            objects: Find,
            validate: gix_validate::path::component::Options,
            previous: Option<(&State, stat::Options)>,
        ) -> Result<Self>
        where
            Find: gix_object::Find,
        {
            let _span = gix_trace::coarse!("gix_index::State::from_tree()");
            let mut delegate = CollectEntries::new(validate, previous);
            let traversal = depthfirst(tree.to_owned(), depthfirst::State::default(), &objects, &mut delegate);

            if let Some((path, err)) = delegate.invalid_path.take() {
                bail!(err.and_raise(message!("The path \"{path}\" is invalid").validation()));
            }
            traversal?;

            let CollectEntries {
                entries,
                path_backing,
                path: _,
                path_deque: _,
                validate: _,
                invalid_path: _,
                previous: _,
            } = delegate;

            Ok(State {
                object_hash: tree.kind(),
                timestamp: filetime::FileTime::now(),
                version: Version::V2,
                entries,
                path_backing,
                is_sparse: false,
                tree: None,
                link: None,
                resolve_undo: None,
                untracked: None,
                fs_monitor: None,
                offset_table_at_decode_time: false,
                end_of_index_at_decode_time: false,
            })
        }
    }

    struct CollectEntries<'a> {
        entries: Vec<Entry>,
        path_backing: PathStorage,
        path: BString,
        path_deque: VecDeque<BString>,
        validate: gix_validate::path::component::Options,
        invalid_path: Option<(BString, gix_validate::path::component::Error)>,
        previous: Option<(&'a State, stat::Options)>,
    }

    impl<'a> CollectEntries<'a> {
        pub fn new(
            validate: gix_validate::path::component::Options,
            previous: Option<(&'a State, stat::Options)>,
        ) -> Self {
            CollectEntries {
                entries: Vec::new(),
                path_backing: Vec::new(),
                path: BString::default(),
                path_deque: VecDeque::new(),
                validate,
                invalid_path: None,
                previous,
            }
        }

        fn push_element(&mut self, name: &BStr) {
            if name.is_empty() {
                return;
            }
            if !self.path.is_empty() {
                self.path.push(b'/');
            }
            self.path.push_str(name);
            if self.invalid_path.is_none()
                && let Err(err) = gix_validate::path::component(name, None, self.validate)
            {
                self.invalid_path = Some((self.path.clone(), err));
            }
        }

        pub fn add_entry(&mut self, entry: &tree::EntryRef<'_>) {
            let mode = match entry.mode.kind() {
                EntryKind::Tree => unreachable!("visit_non_tree() called us"),
                EntryKind::Blob => Mode::FILE,
                EntryKind::BlobExecutable => Mode::FILE_EXECUTABLE,
                EntryKind::Link => Mode::SYMLINK,
                EntryKind::Commit => Mode::COMMIT,
            };
            // There are leaf-names that require special validation, specific to their mode.
            // Double-validate just for this case, as the previous validation didn't know the mode yet.
            if self.invalid_path.is_none() {
                let start = self.path.rfind_byte(b'/').map(|pos| pos + 1).unwrap_or_default();
                if let Err(err) = gix_validate::path::component(
                    self.path[start..].as_ref(),
                    (entry.mode.kind() == EntryKind::Link).then_some(gix_validate::path::component::Mode::Symlink),
                    self.validate,
                ) {
                    self.invalid_path = Some((self.path.clone(), err));
                }
            }

            let path_start = self.path_backing.len();
            self.path_backing.extend_from_slice(&self.path);

            let mut new_entry = Entry {
                stat: Stat::default(),
                id: entry.oid.into(),
                flags: Flags::empty(),
                mode,
                path: path_start..self.path_backing.len(),
            };

            if let Some((previous, stat_options)) = self.previous {
                if let Some(old) = previous
                    .entry_by_path_and_stage(self.path.as_ref(), Stage::Unconflicted)
                    .filter(|entry| !entry.flags.contains(Flags::REMOVE))
                {
                    new_entry.flags |= old.flags & Flags::SKIP_WORKTREE;
                    if old.id == new_entry.id && old.mode == new_entry.mode {
                        new_entry.flags |= old.flags & Flags::ASSUME_VALID;
                        if !old.stat.is_racy(previous.timestamp(), stat_options) {
                            new_entry.stat = old.stat;
                        }
                    }
                } else if let Err(position) = previous.entry_index_by_path(self.path.as_ref())
                    && let Some(old) = position
                        .checked_sub(1)
                        .and_then(|position| previous.entries().get(position))
                    && old.mode == Mode::DIR
                    && old.flags.contains(Flags::SKIP_WORKTREE)
                    && !old.flags.contains(Flags::REMOVE)
                    && self.path.starts_with(old.path(previous))
                {
                    new_entry.flags |= Flags::SKIP_WORKTREE;
                }
                if new_entry.flags.contains(Flags::SKIP_WORKTREE) {
                    new_entry.flags |= Flags::EXTENDED;
                }
            }
            self.entries.push(new_entry);
        }

        fn determine_action(&self) -> Action {
            if self.invalid_path.is_none() {
                std::ops::ControlFlow::Continue(true)
            } else {
                std::ops::ControlFlow::Break(())
            }
        }
    }

    impl Visit for CollectEntries<'_> {
        fn pop_back_tracked_path_and_set_current(&mut self) {
            self.path = self.path_deque.pop_back().unwrap_or_default();
        }

        fn pop_front_tracked_path_and_set_current(&mut self) {
            self.path = self
                .path_deque
                .pop_front()
                .expect("every call is matched with push_tracked_path_component");
        }

        fn push_back_tracked_path_component(&mut self, component: &BStr) {
            self.push_element(component);
            self.path_deque.push_back(self.path.clone());
        }

        fn push_path_component(&mut self, component: &BStr) {
            self.push_element(component);
        }

        fn pop_path_component(&mut self) {
            if let Some(pos) = self.path.rfind_byte(b'/') {
                self.path.resize(pos, 0);
            } else {
                self.path.clear();
            }
        }

        fn visit_tree(&mut self, _entry: &gix_object::tree::EntryRef<'_>) -> Action {
            self.determine_action()
        }

        fn visit_nontree(&mut self, entry: &gix_object::tree::EntryRef<'_>) -> Action {
            self.add_entry(entry);
            self.determine_action()
        }
    }
}

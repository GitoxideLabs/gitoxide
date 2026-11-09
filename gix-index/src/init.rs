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
        entry::{Flags, Mode, Stat},
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

        /// Like [`Self::from_tree()`], but reuse entries from `index` when its cache-tree identifies
        /// a matching subtree. Matching trees need not be read from `objects`, including the root.
        ///
        /// Cached paths receive the same component validation as traversed paths, and the returned
        /// entries have empty flags and stat information. No extension data is copied.
        /// Invalid cache nodes, mismatched entry counts, and sparse, unmerged, intent-to-add, or
        /// removed entries fall back to traversal. The cache-tree must describe the current entries;
        /// mutable entry and path access invalidates it to preserve that invariant.
        ///
        /// The tree-shape limitations described in [`Self::from_tree()`] also apply here.
        pub fn from_tree_with_cache<Find>(
            tree: &gix_hash::oid,
            objects: Find,
            validate: gix_validate::path::component::Options,
            index: &State,
        ) -> Result<Self>
        where
            Find: gix_object::Find,
        {
            Self::from_tree_inner(tree, objects, validate, Some(index))
        }

        fn from_tree_inner<Find>(
            tree: &gix_hash::oid,
            objects: Find,
            validate: gix_validate::path::component::Options,
            index: Option<&State>,
        ) -> Result<Self>
        where
            Find: gix_object::Find,
        {
            let _span = gix_trace::coarse!("gix_index::State::from_tree()");
            let mut delegate = CollectEntries::new(
                validate,
                index.filter(|index| !index.is_sparse() && index.object_hash() == tree.kind()),
            );
            let traversal = if delegate.copy_cached_tree(tree) {
                Ok(())
            } else {
                depthfirst(tree.to_owned(), depthfirst::State::default(), &objects, &mut delegate)
            };

            if let Some((path, err)) = delegate.invalid_path.take() {
                bail!(err.and_raise(message!("The path \"{path}\" is invalid").validation()));
            }
            traversal?;

            let CollectEntries {
                entries, path_backing, ..
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

    struct CollectEntries<'index> {
        entries: Vec<Entry>,
        path_backing: PathStorage,
        path: BString,
        path_deque: VecDeque<BString>,
        validate: gix_validate::path::component::Options,
        invalid_path: Option<(BString, gix_validate::path::component::Error)>,
        index: Option<&'index State>,
    }

    impl<'index> CollectEntries<'index> {
        pub fn new(validate: gix_validate::path::component::Options, index: Option<&'index State>) -> Self {
            CollectEntries {
                entries: Vec::new(),
                path_backing: Vec::new(),
                path: BString::default(),
                path_deque: VecDeque::new(),
                validate,
                invalid_path: None,
                index,
            }
        }

        fn copy_cached_tree(&mut self, tree_id: &gix_hash::oid) -> bool {
            let Some(index) = self.index.filter(|_| self.invalid_path.is_none()) else {
                return false;
            };
            let Some(mut cached) = index.tree().filter(|root| root.name.is_empty()) else {
                return false;
            };
            if !self.path.is_empty() {
                for component in self.path.split(|byte| *byte == b'/') {
                    let Ok(position) = cached
                        .children
                        .binary_search_by(|child| child.name.as_slice().cmp(component))
                    else {
                        return false;
                    };
                    cached = &cached.children[position];
                }
            }
            let Some(num_entries) = cached.num_entries.filter(|_| cached.id == tree_id) else {
                return false;
            };
            let range = if self.path.is_empty() {
                0..index.entries().len()
            } else {
                self.path.push(b'/');
                let range = index.prefixed_entries_range(self.path.as_bstr()).unwrap_or(0..0);
                self.path.pop();
                range
            };
            let entries = &index.entries()[range];
            if entries.len() != num_entries as usize
                || entries.iter().any(|entry| {
                    entry
                        .flags
                        .intersects(Flags::STAGE_MASK | Flags::INTENT_TO_ADD | Flags::REMOVE)
                        || !matches!(
                            entry.mode,
                            Mode::FILE | Mode::FILE_EXECUTABLE | Mode::SYMLINK | Mode::COMMIT
                        )
                        || entry.path(index).split(|byte| *byte == b'/').any(<[u8]>::is_empty)
                })
            {
                return false;
            }

            self.entries.reserve(entries.len());
            let mut validated_directory = b"".as_slice();
            for entry in entries {
                let path = entry.path(index);
                let filename_start = path.rfind_byte(b'/').map_or(0, |pos| pos + 1);
                let directory = &path[..filename_start];
                if !directory.is_empty() && directory != validated_directory {
                    let mut end = 0;
                    for component in directory[..directory.len() - 1].split(|byte| *byte == b'/') {
                        end += component.len();
                        if let Err(err) = gix_validate::path::component(component.as_bstr(), None, self.validate) {
                            self.invalid_path = Some((path[..end].into(), err));
                            return true;
                        }
                        end += 1;
                    }
                    validated_directory = directory;
                }
                if let Err(err) = gix_validate::path::component(
                    path[filename_start..].as_bstr(),
                    (entry.mode == Mode::SYMLINK).then_some(gix_validate::path::component::Mode::Symlink),
                    self.validate,
                ) {
                    self.invalid_path = Some((path.into(), err));
                    return true;
                }
                let path_start = self.path_backing.len();
                self.path_backing.extend_from_slice(path);
                self.entries.push(Entry {
                    stat: Stat::default(),
                    id: entry.id,
                    flags: Flags::empty(),
                    mode: entry.mode,
                    path: path_start..self.path_backing.len(),
                });
            }
            true
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

            let new_entry = Entry {
                stat: Stat::default(),
                id: entry.oid.into(),
                flags: Flags::empty(),
                mode,
                path: path_start..self.path_backing.len(),
            };

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

        fn visit_tree(&mut self, entry: &gix_object::tree::EntryRef<'_>) -> Action {
            if self.copy_cached_tree(entry.oid) && self.invalid_path.is_none() {
                std::ops::ControlFlow::Continue(false)
            } else {
                self.determine_action()
            }
        }

        fn visit_nontree(&mut self, entry: &gix_object::tree::EntryRef<'_>) -> Action {
            self.add_entry(entry);
            self.determine_action()
        }
    }
}

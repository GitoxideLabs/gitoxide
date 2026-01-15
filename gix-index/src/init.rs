///
pub mod from_tree {
    use std::collections::VecDeque;

    use bstr::{BStr, BString, ByteSlice, ByteVec};
    use gix_object::{tree, tree::EntryKind};
    use gix_traverse::tree::{depthfirst, visit::Action, Visit};

    use crate::{
        entry::{Flags, Mode, Stat},
        Entry, PathStorage, State, Version,
    };

    /// The error returned by [State::from_tree()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("The path \"{path}\" is invalid")]
        InvalidComponent {
            path: BString,
            source: gix_validate::path::component::Error,
        },
        #[error(transparent)]
        Traversal(#[from] gix_traverse::tree::depthfirst::Error),
    }

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
        /// **No extension data is currently produced**.
        pub fn from_tree<Find>(
            tree: &gix_hash::oid,
            objects: Find,
            validate: gix_validate::path::component::Options,
        ) -> Result<Self, Error>
        where
            Find: gix_object::Find,
        {
            let _span = gix_features::trace::coarse!("gix_index::State::from_tree()");
            let mut delegate = CollectEntries::new(validate);
            match depthfirst(tree.to_owned(), depthfirst::State::default(), &objects, &mut delegate) {
                Ok(()) => {}
                Err(gix_traverse::tree::breadthfirst::Error::Cancelled) => {
                    let (path, err) = delegate
                        .invalid_path
                        .take()
                        .expect("cancellation only happens on validation error");
                    return Err(Error::InvalidComponent { path, source: err });
                }
                Err(err) => return Err(err.into()),
            }

            let CollectEntries {
                entries,
                path_backing,
                path: _,
                path_deque: _,
                validate: _,
                invalid_path,
            } = delegate;

            if let Some((path, err)) = invalid_path {
                return Err(Error::InvalidComponent { path, source: err });
            }

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

    struct CollectEntries {
        entries: Vec<Entry>,
        path_backing: PathStorage,
        path: BString,
        path_deque: VecDeque<BString>,
        validate: gix_validate::path::component::Options,
        invalid_path: Option<(BString, gix_validate::path::component::Error)>,
    }

    impl CollectEntries {
        pub fn new(validate: gix_validate::path::component::Options) -> CollectEntries {
            CollectEntries {
                entries: Vec::new(),
                path_backing: Vec::new(),
                path: BString::default(),
                path_deque: VecDeque::new(),
                validate,
                invalid_path: None,
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
            if self.invalid_path.is_none() {
                if let Err(err) = gix_validate::path::component(name, None, self.validate) {
                    self.invalid_path = Some((self.path.clone(), err));
                }
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

    impl Visit for CollectEntries {
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

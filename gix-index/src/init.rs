///
pub mod from_tree {
    use std::collections::VecDeque;

    use bstr::{BStr, BString, ByteSlice, ByteVec};
    use gix_object::{tree, tree::EntryKind};
    use gix_traverse::tree::{Visit, depthfirst, visit::Action};

    use crate::{
        Entry, PathStorage, State, Version,
        entry::{Flags, Mode, Stat},
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

/// Initialize tree objects from an index state.
pub mod to_tree {
    use std::io::Write;

    use bstr::{BStr, BString, ByteSlice};
    use gix_object::{tree, tree::EntryMode};

    use crate::{
        Entry, State,
        entry::{self, Stage},
        extension,
    };

    /// The error returned by [State::to_tree()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("Entry '{path}' is unmerged at stage {stage}")]
        Unmerged { path: BString, stage: u32 },
        #[error("Entry '{path}' is invalid as both a file and directory would exist in the tree")]
        FileDirectoryConflict { path: BString },
        #[error("The path \"{path}\" is invalid")]
        InvalidComponent {
            path: BString,
            source: gix_validate::path::component::Error,
        },
        #[error("Entry '{path}' has an invalid index mode {mode:?}")]
        InvalidMode { path: BString, mode: entry::Mode },
        #[error("The object {id} at '{path}' does not exist")]
        MissingObject { path: BString, id: gix_hash::ObjectId },
        #[error(transparent)]
        Write(#[from] gix_object::write::Error),
        #[error(transparent)]
        Entries(#[from] crate::verify::entries::Error),
        #[error("More than 4 billion entries would be represented by the tree at '{path}'")]
        EntriesOverflow { path: BString },
    }

    /// Options for use with [State::to_tree()].
    #[derive(Default, Debug, Clone, Copy)]
    pub struct Options {
        /// Path component validation options.
        pub validate: gix_validate::path::component::Options,
        /// If true, don't fail if referenced objects are missing from `objects`.
        ///
        /// Commit entries, representing submodules, are never checked for existence.
        pub missing_ok: bool,
    }

    /// Tree creation.
    impl State {
        /// Write this index state as Git tree objects into `objects` and return the root tree id.
        ///
        /// If this state has a TREE extension, it is refreshed from the written trees on success.
        /// If no TREE extension is present, none is created.
        pub fn to_tree<Db>(&mut self, objects: Db, options: Options) -> Result<gix_hash::ObjectId, Error>
        where
            Db: gix_object::Write + gix_object::Exists,
        {
            let _span = gix_features::trace::coarse!("gix_index::State::to_tree()");
            self.verify_entries()?;
            if let Some(tree) = self.tree.as_ref().filter(|tree| tree.is_fully_valid(&objects)) {
                return Ok(tree.id);
            }
            let update_tree_cache = self.tree.is_some();

            let mut builder = Builder {
                state: self,
                objects: &objects,
                options,
                update_tree_cache,
            };
            let (next_index, written) = builder.write_tree_at(0, BStr::new(b""), BStr::new(b""), true)?;
            debug_assert_eq!(next_index, builder.state.entries.len());
            let written = written.expect("the root tree is always written");
            if update_tree_cache {
                self.tree = written.cache_tree;
            }
            Ok(written.id)
        }
    }

    struct Builder<'a, Db> {
        state: &'a State,
        objects: &'a Db,
        options: Options,
        update_tree_cache: bool,
    }

    struct WrittenTree {
        id: gix_hash::ObjectId,
        num_entries: u32,
        cache_tree: Option<extension::Tree>,
    }

    impl<Db> Builder<'_, Db>
    where
        Db: gix_object::Write + gix_object::Exists,
    {
        fn write_tree_at(
            &mut self,
            mut index: usize,
            prefix: &BStr,
            name: &BStr,
            write_empty: bool,
        ) -> Result<(usize, Option<WrittenTree>), Error> {
            let mut tree_data = Vec::new();
            let mut num_entries = 0u32;
            let mut children = Vec::new();
            let mut leaf_names = Vec::<&BStr>::new();

            while let Some(entry) = self.state.entries.get(index) {
                let path = entry.path(self.state);
                if !path.starts_with(prefix) {
                    break;
                }

                self.assure_unmerged(entry, path)?;

                if entry
                    .flags
                    .intersects(entry::Flags::REMOVE | entry::Flags::INTENT_TO_ADD)
                {
                    index += 1;
                    continue;
                }

                let rela_path = BStr::new(&path[prefix.len()..]);
                if rela_path.is_empty() {
                    return Err(Error::InvalidComponent {
                        path: path.into(),
                        source: gix_validate::path::component(BStr::new(b""), None, self.options.validate)
                            .expect_err("empty component is invalid"),
                    });
                }

                let slash_pos = rela_path.find_byte(b'/');
                let is_sparse_leaf = entry.mode.is_sparse() && slash_pos == Some(rela_path.len() - 1);
                if let Some(slash_pos) = slash_pos.filter(|_| !is_sparse_leaf) {
                    let component = BStr::new(&rela_path[..slash_pos]);
                    let child_path = BStr::new(&path[..prefix.len() + slash_pos]);
                    self.validate_component(child_path, component, None)?;
                    if leaf_names.binary_search_by(|name| (*name).cmp(component)).is_ok() {
                        return Err(Error::FileDirectoryConflict {
                            path: child_path.into(),
                        });
                    }

                    let child_prefix = BStr::new(&path[..prefix.len() + slash_pos + 1]);
                    let (next_index, child) = self.write_tree_at(index, child_prefix, component, false)?;
                    index = next_index;

                    if let Some(child) = child {
                        encode_entry(&mut tree_data, EntryKind::Tree.into(), component, &child.id)?;
                        num_entries = num_entries
                            .checked_add(child.num_entries)
                            .ok_or_else(|| Error::EntriesOverflow { path: prefix.into() })?;
                        if let Some(child_cache_tree) = child.cache_tree {
                            children.push(child_cache_tree);
                        }
                    }
                } else {
                    let filename = if is_sparse_leaf {
                        BStr::new(&rela_path[..rela_path.len() - 1])
                    } else {
                        rela_path
                    };
                    let insertion_pos = match leaf_names.binary_search_by(|name| (*name).cmp(filename)) {
                        Ok(_) => return Err(Error::FileDirectoryConflict { path: path.into() }),
                        Err(pos) => pos,
                    };
                    self.write_entry(&mut tree_data, path, filename, entry)?;
                    leaf_names.insert(insertion_pos, filename);
                    num_entries = num_entries
                        .checked_add(1)
                        .ok_or_else(|| Error::EntriesOverflow { path: prefix.into() })?;
                    index += 1;
                }
            }

            if num_entries == 0 && !write_empty {
                return Ok((index, None));
            }

            let id = self.objects.write_buf(gix_object::Kind::Tree, &tree_data)?;
            let cache_tree = self.update_tree_cache.then(|| extension::Tree {
                name: name.iter().copied().collect(),
                id,
                num_entries: Some(num_entries),
                children,
            });
            Ok((
                index,
                Some(WrittenTree {
                    id,
                    num_entries,
                    cache_tree,
                }),
            ))
        }

        fn assure_unmerged(&self, entry: &Entry, path: &BStr) -> Result<(), Error> {
            let stage = entry.stage();
            if stage != Stage::Unconflicted {
                return Err(Error::Unmerged {
                    path: path.into(),
                    stage: stage as u32,
                });
            }
            Ok(())
        }

        fn write_entry(
            &self,
            tree_data: &mut Vec<u8>,
            path: &BStr,
            filename: &BStr,
            entry: &Entry,
        ) -> Result<(), Error> {
            let mode = entry.mode.to_tree_entry_mode().ok_or_else(|| Error::InvalidMode {
                path: path.into(),
                mode: entry.mode,
            })?;
            self.validate_component(
                path,
                filename,
                mode.is_link().then_some(gix_validate::path::component::Mode::Symlink),
            )?;
            if !self.options.missing_ok && !mode.is_commit() && !self.objects.exists(&entry.id) {
                return Err(Error::MissingObject {
                    path: path.into(),
                    id: entry.id,
                });
            }

            encode_entry(tree_data, mode, filename, &entry.id)?;
            Ok(())
        }

        fn validate_component(
            &self,
            path: &BStr,
            component: &BStr,
            mode: Option<gix_validate::path::component::Mode>,
        ) -> Result<(), Error> {
            gix_validate::path::component(component, mode, self.options.validate)
                .map(|_| ())
                .map_err(|source| Error::InvalidComponent {
                    path: path.into(),
                    source,
                })
        }
    }

    fn encode_entry(
        out: &mut Vec<u8>,
        mode: EntryMode,
        filename: &BStr,
        id: &gix_hash::oid,
    ) -> Result<(), gix_object::write::Error> {
        out.write_all(mode.as_bytes(&mut Default::default()))?;
        out.write_all(b" ")?;
        out.write_all(filename)?;
        out.write_all(b"\0")?;
        out.write_all(id.as_bytes())?;
        Ok(())
    }

    use tree::EntryKind;
}

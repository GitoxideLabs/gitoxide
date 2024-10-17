use crate::tree::{Editor, EntryKind};
use crate::{tree, Tree};
use bstr::{BStr, BString, ByteSlice, ByteVec};
use gix_hash::ObjectId;
use std::cmp::Ordering;
use std::collections::{hash_map, HashMap};
use std::fmt::Formatter;

/// A way to constrain all [tree-edits](Editor) to a given subtree.
pub struct Cursor<'a, 'find> {
    /// The underlying editor
    parent: &'a mut Editor<'find>,
    /// Our own location, used as prefix for all operations.
    /// Note that it's assumed to always contain a tree.
    prefix: BString,
}

impl std::fmt::Debug for Editor<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Editor")
            .field("object_hash", &self.object_hash)
            .field("path_buf", &self.path_buf)
            .field("trees", &self.trees)
            .finish()
    }
}

/// The error returned by [Editor] or [Cursor] edit operation.
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Empty path components are not allowed")]
    EmptyPathComponent,
    #[error(transparent)]
    FindExistingObject(#[from] crate::find::existing_object::Error),
}

/// Lifecycle
impl<'a> Editor<'a> {
    /// Create a new editor that uses `root` as base for all edits. Use `find` to lookup existing
    /// trees when edits are made. Each tree will only be looked-up once and then edited in place from
    /// that point on.
    /// `object_hash` denotes the kind of hash to create.
    pub fn new(root: Tree, find: &'a dyn crate::FindExt, object_hash: gix_hash::Kind) -> Self {
        Editor {
            find,
            object_hash,
            trees: HashMap::from_iter(Some((empty_path(), root))),
            path_buf: BString::from(Vec::with_capacity(256)).into(),
            tree_buf: Vec::with_capacity(512),
        }
    }
}

/// Operations
impl Editor<'_> {
    /// Write the entire in-memory state of all changed trees (and only changed trees) to `out`, and remove
    /// written portions from our state except for the root tree, which affects [`get()`](Editor::get()).
    /// Note that the returned object id *can* be the empty tree if everything was removed or if nothing
    /// was added to the tree.
    ///
    /// The last call to `out` will be the changed root tree, whose object-id will also be returned.
    /// `out` is free to do any kind of additional validation, like to assure that all entries in the tree exist.
    /// We don't assure that as there is no validation that inserted entries are valid object ids.
    ///
    /// Future calls to [`upsert`](Self::upsert) or similar will keep working on the last seen state of the
    /// just-written root-tree.
    /// If this is not desired, use [set_root()](Self::set_root()).
    ///
    /// ### Validation
    ///
    /// Note that no additional validation is performed to assure correctness of entry-names.
    /// It is absolutely and intentionally possible to write out invalid trees with this method.
    /// Higher layers are expected to perform detailed validation.
    pub fn write<E>(&mut self, out: impl FnMut(&Tree) -> Result<ObjectId, E>) -> Result<ObjectId, E> {
        self.path_buf.borrow_mut().clear();
        self.write_at_pathbuf(out, WriteMode::Normal)
    }

    /// Remove the entry at `rela_path`, loading all trees on the path accordingly.
    /// It's no error if the entry doesn't exist, or if `rela_path` doesn't lead to an existing entry at all.
    ///
    /// Note that trying to remove a path with an empty component is also forbidden.
    pub fn remove<I, C>(&mut self, rela_path: I) -> Result<&mut Self, Error>
    where
        I: IntoIterator<Item = C>,
        C: AsRef<BStr>,
    {
        self.path_buf.borrow_mut().clear();
        self.upsert_or_remove_at_pathbuf(rela_path, None)
    }

    /// Obtain the entry at `rela_path` or return `None` if none was found, or the tree wasn't yet written
    /// to that point.
    /// Note that after [writing](Self::write) only the root path remains, all other intermediate trees are removed.
    /// The entry can be anything that can be stored in a tree, but may have a null-id if it's a newly
    /// inserted tree. Also, ids of trees might not be accurate as they may have been changed in memory.
    pub fn get<I, C>(&self, rela_path: I) -> Option<&tree::Entry>
    where
        I: IntoIterator<Item = C>,
        C: AsRef<BStr>,
    {
        self.path_buf.borrow_mut().clear();
        self.get_inner(rela_path)
    }

    /// Insert a new entry of `kind` with `id` at `rela_path`, an iterator over each path component in the tree,
    /// like `a/b/c`. Names are matched case-sensitively.
    ///
    /// Existing leaf-entries will be overwritten unconditionally, and it is assumed that `id` is available in the object database
    /// or will be made available at a later point to assure the integrity of the produced tree.
    ///
    /// Intermediate trees will be created if they don't exist in the object database, otherwise they will be loaded and entries
    /// will be inserted into them instead.
    ///
    /// Note that `id` can be [null](ObjectId::null()) to create a placeholder. These will not be written, and paths leading
    /// through them will not be considered a problem.
    ///
    /// `id` can also be an empty tree, along with [the respective `kind`](EntryKind::Tree), even though that's normally not allowed
    /// in Git trees.
    pub fn upsert<I, C>(&mut self, rela_path: I, kind: EntryKind, id: ObjectId) -> Result<&mut Self, Error>
    where
        I: IntoIterator<Item = C>,
        C: AsRef<BStr>,
    {
        self.path_buf.borrow_mut().clear();
        self.upsert_or_remove_at_pathbuf(rela_path, Some((kind, id, UpsertMode::Normal)))
    }

    fn get_inner<I, C>(&self, rela_path: I) -> Option<&tree::Entry>
    where
        I: IntoIterator<Item = C>,
        C: AsRef<BStr>,
    {
        let mut path_buf = self.path_buf.borrow_mut();
        let mut cursor = self.trees.get(path_buf.as_bstr()).expect("root is always present");
        let mut rela_path = rela_path.into_iter().peekable();
        while let Some(name) = rela_path.next() {
            let name = name.as_ref();
            let is_last = rela_path.peek().is_none();
            match cursor
                .entries
                .binary_search_by(|e| cmp_entry_with_name(e, name, true))
                .or_else(|_| cursor.entries.binary_search_by(|e| cmp_entry_with_name(e, name, false)))
            {
                Ok(idx) if is_last => return Some(&cursor.entries[idx]),
                Ok(idx) => {
                    if cursor.entries[idx].mode.is_tree() {
                        push_path_component(&mut path_buf, name);
                        cursor = self.trees.get(path_buf.as_bstr())?;
                    } else {
                        break;
                    }
                }
                Err(_) => break,
            };
        }
        None
    }

    fn write_at_pathbuf<E>(
        &mut self,
        mut out: impl FnMut(&Tree) -> Result<ObjectId, E>,
        mode: WriteMode,
    ) -> Result<ObjectId, E> {
        assert_ne!(self.trees.len(), 0, "there is at least the root tree");

        // back is for children, front is for parents.
        let path_buf = self.path_buf.borrow_mut();
        let mut parents = vec![(
            None::<usize>,
            path_buf.clone(),
            self.trees
                .remove(path_buf.as_bstr())
                .expect("root tree is always present"),
        )];
        let mut children = Vec::new();
        while let Some((parent_idx, mut rela_path, mut tree)) = children.pop().or_else(|| parents.pop()) {
            let mut all_entries_unchanged_or_written = true;
            for entry in &tree.entries {
                if entry.mode.is_tree() {
                    let prev_len = push_path_component(&mut rela_path, &entry.filename);
                    if let Some(sub_tree) = self.trees.remove(&rela_path) {
                        all_entries_unchanged_or_written = false;
                        let next_parent_idx = parents.len();
                        children.push((Some(next_parent_idx), rela_path.clone(), sub_tree));
                    }
                    rela_path.truncate(prev_len);
                }
            }
            if all_entries_unchanged_or_written {
                tree.entries.retain(|e| !e.oid.is_null());
                if let Some((_, _, parent_to_adjust)) =
                    parent_idx.map(|idx| parents.get_mut(idx).expect("always present, pointing towards zero"))
                {
                    let name = filename(rela_path.as_bstr());
                    let entry_idx = parent_to_adjust
                        .entries
                        .binary_search_by(|e| cmp_entry_with_name(e, name, true))
                        .expect("the parent always knows us by name");
                    if tree.entries.is_empty() {
                        parent_to_adjust.entries.remove(entry_idx);
                    } else {
                        match out(&tree) {
                            Ok(id) => {
                                parent_to_adjust.entries[entry_idx].oid = id;
                            }
                            Err(err) => {
                                let root_tree = parents.into_iter().next().expect("root wasn't consumed yet");
                                self.trees.insert(root_tree.1, root_tree.2);
                                return Err(err);
                            }
                        }
                    }
                } else if parents.is_empty() {
                    debug_assert!(children.is_empty(), "we consume children before parents");
                    debug_assert_eq!(rela_path, **path_buf, "this should always be the root tree");

                    // There may be left-over trees if they are replaced with blobs for example.
                    match out(&tree) {
                        Ok(id) => {
                            let root_tree_id = id;
                            match mode {
                                WriteMode::Normal => {
                                    self.trees.clear();
                                }
                                WriteMode::FromCursor => {}
                            }
                            self.trees.insert(rela_path, tree);
                            return Ok(root_tree_id);
                        }
                        Err(err) => {
                            self.trees.insert(rela_path, tree);
                            return Err(err);
                        }
                    }
                } else if !tree.entries.is_empty() {
                    out(&tree)?;
                }
            } else {
                parents.push((parent_idx, rela_path, tree));
            }
        }

        unreachable!("we exit as soon as everything is consumed")
    }

    fn upsert_or_remove_at_pathbuf<I, C>(
        &mut self,
        rela_path: I,
        kind_and_id: Option<(EntryKind, ObjectId, UpsertMode)>,
    ) -> Result<&mut Self, Error>
    where
        I: IntoIterator<Item = C>,
        C: AsRef<BStr>,
    {
        let mut path_buf = self.path_buf.borrow_mut();
        let mut cursor = self.trees.get_mut(path_buf.as_bstr()).expect("root is always present");
        let mut rela_path = rela_path.into_iter().peekable();
        let new_kind_is_tree = kind_and_id.map_or(false, |(kind, _, _)| kind == EntryKind::Tree);
        while let Some(name) = rela_path.next() {
            let name = name.as_ref();
            if name.is_empty() {
                return Err(Error::EmptyPathComponent);
            }
            let is_last = rela_path.peek().is_none();
            let mut needs_sorting = false;
            let current_level_must_be_tree = !is_last || new_kind_is_tree;
            let check_type_change = |entry: &tree::Entry| entry.mode.is_tree() != current_level_must_be_tree;
            let tree_to_lookup = match cursor
                .entries
                .binary_search_by(|e| cmp_entry_with_name(e, name, false))
                .or_else(|file_insertion_idx| {
                    cursor
                        .entries
                        .binary_search_by(|e| cmp_entry_with_name(e, name, true))
                        .map_err(|dir_insertion_index| {
                            if current_level_must_be_tree {
                                dir_insertion_index
                            } else {
                                file_insertion_idx
                            }
                        })
                }) {
                Ok(idx) => {
                    match kind_and_id {
                        None => {
                            if is_last {
                                cursor.entries.remove(idx);
                                break;
                            } else {
                                let entry = &cursor.entries[idx];
                                if entry.mode.is_tree() {
                                    Some(entry.oid)
                                } else {
                                    break;
                                }
                            }
                        }
                        Some((kind, id, _mode)) => {
                            let entry = &mut cursor.entries[idx];
                            if is_last {
                                // unconditionally overwrite what's there.
                                entry.oid = id;
                                needs_sorting = check_type_change(entry);
                                entry.mode = kind.into();
                                None
                            } else if entry.mode.is_tree() {
                                // Possibly lookup the existing tree on our way down the path.
                                Some(entry.oid)
                            } else {
                                // it is no tree, but we are traversing a path, so turn it into one.
                                entry.oid = id.kind().null();
                                needs_sorting = check_type_change(entry);
                                entry.mode = EntryKind::Tree.into();
                                None
                            }
                        }
                    }
                }
                Err(insertion_idx) => match kind_and_id {
                    None => break,
                    Some((kind, id, _mode)) => {
                        cursor.entries.insert(
                            insertion_idx,
                            tree::Entry {
                                filename: name.into(),
                                mode: if is_last { kind.into() } else { EntryKind::Tree.into() },
                                oid: if is_last { id } else { id.kind().null() },
                            },
                        );
                        None
                    }
                },
            };
            if needs_sorting {
                cursor.entries.sort();
            }
            if is_last && kind_and_id.map_or(false, |(_, _, mode)| mode == UpsertMode::Normal) {
                break;
            }
            push_path_component(&mut path_buf, name);
            cursor = match self.trees.entry(path_buf.clone()) {
                hash_map::Entry::Occupied(e) => e.into_mut(),
                hash_map::Entry::Vacant(e) => e.insert(
                    if let Some(tree_id) = tree_to_lookup.filter(|tree_id| !tree_id.is_empty_tree()) {
                        self.find.find_tree(&tree_id, &mut self.tree_buf)?.into()
                    } else {
                        Tree::default()
                    },
                ),
            };
        }
        drop(path_buf);
        Ok(self)
    }

    /// Set the root tree of the modification to `root`, assuring it has a well-known state.
    ///
    /// Note that this erases all previous edits.
    ///
    /// This is useful if the same editor is re-used for various trees.
    pub fn set_root(&mut self, root: Tree) -> &mut Self {
        self.trees.clear();
        self.trees.insert(empty_path(), root);
        self
    }
}

mod cursor {
    use crate::tree::editor::{Cursor, UpsertMode, WriteMode};
    use crate::tree::{Editor, EntryKind};
    use crate::{tree, Tree};
    use bstr::{BStr, BString};
    use gix_hash::ObjectId;

    /// Cursor handling
    impl<'a> Editor<'a> {
        /// Turn ourselves as a cursor, which points to the same tree as the editor.
        ///
        /// This is useful if a method takes a [`Cursor`], not an [`Editor`].
        pub fn to_cursor(&mut self) -> Cursor<'_, 'a> {
            Cursor {
                parent: self,
                prefix: BString::default(),
            }
        }

        /// Create a cursor at the given `rela_path`, which must be a tree or is turned into a tree as its own edit.
        ///
        /// The returned cursor will then allow applying edits to the tree at `rela_path` as root.
        /// If `rela_path` is a single empty string, it is equivalent to using the current instance itself.
        pub fn cursor_at<I, C>(&mut self, rela_path: I) -> Result<Cursor<'_, 'a>, super::Error>
        where
            I: IntoIterator<Item = C>,
            C: AsRef<BStr>,
        {
            self.path_buf.borrow_mut().clear();
            self.upsert_or_remove_at_pathbuf(
                rela_path,
                Some((EntryKind::Tree, self.object_hash.null(), UpsertMode::AssureTreeOnly)),
            )?;
            let prefix = self.path_buf.borrow_mut().clone();
            Ok(Cursor {
                prefix, /* set during the upsert call */
                parent: self,
            })
        }
    }

    impl Cursor<'_, '_> {
        /// Obtain the entry at `rela_path` or return `None` if none was found, or the tree wasn't yet written
        /// to that point.
        /// Note that after [writing](Self::write) only the root path remains, all other intermediate trees are removed.
        /// The entry can be anything that can be stored in a tree, but may have a null-id if it's a newly
        /// inserted tree. Also, ids of trees might not be accurate as they may have been changed in memory.
        pub fn get<I, C>(&self, rela_path: I) -> Option<&tree::Entry>
        where
            I: IntoIterator<Item = C>,
            C: AsRef<BStr>,
        {
            self.parent.path_buf.borrow_mut().clone_from(&self.prefix);
            self.parent.get_inner(rela_path)
        }

        /// Like [`Editor::upsert()`], but with the constraint of only editing in this cursor's tree.
        pub fn upsert<I, C>(&mut self, rela_path: I, kind: EntryKind, id: ObjectId) -> Result<&mut Self, super::Error>
        where
            I: IntoIterator<Item = C>,
            C: AsRef<BStr>,
        {
            self.parent.path_buf.borrow_mut().clone_from(&self.prefix);
            self.parent
                .upsert_or_remove_at_pathbuf(rela_path, Some((kind, id, UpsertMode::Normal)))?;
            Ok(self)
        }

        /// Like [`Editor::remove()`], but with the constraint of only editing in this cursor's tree.
        pub fn remove<I, C>(&mut self, rela_path: I) -> Result<&mut Self, super::Error>
        where
            I: IntoIterator<Item = C>,
            C: AsRef<BStr>,
        {
            self.parent.path_buf.borrow_mut().clone_from(&self.prefix);
            self.parent.upsert_or_remove_at_pathbuf(rela_path, None)?;
            Ok(self)
        }

        /// Like [`Editor::write()`], but will write only the subtree of the cursor.
        pub fn write<E>(&mut self, out: impl FnMut(&Tree) -> Result<ObjectId, E>) -> Result<ObjectId, E> {
            self.parent.path_buf.borrow_mut().clone_from(&self.prefix);
            self.parent.write_at_pathbuf(out, WriteMode::FromCursor)
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum UpsertMode {
    Normal,
    /// Only make sure there is a tree at the given location (requires kind tree and null-id)
    AssureTreeOnly,
}

enum WriteMode {
    Normal,
    /// Perform less cleanup to assure parent-editor still stays intact
    FromCursor,
}

fn cmp_entry_with_name(a: &tree::Entry, filename: &BStr, is_tree: bool) -> Ordering {
    let common = a.filename.len().min(filename.len());
    a.filename[..common].cmp(&filename[..common]).then_with(|| {
        let a = a.filename.get(common).or_else(|| a.mode.is_tree().then_some(&b'/'));
        let b = filename.get(common).or_else(|| is_tree.then_some(&b'/'));
        a.cmp(&b)
    })
}

fn filename(path: &BStr) -> &BStr {
    path.rfind_byte(b'/').map_or(path, |pos| &path[pos + 1..])
}

fn empty_path() -> BString {
    BString::default()
}

fn push_path_component(base: &mut BString, component: &[u8]) -> usize {
    let prev_len = base.len();
    debug_assert!(base.last() != Some(&b'/'));
    if !base.is_empty() {
        base.push_byte(b'/');
    }
    base.push_str(component);
    prev_len
}

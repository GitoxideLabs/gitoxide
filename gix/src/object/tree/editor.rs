use crate::bstr::{BStr, BString};
use crate::prelude::ObjectIdExt;
use crate::{Id, Repository};
use gix_hash::ObjectId;
use gix_object::tree::EntryKind;

///
pub mod init {
    /// The error returned by [`Editor::new()](crate::object::tree::Editor::new()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        DecodeTree(#[from] gix_object::decode::Error),
        #[error(transparent)]
        ValidationOptions(#[from] crate::config::boolean::Error),
    }
}

///
pub mod write {
    use crate::bstr::BString;

    /// The error returned by [`Editor::write()](crate::object::tree::Editor::write()) and [`Cursor::write()](super::Cursor::write).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        WriteTree(#[from] crate::object::write::Error),
        #[error("The object {} ({}) at '{}' could not be found", id, kind.as_octal_str(), filename)]
        MissingObject {
            filename: BString,
            kind: gix_object::tree::EntryKind,
            id: gix_hash::ObjectId,
        },
        #[error("The object {} ({}) has an invalid filename: '{}'", id, kind.as_octal_str(), filename)]
        InvalidFilename {
            filename: BString,
            kind: gix_object::tree::EntryKind,
            id: gix_hash::ObjectId,
            source: gix_validate::path::component::Error,
        },
    }
}

/// A cursor at a specific portion of a tree to [edit](super::Editor).
pub struct Cursor<'a, 'repo> {
    inner: gix_object::tree::editor::Cursor<'a, 'repo>,
    validate: gix_validate::path::component::Options,
    repo: &'repo Repository,
}

/// Lifecycle
impl<'repo> super::Editor<'repo> {
    /// Initialize a new editor from the given `tree`.
    pub fn new(tree: &crate::Tree<'repo>) -> Result<Self, init::Error> {
        let tree_ref = tree.decode()?;
        let repo = tree.repo;
        let validate = repo.config.protect_options()?;
        Ok(super::Editor {
            inner: gix_object::tree::Editor::new(tree_ref.into(), &repo.objects, repo.object_hash()),
            validate,
            repo,
        })
    }

    /// Detach all extras and return the underlying plumbing editor, which won't perform validation
    /// when writing the tree.
    pub fn detach(self) -> gix_object::tree::Editor<'repo> {
        self.inner
    }
}

/// Tree editing
#[cfg(feature = "tree-editor")]
impl<'repo> crate::Tree<'repo> {
    /// Start editing a new tree based on this one.
    #[doc(alias = "treebuilder", alias = "git2")]
    pub fn edit(&self) -> Result<super::Editor<'repo>, init::Error> {
        super::Editor::new(self)
    }
}

/// Obtain an iterator over `BStr`-components.
///
/// Note that the implementation is simple, and it's mainly meant for statically known strings
/// or locations obtained during a merge.
pub trait ToComponents {
    /// Return an iterator over the components of a path, without the separator.
    fn to_components(&self) -> impl Iterator<Item = &BStr>;
}

impl ToComponents for &str {
    fn to_components(&self) -> impl Iterator<Item = &BStr> {
        self.split('/').map(Into::into)
    }
}

impl ToComponents for String {
    fn to_components(&self) -> impl Iterator<Item = &BStr> {
        self.split('/').map(Into::into)
    }
}

impl ToComponents for &String {
    fn to_components(&self) -> impl Iterator<Item = &BStr> {
        self.split('/').map(Into::into)
    }
}

impl ToComponents for BString {
    fn to_components(&self) -> impl Iterator<Item = &BStr> {
        self.split(|b| *b == b'/').map(Into::into)
    }
}

impl ToComponents for &BString {
    fn to_components(&self) -> impl Iterator<Item = &BStr> {
        self.split(|b| *b == b'/').map(Into::into)
    }
}

impl ToComponents for &BStr {
    fn to_components(&self) -> impl Iterator<Item = &BStr> {
        self.split(|b| *b == b'/').map(Into::into)
    }
}

/// Cursor Handling
impl<'repo> super::Editor<'repo> {
    /// Turn ourselves as a cursor, which points to the same tree as the editor.
    ///
    /// This is useful if a method takes a [`Cursor`], not an [`Editor`](super::Editor).
    pub fn to_cursor(&mut self) -> Cursor<'_, 'repo> {
        Cursor {
            inner: self.inner.to_cursor(),
            validate: self.validate,
            repo: self.repo,
        }
    }

    /// Create a cursor at the given `rela_path`, which must be a tree or is turned into a tree as its own edit.
    ///
    /// The returned cursor will then allow applying edits to the tree at `rela_path` as root.
    /// If `rela_path` is a single empty string, it is equivalent to using the current instance itself.
    pub fn cursor_at(
        &mut self,
        rela_path: impl ToComponents,
    ) -> Result<Cursor<'_, 'repo>, gix_object::tree::editor::Error> {
        Ok(Cursor {
            inner: self.inner.cursor_at(rela_path.to_components())?,
            validate: self.validate,
            repo: self.repo,
        })
    }
}
/// Operations
impl<'repo> Cursor<'_, 'repo> {
    /// Like [`Editor::upsert()`](super::Editor::upsert()), but with the constraint of only editing in this cursor's tree.
    pub fn upsert(
        &mut self,
        rela_path: impl ToComponents,
        kind: EntryKind,
        id: impl Into<ObjectId>,
    ) -> Result<&mut Self, gix_object::tree::editor::Error> {
        self.inner.upsert(rela_path.to_components(), kind, id.into())?;
        Ok(self)
    }

    /// Like [`Editor::remove()`](super::Editor::remove), but with the constraint of only editing in this cursor's tree.
    pub fn remove(&mut self, rela_path: impl ToComponents) -> Result<&mut Self, gix_object::tree::editor::Error> {
        self.inner.remove(rela_path.to_components())?;
        Ok(self)
    }

    /// Like [`Editor::write()`](super::Editor::write()), but will write only the subtree of the cursor.
    pub fn write(&mut self) -> Result<Id<'repo>, write::Error> {
        write_cursor(self)
    }

    /// Obtain the entry at `rela_path` or return `None` if none was found, or the tree wasn't yet written
    /// to that point.
    /// The root tree is always available.
    /// Note that after [writing](Self::write) only the root path remains, all other intermediate trees are removed.
    /// The entry can be anything that can be stored in a tree, but may have a null-id if it's a newly
    /// inserted tree. Also, ids of trees might not be accurate as they may have been changed in memory.
    pub fn get(&self, rela_path: impl ToComponents) -> Option<crate::object::tree::EntryRef<'repo, '_>> {
        self.inner
            .get(rela_path.to_components())
            .map(|entry| crate::object::tree::EntryRef {
                inner: entry.into(),
                repo: self.repo,
            })
    }
}

/// Operations
impl<'repo> super::Editor<'repo> {
    /// Set the root tree of the modification to `root`, assuring it has a well-known state.
    ///
    /// Note that this erases all previous edits.
    ///
    /// This is useful if the same editor is re-used for various trees.
    pub fn set_root(&mut self, root: &crate::Tree<'repo>) -> Result<&mut Self, init::Error> {
        let new_editor = super::Editor::new(root)?;
        self.inner = new_editor.inner;
        self.repo = new_editor.repo;
        Ok(self)
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
    ///
    /// Validation of path-components will not be performed here, but when [writing the tree](Self::write()).
    pub fn upsert(
        &mut self,
        rela_path: impl ToComponents,
        kind: EntryKind,
        id: impl Into<ObjectId>,
    ) -> Result<&mut Self, gix_object::tree::editor::Error> {
        self.inner.upsert(rela_path.to_components(), kind, id.into())?;
        Ok(self)
    }

    /// Remove the entry at `rela_path`, loading all trees on the path accordingly.
    /// It's no error if the entry doesn't exist, or if `rela_path` doesn't lead to an existing entry at all.
    pub fn remove(&mut self, rela_path: impl ToComponents) -> Result<&mut Self, gix_object::tree::editor::Error> {
        self.inner.remove(rela_path.to_components())?;
        Ok(self)
    }

    /// Write the entire in-memory state of all changed trees (and only changed trees) to the object database.
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
    /// Before writing a tree, all of its entries (not only added ones), will be validated to assure they are
    /// correct. The objects pointed to by entries also have to exist already.
    pub fn write(&mut self) -> Result<Id<'repo>, write::Error> {
        write_cursor(&mut self.to_cursor())
    }

    /// Obtain the entry at `rela_path` or return `None` if none was found, or the tree wasn't yet written
    /// to that point.
    /// The root tree is always available.
    /// Note that after [writing](Self::write) only the root path remains, all other intermediate trees are removed.
    /// The entry can be anything that can be stored in a tree, but may have a null-id if it's a newly
    /// inserted tree. Also, ids of trees might not be accurate as they may have been changed in memory.
    pub fn get(&self, rela_path: impl ToComponents) -> Option<crate::object::tree::EntryRef<'repo, '_>> {
        self.inner
            .get(rela_path.to_components())
            .map(|entry| crate::object::tree::EntryRef {
                inner: entry.into(),
                repo: self.repo,
            })
    }
}

fn write_cursor<'repo>(cursor: &mut Cursor<'_, 'repo>) -> Result<Id<'repo>, write::Error> {
    cursor
        .inner
        .write(|tree| -> Result<ObjectId, write::Error> {
            for entry in &tree.entries {
                gix_validate::path::component(
                    entry.filename.as_ref(),
                    entry
                        .mode
                        .is_link()
                        .then_some(gix_validate::path::component::Mode::Symlink),
                    cursor.validate,
                )
                .map_err(|err| write::Error::InvalidFilename {
                    filename: entry.filename.clone(),
                    kind: entry.mode.into(),
                    id: entry.oid,
                    source: err,
                })?;
                if !entry.mode.is_commit() && !cursor.repo.has_object(entry.oid) {
                    return Err(write::Error::MissingObject {
                        filename: entry.filename.clone(),
                        kind: entry.mode.into(),
                        id: entry.oid,
                    });
                }
            }
            Ok(cursor.repo.write_object(tree)?.detach())
        })
        .map(|id| id.attach(cursor.repo))
}

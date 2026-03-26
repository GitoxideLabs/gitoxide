use std::ops::ControlFlow;

use gix_hash::ObjectId;
pub use gix_object::tree::{EntryKind, EntryMode};
use gix_object::{bstr::BStr, tree::next_entry, FindExt, TreeRefIter};

use crate::{object::find, Id, ObjectDetached, Repository, Tree};

/// All state needed to conveniently edit a tree, using only [update-or-insert](Editor::upsert()) and [removals](Editor::remove()).
#[cfg(feature = "tree-editor")]
#[derive(Clone)]
pub struct Editor<'repo> {
    pub(crate) inner: gix_object::tree::Editor<'repo>,
    pub(crate) validate: gix_validate::path::component::Options,
    /// The owning repository.
    pub repo: &'repo crate::Repository,
}

/// Initialization
impl<'repo> Tree<'repo> {
    /// Obtain a tree instance by handing in all components that it is made up of.
    pub fn from_data(id: impl Into<ObjectId>, data: Vec<u8>, repo: &'repo crate::Repository) -> Self {
        Tree {
            id: id.into(),
            data,
            repo,
        }
    }
}

/// Access
impl<'repo> Tree<'repo> {
    /// Return this tree's identifier.
    pub fn id(&self) -> Id<'repo> {
        Id::from_id(self.id, self.repo)
    }

    /// Parse our tree data and return the parse tree for direct access to its entries.
    pub fn decode(&self) -> Result<gix_object::TreeRef<'_>, gix_object::decode::Error> {
        gix_object::TreeRef::from_bytes(&self.data)
    }

    /// Find the entry named `name` by iteration, or return `None` if it wasn't found.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
    /// # let repo = doctest::open_repo(doctest::basic_repo_dir()?)?;
    /// let tree = repo.head_tree()?;
    ///
    /// assert_eq!(tree.find_entry("this").expect("present").filename(), "this");
    /// assert!(tree.find_entry("does-not-exist").is_none());
    /// # Ok(()) }
    /// ```
    pub fn find_entry(&self, name: impl PartialEq<BStr>) -> Option<EntryRef<'repo, '_>> {
        TreeRefIter::from_bytes(&self.data)
            .filter_map(Result::ok)
            .find(|entry| name.eq(entry.filename))
            .map(|entry| EntryRef {
                inner: entry,
                repo: self.repo,
            })
    }

    /// Follow a sequence of `path` components starting from this instance, and look them up one by one until the last component
    /// is looked up and its tree entry is returned.
    ///
    /// # Performance Notes
    ///
    /// Searching tree entries is currently done in sequence, which allows to the search to be allocation free. It would be possible
    /// to reuse a vector and use a binary search instead, which might be able to improve performance over all.
    /// However, a benchmark should be created first to have some data and see which trade-off to choose here.
    ///
    pub fn lookup_entry<I, P>(&self, path: I) -> Result<Option<Entry<'repo>>, find::existing::Error>
    where
        I: IntoIterator<Item = P>,
        P: PartialEq<BStr>,
    {
        let buf = &mut self.repo.empty_reusable_buffer();
        buf.extend_from_slice(&self.data);

        let mut iter = path.into_iter().peekable();
        let mut data = gix_object::Data::new(gix_object::Kind::Tree, buf);

        loop {
            data = match next_entry(&mut iter, data) {
                ControlFlow::Continue(oid) => self.repo.find(&oid, buf)?,
                ControlFlow::Break(entry) => {
                    let mapped = entry.map(|e| Entry {
                        inner: e.into(),
                        repo: self.repo,
                    });

                    break Ok(mapped);
                }
            }
        }
    }

    /// Follow a sequence of `path` components starting from this instance, and look them up one by one until the last component
    /// is looked up and its tree entry is returned, while changing this instance to point to the last seen tree.
    /// If the returned entry itself is a tree, this instance is updated to point to it as well.
    /// It's useful to have this function to be able to reuse the internal buffer of the tree.
    ///
    /// # Performance Notes
    ///
    /// Searching tree entries is currently done in sequence, which allows to the search to be allocation free.
    /// This is beneficial for most 'common' repositiries, but for other cases an allocation might be preferable
    /// to allow a bisect.
    pub fn peel_to_entry<I, P>(&mut self, path: I) -> Result<Option<Entry<'repo>>, find::existing::Error>
    where
        I: IntoIterator<Item = P>,
        P: PartialEq<BStr>,
    {
        let mut iter = path.into_iter().peekable();
        let mut data = gix_object::Data::new(gix_object::Kind::Tree, &self.data);
        let mut data_id = self.id;

        loop {
            data = match next_entry(&mut iter, data) {
                ControlFlow::Continue(id) => {
                    let res = self.repo.find(&id, &mut self.data)?;
                    data_id = id;
                    if res.kind.is_tree() {
                        self.id = data_id;
                    }
                    res
                }
                ControlFlow::Break(entry) => {
                    let entry = if let Some(e) = entry {
                        let inner = e.into();

                        if e.mode.is_tree() {
                            let id = e.oid.to_owned();
                            data_id = id;
                            self.repo.find(&id, &mut self.data)?;
                            self.id = data_id;
                        }

                        Some(Entry { inner, repo: self.repo })
                    } else {
                        None
                    };

                    if data_id != self.id {
                        // Ensure that our data always matches our id, even if this means an extra lookup.
                        self.repo.find(&self.id, &mut self.data)?;
                    }

                    break Ok(entry);
                }
            }
        }
    }

    /// Like [`Self::lookup_entry()`], but takes a `Path` directly via `relative_path`, a path relative to this tree.
    ///
    /// # Note
    ///
    /// If any path component contains illformed UTF-8 and thus can't be converted to bytes on platforms which can't do so natively,
    /// the returned component will be empty which makes the lookup fail.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
    /// # let repo = doctest::open_repo(doctest::worktree_repo_dir()?)?;
    /// let tree = repo.head_commit()?.tree()?;
    /// let entry = tree.lookup_entry_by_path("dir/c")?.expect("present");
    ///
    /// assert_eq!(entry.filename(), "c");
    /// # Ok(()) }
    /// ```
    pub fn lookup_entry_by_path(
        &self,
        relative_path: impl AsRef<std::path::Path>,
    ) -> Result<Option<Entry<'repo>>, find::existing::Error> {
        self.lookup_entry(
            relative_path
                .as_ref()
                .components()
                .map(|c| c.as_os_str().as_encoded_bytes()),
        )
    }

    /// Like [`Self::peel_to_entry()`], but takes a `Path` directly via `relative_path`, a path relative to this tree.
    ///
    /// # Note
    ///
    /// If any path component contains illformed UTF-8 and thus can't be converted to bytes on platforms which can't do so natively,
    /// the returned component will be empty which makes the lookup fail.
    pub fn peel_to_entry_by_path(
        &mut self,
        relative_path: impl AsRef<std::path::Path>,
    ) -> Result<Option<Entry<'repo>>, find::existing::Error> {
        self.peel_to_entry(
            relative_path
                .as_ref()
                .components()
                .map(|c| c.as_os_str().as_encoded_bytes()),
        )
    }
}

///
#[cfg(feature = "tree-editor")]
pub mod editor;

///
#[cfg(feature = "blob-diff")]
pub mod diff;

///
pub mod traverse;

///
mod iter {
    use super::{EntryRef, Tree};

    impl<'repo> Tree<'repo> {
        /// Return an iterator over tree entries to obtain information about files and directories this tree contains.
        pub fn iter(&self) -> impl Iterator<Item = Result<EntryRef<'repo, '_>, gix_object::decode::Error>> {
            let repo = self.repo;
            gix_object::TreeRefIter::from_bytes(&self.data).map(move |e| e.map(|entry| EntryRef { inner: entry, repo }))
        }
    }
}

impl std::fmt::Debug for Tree<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Tree({})", self.id)
    }
}

/// An entry within a tree
pub struct EntryRef<'repo, 'a> {
    /// The actual entry ref we are wrapping.
    pub inner: gix_object::tree::EntryRef<'a>,
    /// The owning repository.
    pub repo: &'repo Repository,
}

/// An entry in a [`Tree`], similar to an entry in a directory.
#[derive(PartialEq, Debug, Clone)]
pub struct Entry<'repo> {
    pub(crate) inner: gix_object::tree::Entry,
    /// The owning repository.
    pub repo: &'repo crate::Repository,
}

mod entry;

mod _impls {
    use crate::Tree;

    impl TryFrom<Tree<'_>> for gix_object::Tree {
        type Error = gix_object::decode::Error;

        fn try_from(t: Tree<'_>) -> Result<Self, Self::Error> {
            t.decode().map(Into::into)
        }
    }
}

/// Remove Lifetime
impl Tree<'_> {
    /// Create an owned instance of this object, copying our data in the process.
    pub fn detached(&self) -> ObjectDetached {
        ObjectDetached {
            id: self.id,
            kind: gix_object::Kind::Tree,
            data: self.data.clone(),
        }
    }

    /// Sever the connection to the `Repository` and turn this instance into a standalone object.
    pub fn detach(self) -> ObjectDetached {
        self.into()
    }

    /// Retrieve this instance's encoded data, leaving its own data empty.
    ///
    /// This method works around the immovability of members of this type.
    pub fn take_data(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.data)
    }
}

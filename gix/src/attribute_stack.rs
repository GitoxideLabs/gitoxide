use std::ops::{Deref, DerefMut};

use gix_fs::stack::ToNormalPathComponents;

use crate::{Repository, types::AttributeStack};

/// Lifecycle
impl<'repo> AttributeStack<'repo> {
    /// Create a new instance from a `repo` and the underlying pre-configured `stack`.
    ///
    /// Note that this type is typically created by [`Repository::attributes()`] or [`Repository::attributes_only()`].
    pub fn new(stack: gix_worktree::Stack, repo: &'repo Repository) -> Self {
        AttributeStack { repo, inner: stack }
    }

    /// Detach the repository and return the underlying plumbing datatype.
    pub fn detach(self) -> gix_worktree::Stack {
        self.inner
    }
}

impl Deref for AttributeStack<'_> {
    type Target = gix_worktree::Stack;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for AttributeStack<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Platform retrieval
impl AttributeStack<'_> {
    /// Append the `relative` path to the root directory of the cache and load all attribute or ignore files on the way as needed.
    /// Use `mode` to specify what kind of item lives at `relative` - directories may match against rules specifically.
    /// If `mode` is `None`, the item at `relative` is assumed to be a file.
    ///
    /// The returned platform may be used to access the actual attribute or ignore information.
    #[doc(alias = "is_path_ignored", alias = "git2")]
    pub fn at_path(
        &mut self,
        relative: impl AsRef<std::path::Path>,
        mode: Option<gix_index::entry::Mode>,
    ) -> std::io::Result<gix_worktree::stack::Platform<'_>> {
        self.inner.at_path(relative.as_ref(), mode, &self.repo.objects)
    }

    /// Obtain a platform for attribute or ignore lookups from a repo-`relative` path, typically obtained from an index entry.
    /// `mode` should reflect whether it's a directory or not, or left at `None` if unknown.
    ///
    /// If `relative` ends with `/` and `mode` is `None`, it is automatically assumed to be a directory.
    pub fn at_entry(
        &mut self,
        relative: impl ToNormalPathComponents,
        mode: Option<gix_index::entry::Mode>,
    ) -> std::io::Result<gix_worktree::stack::Platform<'_>> {
        self.inner.at_path(relative, mode, &self.repo.objects)
    }
}

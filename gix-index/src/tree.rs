use bstr::ByteSlice;
use gix_error::{ErrorExt, Result, ResultExt, ensure, validation};
use gix_object::tree::EntryKind;

use crate::{
    State,
    entry::{Flags, Mode, Stage},
};

impl State {
    /// Write this index as trees to `objects`, returning the root tree ID without changing the index.
    /// `validate` controls path-component validation, as in [`Self::from_tree()`].
    ///
    /// Entries must be sorted, fully merged, and free of file/directory collisions. Intent-to-add and
    /// removal entries are omitted. Sparse directory entries are reused without expansion; they must
    /// have a trailing slash and the skip-worktree flag. Other index flags and cached tree data do not
    /// affect the result. Object IDs must be non-null and use this index's hash kind. Objects must exist,
    /// except for gitlinks whose commits belong to another repository.
    ///
    /// All index validation finishes before writing any trees. Object-store failures while writing
    /// may leave unreachable trees behind. Unlike `git write-tree`, this does not refresh cached tree
    /// data in the index or fetch missing objects from a promisor remote.
    pub fn write_tree<Objects>(
        &self,
        objects: &Objects,
        validate: gix_validate::path::component::Options,
    ) -> Result<gix_hash::ObjectId>
    where
        Objects: gix_object::Find + gix_object::Exists + gix_object::Write,
    {
        self.verify_entries()?;
        let mut tree = gix_object::tree::Editor::new(Default::default(), objects, self.object_hash());
        for entry in self.entries() {
            let path = entry.path(self);
            ensure!(
                entry.stage() == Stage::Unconflicted,
                validation(format!("Cannot write unmerged index entry at '{path}'"))
            );
            let kind = match entry.mode {
                Mode::FILE => EntryKind::Blob,
                Mode::FILE_EXECUTABLE => EntryKind::BlobExecutable,
                Mode::SYMLINK => EntryKind::Link,
                Mode::COMMIT => EntryKind::Commit,
                Mode::DIR => EntryKind::Tree,
                mode => return Err(validation(format!("Invalid index mode {mode:?} at '{path}'")).raise()),
            };
            let path = if kind == EntryKind::Tree {
                ensure!(
                    path.ends_with(b"/") && entry.flags.contains(Flags::SKIP_WORKTREE),
                    validation(format!("Invalid sparse directory entry at '{path}'"))
                );
                let path = path[..path.len() - 1].as_bstr();
                ensure!(
                    self.entry_by_path(path).is_none(),
                    validation(format!("Index contains both a file and sparse directory at '{path}'"))
                );
                path
            } else {
                path
            };
            let mut components = path.split(|byte| *byte == b'/').peekable();
            while let Some(component) = components.next() {
                let is_last = components.peek().is_none();
                gix_validate::path::component(
                    component.as_bstr(),
                    (is_last && kind == EntryKind::Link).then_some(gix_validate::path::component::Mode::Symlink),
                    validate,
                )
                .or_raise(|| validation(format!("Invalid index path '{path}'")))?;
            }
            for (position, _) in path.iter().enumerate().filter(|(_, byte)| **byte == b'/') {
                ensure!(
                    self.entry_by_path(path[..position].as_bstr()).is_none()
                        && self.entry_by_path(path[..=position].as_bstr()).is_none(),
                    validation(format!("Index contains a file or sparse directory above '{path}'"))
                );
            }
            ensure!(
                entry.id.kind() == self.object_hash() && !entry.id.is_null(),
                validation(format!("Invalid index object ID {} at '{path}'", entry.id))
            );
            ensure!(
                kind == EntryKind::Commit || objects.exists(&entry.id),
                validation(format!("Index object {} at '{path}' could not be found", entry.id))
            );
            if entry.flags.intersects(Flags::INTENT_TO_ADD | Flags::REMOVE) {
                continue;
            }
            tree.upsert(path.split(|byte| *byte == b'/').map(ByteSlice::as_bstr), kind, entry.id)?;
        }
        tree.write(|tree| {
            let tree_id = objects.write(tree)?;
            ensure!(
                tree_id.kind() == self.object_hash(),
                validation("Object store must write using the index's object hash")
            );
            Ok(tree_id)
        })
    }
}

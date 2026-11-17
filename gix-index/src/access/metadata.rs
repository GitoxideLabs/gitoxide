use bstr::BStr;
use gix_error::{Result, bail, message};

use crate::{
    State,
    entry::{Flags, Stage},
    extension::{Tree, UntrackedCache},
};

impl State {
    /// Reuse `source`'s metadata in a this index.
    ///
    /// Both indexes must be sorted; their entries are compared in one linear pass. Stage-zero entries
    /// with matching paths, modes, and object IDs inherit stat data, assume-valid, skip-worktree, and
    /// up-to-date flags. Source entries marked for removal or intent-to-add are not reused.
    /// Racy entries have their cached size zeroed and up-to-date flag cleared, so a later index
    /// timestamp cannot hide the need to check their contents.
    /// Other entries retain their existing stat data and flags, allowing checkout to populate them later.
    ///
    /// Copy the timestamp and in-memory version, and clone `TREE` and `UNTR`, invalidating changed,
    /// added, deleted, and unmerged paths while preserving unaffected cached directories.
    /// `source` must describe the same worktree, and its caches must already match its entries.
    /// It is left untouched. No filesystem access occurs.
    /// Resolve-undo, split-index, filesystem-monitor, and decoded layout extensions are discarded.
    pub fn copy_metadata_from(&mut self, source: &State) -> Result {
        if self.object_hash != source.object_hash {
            bail!(message("Cannot reuse index metadata across different object-hash kinds").validation());
        }
        self.verify_entries()?;
        source.verify_entries()?;
        source.verify_extensions(false, gix_object::find::Never)?;

        let mut tree = source.tree.clone();
        let mut untracked = source.untracked.clone();
        let mut old_entries = source.entries.iter().peekable();
        for entry in &mut self.entries {
            let path = entry.path_in(&self.path_backing);
            entry.flags.remove(Flags::FSMONITOR_VALID);
            while let Some(old) = old_entries.next_if(|old| old.path(source) < path) {
                invalidate(old.path(source), &mut tree, &mut untracked);
            }
            let old = old_entries.next_if(|old| old.path(source) == path);
            if let Some(old) = old
                && entry.stage() == Stage::Unconflicted
                && old.stage() == Stage::Unconflicted
                && entry.mode == old.mode
                && entry.id == old.id
                && !old.flags.intersects(Flags::REMOVE | Flags::INTENT_TO_ADD)
                && !entry.flags.intersects(Flags::REMOVE | Flags::INTENT_TO_ADD)
            {
                entry.stat = old.stat;
                entry.flags |= old.flags & (Flags::ASSUME_VALID | Flags::SKIP_WORKTREE | Flags::UPTODATE);
                if entry.flags.contains(Flags::SKIP_WORKTREE) {
                    entry.flags.insert(Flags::EXTENDED);
                }
                // Conservatively smudge racy entries before a newly written index advances its timestamp.
                if entry.stat.is_racy(source.timestamp, Default::default()) {
                    entry.stat.size = 0;
                    entry.flags.remove(Flags::UPTODATE);
                }
            } else {
                invalidate(path, &mut tree, &mut untracked);
            }
        }
        for old in old_entries {
            invalidate(old.path(source), &mut tree, &mut untracked);
        }

        self.timestamp = source.timestamp;
        self.version = source.version;
        self.tree = tree;
        self.untracked = untracked;
        self.link = None;
        self.resolve_undo = None;
        self.fs_monitor = None;
        self.offset_table_at_decode_time = false;
        self.end_of_index_at_decode_time = false;
        Ok(())
    }
}

fn invalidate(path: &BStr, tree: &mut Option<Tree>, untracked: &mut Option<UntrackedCache>) {
    if let Some(tree) = tree {
        tree.invalidate_path(path);
    }
    if let Some(untracked) = untracked {
        untracked.invalidate_path(path);
    }
}

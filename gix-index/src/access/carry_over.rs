use bstr::{BStr, ByteSlice};
use gix_error::{OptionExt, Result, bail};

use crate::{
    State,
    entry::Flags,
    extension::{FsMonitor, UntrackedCache, fs_monitor::entry_is_eligible},
};

impl State {
    /// Carry the filesystem monitor checkpoint and untracked cache from `previous` into this rebuilt index.
    ///
    /// Both indices must refer to the same worktree and have sorted entries. Monitor validity is retained only
    /// for eligible entries whose path, stage, object id, mode, stat and assume-valid/skip-worktree flags match.
    /// Existing validity flags are replaced; stat information is never copied. The checkpoint is not advanced,
    /// so a monitor query is still required before using these flags to skip checking the worktree.
    ///
    /// Cached untracked listings are invalidated for changed index membership and modes. Changes to a tracked
    /// ignore file invalidate its directory and descendants, as the index may supply fallback ignore rules.
    /// Absent source extensions remove their destination counterparts. An invalid source bitmap or incompatible
    /// object hash causes an error without changing this index.
    ///
    /// This is explicit: writing an index never reads another index or automatically restores these extensions.
    ///
    /// ```no_run
    /// # fn rewrite(mut rebuilt: gix_index::File, previous: &gix_index::State) -> gix_error::Result<()> {
    /// rebuilt.carry_over_fsmonitor_data_from(previous)?;
    /// rebuilt.write(Default::default())?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn carry_over_fsmonitor_data_from(&mut self, previous: &State) -> Result<()> {
        if self.object_hash != previous.object_hash {
            bail!(gix_error::validation(
                "Cannot carry index caches between different object hash kinds"
            ));
        }
        let previous_valid = previous
            .fs_monitor
            .as_ref()
            .map(|monitor| {
                monitor
                    .valid_entries(previous.entries.len())
                    .ok_or_raise(|| gix_error::corruption("Cannot carry an invalid filesystem monitor bitmap"))
            })
            .transpose()?;
        let mut untracked = previous.untracked.clone();
        let mut previous_entries = previous
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| !entry.flags.contains(Flags::REMOVE))
            .peekable();
        for entry in &mut self.entries {
            entry.flags.remove(Flags::FSMONITOR_VALID);
            if entry.flags.contains(Flags::REMOVE) {
                continue;
            }
            let path = entry.path_in(&self.path_backing);
            while previous_entries.peek().is_some_and(|(_, old)| {
                old.path(previous)
                    .cmp(path)
                    .then_with(|| old.stage().cmp(&entry.stage()))
                    .is_lt()
            }) {
                let (_, old) = previous_entries.next().expect("peeked entry exists");
                invalidate(&mut untracked, old.path(previous));
            }
            let matching = previous_entries
                .peek()
                .is_some_and(|(_, old)| old.path(previous) == path && old.stage() == entry.stage());
            if matching {
                let (index, old) = previous_entries.next().expect("matching entry exists");
                let same_data = old.id == entry.id && old.mode == entry.mode && old.stat == entry.stat;
                let checking_flags = Flags::ASSUME_VALID | Flags::SKIP_WORKTREE;
                entry.flags.set(
                    Flags::FSMONITOR_VALID,
                    same_data
                        && (old.flags & checking_flags) == (entry.flags & checking_flags)
                        && entry_is_eligible(old)
                        && entry_is_eligible(entry)
                        && previous_valid.as_ref().is_some_and(|valid| valid[index]),
                );
                let membership_flags = Flags::INTENT_TO_ADD | Flags::SKIP_WORKTREE;
                if old.mode != entry.mode
                    || (old.flags & membership_flags) != (entry.flags & membership_flags)
                    || (!same_data
                        && untracked
                            .as_ref()
                            .is_some_and(|cache| exclude_parent(cache, path).is_some()))
                {
                    invalidate(&mut untracked, path);
                }
            } else {
                invalidate(&mut untracked, path);
            }
        }
        for (_, old) in previous_entries {
            invalidate(&mut untracked, old.path(previous));
        }
        self.fs_monitor = previous
            .fs_monitor
            .as_ref()
            .map(|monitor| FsMonitor::from_entries(monitor.token().clone(), &self.entries));
        self.untracked = untracked;
        Ok(())
    }
}

fn invalidate(cache: &mut Option<UntrackedCache>, path: &BStr) {
    if let Some(cache) = cache {
        cache.invalidate_path(exclude_parent(cache, path).unwrap_or(path));
    }
}

fn exclude_parent<'a>(cache: &UntrackedCache, path: &'a BStr) -> Option<&'a BStr> {
    let start = path.rfind_byte(b'/').map_or(0, |pos| pos + 1);
    (path[start..] == *cache.exclude_filename_per_dir()).then(|| path[..start.saturating_sub(1)].as_bstr())
}

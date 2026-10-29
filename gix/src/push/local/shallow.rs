use std::{collections::VecDeque, io::Write, path::Path, sync::atomic::AtomicBool};

use gix_object::{Find, Kind};

use crate::{
    ObjectId, Repository, Result,
    error::{OptionExt, ResultExt, message},
    push::{self, Rejection, Status, UpdateOutcome},
};

/// Receiver boundaries needed by each command, separate from the sender's grafts.
/// A sender graft does not make the receiver shallow when its parent history is
/// already available there. Existing receiver grafts always remain in force.
pub(super) struct Shallow {
    existing: Vec<ObjectId>,
    additions: Vec<Vec<ObjectId>>,
    alternate: Option<tempfile::NamedTempFile>,
}

impl Shallow {
    pub fn prepare(
        destination: &Repository,
        incoming: &gix_odb::loose::Store,
        source: &[ObjectId],
        updates: &mut [UpdateOutcome],
        allow_updates: bool,
        interrupt: &AtomicBool,
    ) -> Result<Self> {
        let existing: Vec<_> = gix_shallow::read(&destination.shallow_file()?)?
            .map(Vec::from)
            .unwrap_or_default();
        let objects = Incoming { destination, incoming };
        let mut additions = Vec::with_capacity(updates.len());
        let mut needed = gix_hashtable::HashMap::default();
        for update in updates {
            let mut extra = Vec::new();
            if matches!(update.status, Status::WouldUpdate { .. })
                && let Some(tip_id) = update.update.new_id
            {
                // Walk with both sets of grafts first, so omitted sender parents
                // do not cause lookup failures before they can be classified.
                for entry in gix_traverse::object::BreadthFirst::new([tip_id], &objects)
                    .with_shallow_commits(existing.iter().chain(source).copied())
                {
                    push::plan::check_interrupt(interrupt)?;
                    let entry = entry?;
                    if entry.kind == Kind::Commit && source.contains(&entry.id) && !existing.contains(&entry.id) {
                        let required = match needed.get(&entry.id) {
                            Some(required) => *required,
                            None => {
                                let required = !ancestry_complete(&objects, entry.id, &existing, interrupt)?;
                                needed.insert(entry.id, required);
                                required
                            }
                        };
                        if required {
                            extra.push(entry.id);
                        }
                    }
                }
                if !extra.is_empty() && !allow_updates {
                    update.status = Status::Rejected(Rejection::Server("shallow update not allowed".into()));
                } else {
                    check_connected(&objects, tip_id, existing.iter().chain(&extra).copied(), interrupt)?;
                }
            }
            additions.push(extra);
        }
        // Hooks must see incoming histories under their temporary grafts, while
        // the real shallow file changes only for commands that pass the hooks.
        let alternate = if allow_updates && additions.iter().any(|ids| !ids.is_empty()) {
            let mut file = tempfile::Builder::new()
                .prefix("shallow-incoming-")
                .tempfile_in(destination.common_dir())
                .or_error()?;
            let mut boundaries: Vec<_> = existing.iter().chain(additions.iter().flatten()).copied().collect();
            boundaries.sort_unstable();
            boundaries.dedup();
            for commit_id in boundaries {
                writeln!(file, "{commit_id}").or_error()?;
            }
            Some(file)
        } else {
            None
        };
        Ok(Self {
            existing,
            additions,
            alternate,
        })
    }

    pub fn alternate(&self) -> Option<&Path> {
        self.alternate.as_ref().map(tempfile::NamedTempFile::path)
    }

    pub fn boundaries(&self, index: usize) -> Vec<ObjectId> {
        self.existing.iter().chain(&self.additions[index]).copied().collect()
    }

    pub fn commit(
        &self,
        destination: &Repository,
        indices: &[usize],
        updates: &[UpdateOutcome],
        interrupt: &AtomicBool,
    ) -> Result<()> {
        if !indices.iter().any(|&index| !self.additions[index].is_empty()) {
            return Ok(());
        }
        push::plan::check_interrupt(interrupt)?;
        let path = destination.shallow_file()?;
        let lock = gix_lock::File::acquire_to_update_resource(&path, gix_lock::acquire::Fail::Immediately, None, 0)?;
        // Refresh under the lock: a concurrent fetch may have deepened the
        // receiver since preparation. Never restore its stale old boundaries.
        let current = gix_shallow::read(&path)?;
        let existing: Vec<_> = current
            .as_ref()
            .map(|ids| ids.iter().copied().collect())
            .unwrap_or_default();
        let mut extra = Vec::new();
        for &index in indices {
            for &commit_id in &self.additions[index] {
                if !existing.contains(&commit_id)
                    && !extra.contains(&commit_id)
                    && !ancestry_complete(destination, commit_id, &existing, interrupt)?
                {
                    extra.push(commit_id);
                }
            }
            if let Some(tip_id) = updates[index].update.new_id {
                check_connected(destination, tip_id, existing.iter().chain(&extra).copied(), interrupt)?;
            }
        }
        if !extra.is_empty() {
            let updates: Vec<_> = extra.into_iter().map(gix_shallow::Update::Shallow).collect();
            gix_shallow::write(lock, current, &updates)?;
        }
        Ok(())
    }
}

fn ancestry_complete(
    objects: &impl Find,
    tip_id: ObjectId,
    boundaries: &[ObjectId],
    interrupt: &AtomicBool,
) -> Result<bool> {
    let mut pending = VecDeque::from([tip_id]);
    let mut seen = gix_hashtable::HashSet::default();
    let mut buffer = Vec::new();
    while let Some(commit_id) = pending.pop_front() {
        push::plan::check_interrupt(interrupt)?;
        if !seen.insert(commit_id) || boundaries.contains(&commit_id) {
            continue;
        }
        let Some(object) = objects.try_find(&commit_id, &mut buffer)? else {
            return Ok(false);
        };
        pending.extend(
            object
                .decode()?
                .into_commit()
                .ok_or_raise(|| message("A commit parent has the wrong object kind").corrupted())?
                .parents(),
        );
    }
    Ok(true)
}

fn check_connected(
    objects: &impl Find,
    tip_id: ObjectId,
    boundaries: impl IntoIterator<Item = ObjectId>,
    interrupt: &AtomicBool,
) -> Result<()> {
    for entry in gix_traverse::object::BreadthFirst::new([tip_id], objects).with_shallow_commits(boundaries) {
        push::plan::check_interrupt(interrupt)?;
        entry?;
    }
    Ok(())
}

struct Incoming<'a> {
    destination: &'a Repository,
    incoming: &'a gix_odb::loose::Store,
}

impl Find for Incoming<'_> {
    fn try_find<'a>(&self, id: &gix_hash::oid, buffer: &'a mut Vec<u8>) -> Result<Option<gix_object::Data<'a>>> {
        if self.incoming.contains(id) {
            self.incoming.try_find(id, buffer)
        } else {
            self.destination.try_find(id, buffer)
        }
    }
}

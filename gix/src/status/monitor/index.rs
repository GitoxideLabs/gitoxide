use std::{cmp::Ordering, sync::atomic::AtomicBool};

use gix_error::{Result, ResultExt, bail, message};
use gix_index::entry::{Flags, Mode, Stat, stat};

use crate::bstr::{BStr, BString, ByteSlice};

/// An exact index baseline without per-entry allocations, path offsets, or an open repository.
pub(super) struct Snapshot {
    data: Vec<u8>,
    object_hash: gix_hash::Kind,
}

pub(super) struct Change<'a> {
    pub path: &'a BStr,
    pub staged: bool,
    pub previous_mode: Option<Mode>,
    pub current_mode: Option<Mode>,
}

impl Snapshot {
    pub fn new(index: &gix_index::State, stat_options: stat::Options, interrupt: &AtomicBool) -> Result<Self> {
        super::check_interrupted(interrupt)?;
        let mut data = Vec::new();
        let mut previous = BStr::new(b"");
        for (position, entry) in index.entries().iter().enumerate() {
            super::check_interrupted(interrupt)?;
            validate_entry(index, position)?;
            let path = entry.path(index);
            let common = previous.iter().zip(path.iter()).take_while(|(a, b)| a == b).count();
            let common = u32::try_from(common)
                .or_raise(|| message("index path prefix is too long for a compact status snapshot"))?;
            data.extend_from_slice(&common.to_le_bytes());
            data.extend_from_slice(&path[common as usize..]);
            data.push(0);
            data.extend_from_slice(entry.id.as_slice());
            for value in [
                entry.mode.bits(),
                status_flags(entry.flags).bits(),
                entry.stat.mtime.secs,
                entry.stat.mtime.nsecs,
                entry.stat.ctime.secs,
                entry.stat.ctime.nsecs,
                entry.stat.dev,
                entry.stat.ino,
                entry.stat.uid,
                entry.stat.gid,
                entry.stat.size,
            ] {
                data.extend_from_slice(&value.to_le_bytes());
            }
            // Configuration changes replace the baseline, so retain the racy result instead of the timestamp.
            data.push(u8::from(entry.stat.is_racy(index.timestamp(), stat_options)));
            previous = path;
        }
        super::check_interrupted(interrupt)?;
        data.shrink_to_fit();
        Ok(Self {
            data,
            object_hash: index.object_hash(),
        })
    }

    pub fn changes(
        &self,
        index: &gix_index::State,
        stat_options: stat::Options,
        interrupt: &AtomicBool,
        mut callback: impl FnMut(Change<'_>),
    ) -> Result {
        super::check_interrupted(interrupt)?;
        let mut cursor = Cursor {
            data: &self.data,
            hash_len: self.object_hash.len_in_bytes(),
        };
        let mut path = BString::default();
        let mut previous = cursor.next(&mut path);
        let mut position = 0;
        loop {
            super::check_interrupted(interrupt)?;
            let current = index.entries().get(position);
            if current.is_some() {
                validate_entry(index, position)?;
            }
            let ordering = match (&previous, current) {
                (Some(previous), Some(current)) => path
                    .as_bstr()
                    .cmp(current.path(index))
                    .then_with(|| previous.flags.stage().cmp(&current.stage())),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => break,
            };
            match ordering {
                Ordering::Less => {
                    callback(Change {
                        path: path.as_bstr(),
                        staged: true,
                        previous_mode: previous.as_ref().map(|entry| entry.mode),
                        current_mode: None,
                    });
                    previous = cursor.next(&mut path);
                }
                Ordering::Greater => {
                    let current = current.expect("ordering requires a current index entry");
                    callback(Change {
                        path: current.path(index),
                        staged: true,
                        previous_mode: None,
                        current_mode: Some(current.mode),
                    });
                    position += 1;
                }
                Ordering::Equal => {
                    let current = current.expect("equal ordering requires a current index entry");
                    let old = previous
                        .as_ref()
                        .expect("equal ordering requires a previous index entry");
                    let staged = old.id != current.id.as_slice()
                        || old.mode != current.mode
                        || old.flags != status_flags(current.flags);
                    if staged
                        || old.stat != current.stat
                        || old.racy != current.stat.is_racy(index.timestamp(), stat_options)
                    {
                        callback(Change {
                            path: current.path(index),
                            staged,
                            previous_mode: Some(old.mode),
                            current_mode: Some(current.mode),
                        });
                    }
                    previous = cursor.next(&mut path);
                    position += 1;
                }
            }
        }
        Ok(())
    }
}

fn validate_entry(index: &gix_index::State, position: usize) -> Result {
    let entry = &index.entries()[position];
    let path = entry.path(index);
    if path.contains(&0) {
        bail!(message(
            "index paths must not contain NUL bytes for a compact status snapshot"
        ));
    }
    if entry.id.kind() != index.object_hash() {
        bail!(message(
            "index entry hash kind differs from its status snapshot's hash kind"
        ));
    }
    if let Some(previous) = position.checked_sub(1).map(|position| &index.entries()[position])
        && !previous.cmp(entry, index).is_lt()
    {
        bail!(message(
            "index entries must be strictly sorted by path and stage for a compact status snapshot"
        ));
    }
    Ok(())
}

fn status_flags(flags: Flags) -> Flags {
    flags
        & (Flags::STAGE_MASK
            | Flags::ASSUME_VALID
            | Flags::SKIP_WORKTREE
            | Flags::UPTODATE
            | Flags::FSMONITOR_VALID
            | Flags::INTENT_TO_ADD)
}

struct Record<'a> {
    id: &'a [u8],
    mode: Mode,
    flags: Flags,
    stat: Stat,
    racy: bool,
}

struct Cursor<'a> {
    data: &'a [u8],
    hash_len: usize,
}

impl<'a> Cursor<'a> {
    fn next(&mut self, path: &mut BString) -> Option<Record<'a>> {
        if self.data.is_empty() {
            return None;
        }
        let common = self.u32() as usize;
        path.truncate(common);
        let suffix_len = self
            .data
            .iter()
            .position(|byte| *byte == 0)
            .expect("snapshot paths were serialized with a NUL terminator");
        path.extend_from_slice(self.take(suffix_len));
        self.take(1);
        let id = self.take(self.hash_len);
        let mode = Mode::from_bits_retain(self.u32());
        let flags = Flags::from_bits_retain(self.u32());
        let stat = Stat {
            mtime: stat::Time {
                secs: self.u32(),
                nsecs: self.u32(),
            },
            ctime: stat::Time {
                secs: self.u32(),
                nsecs: self.u32(),
            },
            dev: self.u32(),
            ino: self.u32(),
            uid: self.u32(),
            gid: self.u32(),
            size: self.u32(),
        };
        let racy = self.take(1)[0] != 0;
        Some(Record {
            id,
            mode,
            flags,
            stat,
            racy,
        })
    }

    fn take(&mut self, length: usize) -> &'a [u8] {
        let (data, rest) = self
            .data
            .split_at_checked(length)
            .expect("snapshot records were serialized with their complete fixed-width fields");
        self.data = rest;
        data
    }

    fn u32(&mut self) -> u32 {
        u32::from_le_bytes(self.take(4).try_into().expect("four bytes encode one u32"))
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use gix_error::TestResult;

    use super::*;

    fn state(paths: &[&[u8]]) -> gix_index::State {
        let mut index = gix_index::State::new(gix_hash::Kind::Sha1);
        index.set_timestamp((UNIX_EPOCH + Duration::from_secs(10)).into());
        for path in paths {
            index.dangerously_push_entry(
                Stat::default(),
                gix_hash::ObjectId::null(gix_hash::Kind::Sha1),
                Flags::empty(),
                Mode::FILE,
                path.as_bstr(),
            );
        }
        index.sort_entries();
        index
    }

    type Changed = (BString, bool, Option<Mode>, Option<Mode>);

    fn changes(previous: &gix_index::State, current: &gix_index::State) -> TestResult<Vec<Changed>> {
        let interrupt = AtomicBool::new(false);
        let mut out = Vec::new();
        Snapshot::new(previous, stat::Options::default(), &interrupt)?.changes(
            current,
            stat::Options::default(),
            &interrupt,
            |change| {
                out.push((
                    change.path.to_owned(),
                    change.staged,
                    change.previous_mode,
                    change.current_mode,
                ));
            },
        )?;
        Ok(out)
    }

    #[test]
    fn additions_deletions_and_content_changes_are_exact() -> TestResult {
        let previous = state(&[b"a", b"c", b"d"]);
        let mut current = state(&[b"b", b"c", b"d"]);
        current.entries_mut()[1].id = gix_hash::ObjectId::try_from(&[1; 20][..])?;
        assert_eq!(
            changes(&previous, &current)?,
            [
                ("a".into(), true, Some(Mode::FILE), None),
                ("b".into(), true, None, Some(Mode::FILE)),
                ("c".into(), true, Some(Mode::FILE), Some(Mode::FILE)),
            ],
            "unchanged entries are omitted while both sides of a replacement are retained"
        );
        Ok(())
    }

    #[test]
    fn conflict_stages_are_not_discarded() -> TestResult {
        let mut previous = state(&[b"conflict", b"conflict", b"conflict"]);
        for (entry, stage) in previous.entries_mut().iter_mut().zip([
            gix_index::entry::Stage::Base,
            gix_index::entry::Stage::Ours,
            gix_index::entry::Stage::Theirs,
        ]) {
            entry.flags = Flags::from_stage(stage);
        }
        let current = state(&[b"conflict"]);
        let actual = changes(&previous, &current)?;
        assert_eq!(
            actual.len(),
            4,
            "all three conflict removals and the resolved entry are reported"
        );
        assert!(
            actual.iter().all(|(path, staged, _, _)| path == "conflict" && *staged),
            "every conflict-stage transition invalidates the same staged path"
        );
        Ok(())
    }

    #[test]
    fn modes_and_flags_invalidate_status() -> TestResult {
        let previous = state(&[b"tracked"]);
        for flag in [
            Flags::ASSUME_VALID,
            Flags::SKIP_WORKTREE,
            Flags::INTENT_TO_ADD,
            Flags::FSMONITOR_VALID,
            Flags::UPTODATE,
        ] {
            let mut current = previous.clone();
            current.entries_mut()[0].flags = flag;
            assert_eq!(
                changes(&previous, &current)?,
                [("tracked".into(), true, Some(Mode::FILE), Some(Mode::FILE))],
                "{flag:?} changes status eligibility even when object IDs match"
            );
        }
        let mut current = previous.clone();
        current.entries_mut()[0].mode = Mode::COMMIT;
        assert_eq!(
            changes(&previous, &current)?,
            [("tracked".into(), true, Some(Mode::FILE), Some(Mode::COMMIT))],
            "the caller can recognize transitions into submodule topology"
        );
        Ok(())
    }

    #[test]
    fn storage_flags_do_not_invalidate_status_or_change_the_baseline() -> TestResult {
        let previous = state(&[b"tracked"]);
        let mut current = previous.clone();
        current.entries_mut()[0].flags = Flags::from_bits_retain(7) | Flags::EXTENDED;
        assert!(
            changes(&previous, &current)?.is_empty(),
            "path-length and extended-storage markers do not affect status"
        );
        let interrupt = AtomicBool::new(false);
        assert_eq!(
            Snapshot::new(&previous, stat::Options::default(), &interrupt)?.data,
            Snapshot::new(&current, stat::Options::default(), &interrupt)?.data,
            "representation bookkeeping is excluded from the retained semantic baseline"
        );
        Ok(())
    }

    #[test]
    fn every_stat_field_is_preserved_without_staged_invalidation() -> TestResult {
        let previous = state(&[b"tracked"]);
        for field in 0..9 {
            let mut current = previous.clone();
            let stat = &mut current.entries_mut()[0].stat;
            match field {
                0 => stat.mtime.secs = 1,
                1 => stat.mtime.nsecs = 1,
                2 => stat.ctime.secs = 1,
                3 => stat.ctime.nsecs = 1,
                4 => stat.dev = 1,
                5 => stat.ino = 1,
                6 => stat.uid = 1,
                7 => stat.gid = 1,
                8 => stat.size = 1,
                _ => unreachable!("the loop covers the nine stat fields"),
            }
            assert_eq!(
                changes(&previous, &current)?,
                [("tracked".into(), false, Some(Mode::FILE), Some(Mode::FILE))],
                "stat field {field} refreshes worktree metadata without changing staged content"
            );
        }
        Ok(())
    }

    #[test]
    fn timestamp_changes_refresh_only_changed_racy_classifications() -> TestResult {
        let mut previous = state(&[b"stable", b"racy"]);
        previous.entries_mut()[0].stat.mtime.secs = 5;
        let mut current = previous.clone();
        current.set_timestamp((UNIX_EPOCH + Duration::from_secs(5)).into());
        assert_eq!(
            changes(&previous, &current)?,
            [("racy".into(), false, Some(Mode::FILE), Some(Mode::FILE))],
            "an unchanged entry still needs checking when the index timestamp makes it racy"
        );
        assert_eq!(
            changes(&current, &previous)?,
            [("racy".into(), false, Some(Mode::FILE), Some(Mode::FILE))],
            "leaving the racy window also refreshes the path"
        );
        Ok(())
    }

    #[test]
    fn paths_preserve_bytes_and_share_long_prefixes() -> TestResult {
        let mut first = vec![0xff; 65_536];
        first.extend_from_slice(b"/first");
        let mut second = vec![0xff; 65_536];
        second.extend_from_slice(b"/second");
        let previous = state(&[&first, &second]);
        let interrupt = AtomicBool::new(false);
        let snapshot = Snapshot::new(&previous, stat::Options::default(), &interrupt)?;
        assert!(
            snapshot.data.len() < 65_536 + 160,
            "shared byte prefixes are stored once, with 70 metadata bytes per SHA-1 entry"
        );
        assert!(
            changes(&previous, &previous)?.is_empty(),
            "byte paths survive exact roundtrips"
        );
        let mut current = previous.clone();
        current.entries_mut()[1].stat.size = 1;
        assert_eq!(
            changes(&previous, &current)?,
            [(second.into(), false, Some(Mode::FILE), Some(Mode::FILE))],
            "paths beyond u16-length prefixes are reconstructed without Unicode conversion"
        );
        Ok(())
    }

    #[test]
    fn representative_paths_retain_less_than_two_thirds_of_a_native_index() -> TestResult {
        let paths: Vec<_> = (0..10_000)
            .map(|position| {
                format!(
                    "crates/component_{:03}/src/module_{:03}/file_{:04}.rs",
                    position / 1000,
                    position / 100 % 10,
                    position % 100
                )
            })
            .collect();
        let index = state(&paths.iter().map(String::as_bytes).collect::<Vec<_>>());
        let snapshot = Snapshot::new(&index, stat::Options::default(), &AtomicBool::new(false))?;
        let retained = snapshot.data.capacity();
        let native = std::mem::size_of_val(index.entries()) + index.path_backing().len();
        eprintln!(
            "status index baseline: {retained} retained bytes for {} paths, versus {native} native index bytes ({:.1}% reduction)",
            index.entries().len(),
            100.0 * (1.0 - retained as f64 / native as f64)
        );
        assert!(
            retained * 3 < native * 2,
            "realistic directory prefixes reduce allocated baseline memory by more than one third"
        );
        Ok(())
    }

    #[test]
    fn invalid_input_and_cancellation_do_not_produce_a_baseline() -> TestResult {
        let mut index = state(&[b"a", b"b"]);
        let interrupt = AtomicBool::new(true);
        assert!(
            Snapshot::new(&index, stat::Options::default(), &interrupt).is_err(),
            "interrupted setup returns no partial baseline"
        );
        interrupt.store(false, std::sync::atomic::Ordering::Relaxed);
        index.entries_mut().swap(0, 1);
        assert!(
            Snapshot::new(&index, stat::Options::default(), &interrupt).is_err(),
            "an unsorted index cannot silently hide additions or removals"
        );
        assert!(
            Snapshot::new(&state(&[b"invalid\0path"]), stat::Options::default(), &interrupt).is_err(),
            "NUL cannot be confused with the private path terminator"
        );
        let snapshot = Snapshot::new(&state(&[]), stat::Options::default(), &interrupt)?;
        let duplicate = state(&[b"duplicate", b"duplicate"]);
        assert!(
            Snapshot::new(&duplicate, stat::Options::default(), &interrupt).is_err(),
            "a baseline cannot contain duplicate path-and-stage identities"
        );
        assert!(
            snapshot
                .changes(&duplicate, stat::Options::default(), &interrupt, |_| {})
                .is_err(),
            "incoming index entries must also have unique path-and-stage identities"
        );
        let mut visited = 0;
        assert!(
            snapshot
                .changes(&state(&[b"a", b"b"]), stat::Options::default(), &interrupt, |_| {
                    visited += 1;
                    interrupt.store(true, std::sync::atomic::Ordering::Relaxed);
                })
                .is_err(),
            "cancellation during comparison stops further callbacks"
        );
        assert_eq!(
            visited, 1,
            "the callback is checked for interruption before the next path"
        );
        Ok(())
    }

    #[cfg(feature = "sha256")]
    #[test]
    fn sha256_preserves_the_entire_object_id() -> TestResult {
        let mut previous = gix_index::State::new(gix_hash::Kind::Sha256);
        previous.dangerously_push_entry(
            Stat::default(),
            gix_hash::ObjectId::null(gix_hash::Kind::Sha256),
            Flags::empty(),
            Mode::FILE,
            b"tracked".as_bstr(),
        );
        let mut current = previous.clone();
        let mut bytes = [0; 32];
        bytes[31] = 1;
        current.entries_mut()[0].id = gix_hash::ObjectId::try_from(&bytes[..])?;
        assert_eq!(
            changes(&previous, &current)?,
            [("tracked".into(), true, Some(Mode::FILE), Some(Mode::FILE))],
            "changes beyond the SHA-1 prefix cannot be lost"
        );
        Ok(())
    }
}

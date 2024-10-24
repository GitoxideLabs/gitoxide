use std::{
    io,
    path::Path,
    slice::Chunks,
    sync::atomic::{AtomicUsize, Ordering},
};

use bstr::BStr;
use filetime::FileTime;
use gix_features::parallel::{in_parallel_if, Reduce};
use gix_filter::pipeline::convert::ToGitOutcome;
use gix_object::FindExt;

use crate::index_as_worktree::Context;
use crate::{
    index_as_worktree::{
        traits,
        traits::{read_data::Stream, CompareBlobs, SubmoduleStatus},
        types::{Error, Options},
        Change, Conflict, EntryStatus, Outcome, VisitEntry,
    },
    is_dir_to_mode, AtomicU64, SymlinkCheck,
};

/// Calculates the changes that need to be applied to an `index` to match the state of the `worktree` and makes them
/// observable in `collector`, along with information produced by `compare` which gets to see blobs that may have changes, and
/// `submodule` which can take a look at submodules in detail to produce status information (BASE version if its conflicting).
/// `options` are used to configure the operation.
///
/// Note `worktree` must be the root path of the worktree, not a path inside of the worktree.
///
/// Note that `index` may require changes to be up-to-date with the working tree and avoid expensive computations by updating
/// respective entries with stat information from the worktree, and its timestamp is adjusted to the current time for which it
/// will be considered fresh. All changes that would be applied to the index are delegated to the caller, which receives these
/// as [`EntryStatus`].
/// The `pathspec` is used to determine which index entries to check for status in the first place.
///
/// `objects` is used to access the version of an object in the object database for direct comparison.
///
/// **It's important to note that the `index` should have its [timestamp updated](gix_index::State::set_timestamp()) with a timestamp
/// from just before making this call *if* [entries were updated](Outcome::entries_to_update)**
///
/// ### Note
///
/// Technically, this function does more as it also provides additional information, like whether a file has conflicts,
/// and files that were added with `git add` are shown as a special as well. It also provides updates to entry filesystem
/// stats like `git status` would if it had to determine the hash.
/// If that happened, the index should be written back after updating the entries with these updated stats, see [Outcome::skipped].
///
/// Thus, some care has to be taken to do the right thing when letting the index match the worktree by evaluating the changes observed
/// by the `collector`.
#[allow(clippy::too_many_arguments)]
pub fn index_as_worktree<'index, T, U, Find, E>(
    index: &'index gix_index::State,
    worktree: &Path,
    collector: &mut impl VisitEntry<'index, ContentChange = T, SubmoduleStatus = U>,
    compare: impl CompareBlobs<Output = T> + Send + Clone,
    submodule: impl SubmoduleStatus<Output = U, Error = E> + Send + Clone,
    objects: Find,
    progress: &mut dyn gix_features::progress::Progress,
    Context {
        pathspec,
        stack,
        filter,
        should_interrupt,
    }: Context<'_>,
    options: Options,
) -> Result<Outcome, Error>
where
    T: Send,
    U: Send,
    E: std::error::Error + Send + Sync + 'static,
    Find: gix_object::Find + Send + Clone,
{
    // the order is absolutely critical here we use the old timestamp to detect racy index entries
    // (modified at or after the last index update) during the index update we then set those
    // entries size to 0 (see below) to ensure they keep showing up as racy and reset the timestamp.
    let timestamp = index.timestamp();
    let (chunk_size, thread_limit, _) = gix_features::parallel::optimize_chunk_size_and_thread_limit(
        500, // just like git
        index.entries().len().into(),
        options.thread_limit,
        None,
    );

    let range = index
        .prefixed_entries_range(pathspec.common_prefix())
        .unwrap_or(0..index.entries().len());

    let (entries, path_backing) = (index.entries(), index.path_backing());
    let mut num_entries = entries.len();
    let entry_index_offset = range.start;
    let entries = &entries[range];

    let _span = gix_features::trace::detail!("gix_status::index_as_worktree",
                                             num_entries = entries.len(),
                                             chunk_size = chunk_size,
                                             thread_limit = ?thread_limit);

    let entries_skipped_by_common_prefix = num_entries - entries.len();
    let (skipped_by_pathspec, skipped_by_entry_flags, symlink_metadata_calls, entries_to_update) = Default::default();
    let (worktree_bytes, worktree_reads, odb_bytes, odb_reads, racy_clean) = Default::default();

    num_entries = entries.len();
    progress.init(entries.len().into(), gix_features::progress::count("files"));
    let count = progress.counter();

    let new_state = {
        let options = &options;
        let (skipped_by_pathspec, skipped_by_entry_flags) = (&skipped_by_pathspec, &skipped_by_entry_flags);
        let (symlink_metadata_calls, entries_to_update) = (&symlink_metadata_calls, &entries_to_update);
        let (racy_clean, worktree_bytes) = (&racy_clean, &worktree_bytes);
        let (worktree_reads, odb_bytes, odb_reads) = (&worktree_reads, &odb_bytes, &odb_reads);
        move |_| {
            (
                State {
                    buf: Vec::new(),
                    buf2: Vec::new(),
                    attr_stack: stack,
                    path_stack: SymlinkCheck::new(worktree.into()),
                    timestamp,
                    path_backing,
                    filter,
                    options,

                    skipped_by_pathspec,
                    skipped_by_entry_flags,
                    symlink_metadata_calls,
                    entries_to_update,
                    racy_clean,
                    worktree_reads,
                    worktree_bytes,
                    odb_reads,
                    odb_bytes,
                },
                compare,
                submodule,
                objects,
                pathspec,
            )
        }
    };
    in_parallel_if(
        || true, // TODO: heuristic: when is parallelization not worth it? Git says 500 items per thread, but to 20 threads, we can be more fine-grained though.
        gix_features::interrupt::Iter::new(
            OffsetIter {
                inner: entries.chunks(chunk_size),
                offset: entry_index_offset,
            },
            should_interrupt,
        ),
        thread_limit,
        new_state,
        |(entry_offset, chunk_entries), (state, blobdiff, submdule, objects, pathspec)| {
            let all_entries = index.entries();
            let mut out = Vec::new();
            let mut idx = 0;
            while let Some(entry) = chunk_entries.get(idx) {
                let absolute_entry_index = entry_offset + idx;
                if idx == 0 && entry.stage_raw() != 0 {
                    let offset = entry_offset.checked_sub(1).and_then(|prev_idx| {
                        let prev_entry = &all_entries[prev_idx];
                        let entry_path = entry.path_in(state.path_backing);
                        if prev_entry.stage_raw() == 0 || prev_entry.path_in(state.path_backing) != entry_path {
                            // prev_entry (in previous chunk) does not belong to our conflict
                            return None;
                        }
                        Conflict::try_from_entry(all_entries, state.path_backing, absolute_entry_index, entry_path)
                            .map(|(_conflict, offset)| offset)
                    });
                    if let Some(entries_to_skip_as_conflict_originates_in_previous_chunk) = offset {
                        // skip current entry as it's done, along with following conflict entries
                        idx += entries_to_skip_as_conflict_originates_in_previous_chunk + 1;
                        continue;
                    }
                }
                let res = state.process(
                    all_entries,
                    entry,
                    absolute_entry_index,
                    pathspec,
                    blobdiff,
                    submdule,
                    objects,
                    &mut idx,
                );
                idx += 1;
                count.fetch_add(1, Ordering::Relaxed);
                if let Some(res) = res {
                    out.push(res);
                }
            }
            out
        },
        ReduceChange {
            collector,
            entries: index.entries(),
        },
    )?;

    Ok(Outcome {
        entries_to_process: num_entries,
        entries_processed: count.load(Ordering::Relaxed),
        entries_skipped_by_common_prefix,
        entries_skipped_by_pathspec: skipped_by_pathspec.load(Ordering::Relaxed),
        entries_skipped_by_entry_flags: skipped_by_entry_flags.load(Ordering::Relaxed),
        entries_to_update: entries_to_update.load(Ordering::Relaxed),
        symlink_metadata_calls: symlink_metadata_calls.load(Ordering::Relaxed),
        racy_clean: racy_clean.load(Ordering::Relaxed),
        worktree_files_read: worktree_reads.load(Ordering::Relaxed),
        worktree_bytes: worktree_bytes.load(Ordering::Relaxed),
        odb_objects_read: odb_reads.load(Ordering::Relaxed),
        odb_bytes: odb_bytes.load(Ordering::Relaxed),
    })
}

struct State<'a, 'b> {
    buf: Vec<u8>,
    buf2: Vec<u8>,
    timestamp: FileTime,
    /// This is the cheap stack that only assure that we don't go through symlinks.
    /// It's always used to get the path to perform an lstat on.
    path_stack: SymlinkCheck,
    /// This is the expensive stack that will need to check for `.gitattributes` files each time
    /// it changes directory. It's only used when we know we have to read a worktree file, which in turn
    /// requires attributes to drive the filter configuration.
    attr_stack: gix_worktree::Stack,
    filter: gix_filter::Pipeline,
    path_backing: &'b gix_index::PathStorageRef,
    options: &'a Options,

    skipped_by_pathspec: &'a AtomicUsize,
    skipped_by_entry_flags: &'a AtomicUsize,
    symlink_metadata_calls: &'a AtomicUsize,
    entries_to_update: &'a AtomicUsize,
    racy_clean: &'a AtomicUsize,
    worktree_bytes: &'a AtomicU64,
    worktree_reads: &'a AtomicUsize,
    odb_bytes: &'a AtomicU64,
    odb_reads: &'a AtomicUsize,
}

type StatusResult<'index, T, U> = Result<(&'index gix_index::Entry, usize, &'index BStr, EntryStatus<T, U>), Error>;

impl<'index> State<'_, 'index> {
    #[allow(clippy::too_many_arguments)]
    fn process<T, U, Find, E>(
        &mut self,
        entries: &'index [gix_index::Entry],
        entry: &'index gix_index::Entry,
        entry_index: usize,
        pathspec: &mut gix_pathspec::Search,
        diff: &mut impl CompareBlobs<Output = T>,
        submodule: &mut impl SubmoduleStatus<Output = U, Error = E>,
        objects: &Find,
        outer_entry_index: &mut usize,
    ) -> Option<StatusResult<'index, T, U>>
    where
        E: std::error::Error + Send + Sync + 'static,
        Find: gix_object::Find,
    {
        if entry.flags.intersects(
            gix_index::entry::Flags::UPTODATE
                | gix_index::entry::Flags::SKIP_WORKTREE
                | gix_index::entry::Flags::ASSUME_VALID
                | gix_index::entry::Flags::FSMONITOR_VALID,
        ) {
            self.skipped_by_entry_flags.fetch_add(1, Ordering::Relaxed);
            return None;
        }
        let path = entry.path_in(self.path_backing);
        let is_excluded = pathspec
            .pattern_matching_relative_path(path, true, &mut |relative_path, case, is_dir, out| {
                self.attr_stack
                    .set_case(case)
                    .at_entry(relative_path, Some(is_dir_to_mode(is_dir)), objects)
                    .map_or(false, |platform| platform.matching_attributes(out))
            })
            .map_or(true, |m| m.is_excluded());

        if is_excluded {
            self.skipped_by_pathspec.fetch_add(1, Ordering::Relaxed);
            return None;
        }
        let status = if entry.stage_raw() != 0 {
            Ok(
                Conflict::try_from_entry(entries, self.path_backing, entry_index, path).map(|(conflict, offset)| {
                    *outer_entry_index += offset; // let out loop skip over entries related to the conflict
                    EntryStatus::Conflict(conflict)
                }),
            )
        } else {
            self.compute_status(entry, path, diff, submodule, objects)
        };
        match status {
            Ok(None) => None,
            Ok(Some(status)) => Some(Ok((entry, entry_index, path, status))),
            Err(err) => Some(Err(err)),
        }
    }

    /// # On how racy-git is handled here
    ///
    /// Basically the racy detection is a safety mechanism that ensures we can always just compare the stat
    /// information between index and worktree and if they match we don't need to look at the content.
    /// This usually just works but if a file updates quickly we could run into the following situation:
    ///
    /// * save file version `A` from disk into worktree (git add)
    /// * file is changed so fast that the mtime doesn't change - *we only looks at seconds by default*
    /// * file contents change but file-size stays the same, so `"foo" -> "bar"` has the same size but different content
    ///
    /// Now both `mtime` and `size`, and all other stat information, is the same but the file has actually changed.
    /// This case is called *racily clean*. *The file should show up as changed but due to a data race it doesn't.*
    /// This is the racy git problem.
    ///
    /// To solve this we do the following trick: Whenever we modify the index, which includes `git status`, we save the
    /// current timestamp before the modification starts. This timestamp fundamentally represents a checkpoint of sorts.
    /// We "promise" ourselves that after the modification finishes all entries modified before this timestamp have the
    /// racy git problem resolved.
    ///
    /// So now when we modify the index we must resolve the racy git problem somehow. To do that we only need to look at
    /// unchanged entries. Changed entries are not interesting since they are already showing up as changed anyway so there
    /// isn't really a race-condition to worry about. This also explains why removing the `return` here doesn't have an apparent effect.
    /// This entire branch here is just the optimization of "don't even look at index entries where the stat hasn't changed".
    /// If we don't have this optimization the result shouldn't change, our status implementation will just be super slow :D
    ///
    /// We calculate whether this change is `racy_clean`, so if the last `timestamp` is before or the same as the `mtime` of the entry
    /// which is what `new_stat.is_racy(..)` does in the branch, and only if we are sure that there is no race condition
    /// do we `return` early. Since we don't `return` early we just do a full content comparison below,
    /// which always yields the correct result, there is no race condition there.
    ///
    /// If a file showed up as racily clean and didn't change then we don't need to do anything. After this status check is
    /// complete and the file won't show up as racily clean anymore, since it's mtime is now before the new timestamp.
    /// However, if the file did actually change then we really ran into one of those rare race conditions in that case we,
    /// and git does the same, set the size of the file in the index to 0. This will always make the file show up as changed.
    /// This adds the need to treat all files of size 0 in the index as changed. This is not quite right of course because 0 sized files
    /// could be entirely valid and unchanged. Therefore this only applies if the oid doesn't match the oid of an empty file,
    /// which is a constant.
    ///
    /// Adapted from [here](https://github.com/GitoxideLabs/gitoxide/pull/805#discussion_r1164676777).
    fn compute_status<T, U, Find, E>(
        &mut self,
        entry: &gix_index::Entry,
        rela_path: &BStr,
        diff: &mut impl CompareBlobs<Output = T>,
        submodule: &mut impl SubmoduleStatus<Output = U, Error = E>,
        objects: &Find,
    ) -> Result<Option<EntryStatus<T, U>>, Error>
    where
        E: std::error::Error + Send + Sync + 'static,
        Find: gix_object::Find,
    {
        let worktree_path = match self.path_stack.verified_path(gix_path::from_bstr(rela_path).as_ref()) {
            Ok(path) => path,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Some(Change::Removed.into())),
            Err(err) => return Err(Error::Io(err)),
        };
        self.symlink_metadata_calls.fetch_add(1, Ordering::Relaxed);
        let metadata = match gix_index::fs::Metadata::from_path_no_follow(worktree_path) {
            Ok(metadata) if metadata.is_dir() => {
                // index entries are normally only for files/symlinks
                // if a file turned into a directory it was removed
                // the only exception here are submodules which are
                // part of the index despite being directories
                if entry.mode.is_submodule() {
                    let status = submodule
                        .status(entry, rela_path)
                        .map_err(|err| Error::SubmoduleStatus {
                            rela_path: rela_path.into(),
                            source: Box::new(err),
                        })?;
                    return Ok(status.map(|status| Change::SubmoduleModification(status).into()));
                } else {
                    return Ok(Some(Change::Removed.into()));
                }
            }
            Ok(metadata) => metadata,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Some(Change::Removed.into())),
            Err(err) => {
                return Err(err.into());
            }
        };
        if entry.flags.contains(gix_index::entry::Flags::INTENT_TO_ADD) {
            return Ok(Some(EntryStatus::IntentToAdd));
        }
        let new_stat = gix_index::entry::Stat::from_fs(&metadata)?;
        let executable_bit_changed =
            match entry
                .mode
                .change_to_match_fs(&metadata, self.options.fs.symlink, self.options.fs.executable_bit)
            {
                Some(gix_index::entry::mode::Change::Type { .. }) => return Ok(Some(Change::Type.into())),
                Some(gix_index::entry::mode::Change::ExecutableBit) => true,
                None => false,
            };

        // Here we implement racy-git. See racy-git.txt in the git documentation for a detailed documentation.
        //
        // A file is racy if:
        // 1. its `mtime` is at or after the last index timestamp and its entry stat information
        //   matches the on-disk file but the file contents are actually modified
        // 2. it's size is 0 (set after detecting a file was racy previously)
        //
        // The first case is detected below by checking the timestamp if the file is marked unmodified.
        // The second case is usually detected either because the on-disk file is not empty, hence
        // the basic stat match fails, or by checking whether the size doesn't fit the oid.
        let mut racy_clean = false;
        if !executable_bit_changed
            && new_stat.matches(&entry.stat, self.options.stat)
            // TODO: find a test for the following line or remove it. Is this more often hit with smudge/clean filters?
            && (!entry.id.is_empty_blob() || entry.stat.size == 0)
        {
            racy_clean = new_stat.is_racy(self.timestamp, self.options.stat);
            if !racy_clean {
                return Ok(None);
            } else {
                self.racy_clean.fetch_add(1, Ordering::Relaxed);
            }
        }

        self.buf.clear();
        self.buf2.clear();
        let file_size_bytes = if cfg!(windows) && metadata.is_symlink() {
            // symlinks on Windows seem to have a length of zero, so just pretend
            // they have the correct length to avoid short-cutting, and enforce a full buffer check.
            u64::from(entry.stat.size)
        } else {
            metadata.len()
        };
        let fetch_data = ReadDataImpl {
            buf: &mut self.buf,
            path: worktree_path,
            rela_path,
            entry,
            file_len: file_size_bytes,
            filter: &mut self.filter,
            attr_stack: &mut self.attr_stack,
            options: self.options,
            id: &entry.id,
            objects,
            worktree_reads: self.worktree_reads,
            worktree_bytes: self.worktree_bytes,
            odb_reads: self.odb_reads,
            odb_bytes: self.odb_bytes,
        };
        let content_change = diff.compare_blobs(entry, file_size_bytes, fetch_data, &mut self.buf2)?;
        // This file is racy clean! Set the size to 0 so we keep detecting this as the file is updated.
        if content_change.is_some() || executable_bit_changed {
            let set_entry_stat_size_zero = content_change.is_some() && racy_clean;
            Ok(Some(
                Change::Modification {
                    executable_bit_changed,
                    content_change,
                    set_entry_stat_size_zero,
                }
                .into(),
            ))
        } else {
            self.entries_to_update.fetch_add(1, Ordering::Relaxed);
            Ok(Some(EntryStatus::NeedsUpdate(new_stat)))
        }
    }
}

struct ReduceChange<'a, 'index, T: VisitEntry<'index>> {
    collector: &'a mut T,
    entries: &'index [gix_index::Entry],
}

impl<'index, T, U, C: VisitEntry<'index, ContentChange = T, SubmoduleStatus = U>> Reduce
    for ReduceChange<'_, 'index, C>
{
    type Input = Vec<StatusResult<'index, T, U>>;

    type FeedProduce = ();

    type Output = ();

    type Error = Error;

    fn feed(&mut self, items: Self::Input) -> Result<Self::FeedProduce, Self::Error> {
        for item in items {
            let (entry, entry_index, path, status) = item?;
            self.collector
                .visit_entry(self.entries, entry, entry_index, path, status);
        }
        Ok(())
    }

    fn finalize(self) -> Result<Self::Output, Self::Error> {
        Ok(())
    }
}

struct ReadDataImpl<'a, Find>
where
    Find: gix_object::Find,
{
    buf: &'a mut Vec<u8>,
    path: &'a Path,
    rela_path: &'a BStr,
    file_len: u64,
    entry: &'a gix_index::Entry,
    filter: &'a mut gix_filter::Pipeline,
    attr_stack: &'a mut gix_worktree::Stack,
    options: &'a Options,
    id: &'a gix_hash::oid,
    objects: Find,
    worktree_bytes: &'a AtomicU64,
    worktree_reads: &'a AtomicUsize,
    odb_bytes: &'a AtomicU64,
    odb_reads: &'a AtomicUsize,
}

impl<'a, Find> traits::ReadData<'a> for ReadDataImpl<'a, Find>
where
    Find: gix_object::Find,
{
    fn read_blob(self) -> Result<&'a [u8], Error> {
        Ok(self.objects.find_blob(self.id, self.buf).map(|b| {
            self.odb_reads.fetch_add(1, Ordering::Relaxed);
            self.odb_bytes.fetch_add(b.data.len() as u64, Ordering::Relaxed);
            b.data
        })?)
    }

    fn stream_worktree_file(self) -> Result<Stream<'a>, Error> {
        self.buf.clear();
        // symlinks are only stored as actual symlinks if the FS supports it otherwise they are just
        // normal files with their content equal to the linked path (so can be read normally)
        //
        let is_symlink = self.entry.mode == gix_index::entry::Mode::SYMLINK;
        // TODO: what to do about precompose unicode and ignore_case for symlinks
        let out = if is_symlink && self.options.fs.symlink {
            // conversion to bstr can never fail because symlinks are only used
            // on unix (by git) so no reason to use the try version here
            let symlink_path =
                gix_path::to_unix_separators_on_windows(gix_path::into_bstr(std::fs::read_link(self.path)?));
            self.buf.extend_from_slice(&symlink_path);
            self.worktree_bytes.fetch_add(self.buf.len() as u64, Ordering::Relaxed);
            Stream {
                inner: ToGitOutcome::Buffer(self.buf),
                bytes: None,
                len: None,
            }
        } else {
            self.buf.clear();
            let platform = self
                .attr_stack
                .at_entry(self.rela_path, Some(self.entry.mode), &self.objects)?;
            let file = std::fs::File::open(self.path)?;
            let out = self
                .filter
                .convert_to_git(
                    file,
                    self.path,
                    &mut |_path, attrs| {
                        platform.matching_attributes(attrs);
                    },
                    &mut |buf| Ok(self.objects.find_blob(self.id, buf).map(|_| Some(()))?),
                )
                .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
            let len = match out {
                ToGitOutcome::Unchanged(_) => Some(self.file_len),
                ToGitOutcome::Process(_) | ToGitOutcome::Buffer(_) => None,
            };
            Stream {
                inner: out,
                bytes: Some(self.worktree_bytes),
                len,
            }
        };

        self.worktree_reads.fetch_add(1, Ordering::Relaxed);
        Ok(out)
    }
}

struct OffsetIter<'a, T> {
    inner: Chunks<'a, T>,
    offset: usize,
}

impl<'a, T> Iterator for OffsetIter<'a, T> {
    type Item = (usize, &'a [T]);

    fn next(&mut self) -> Option<Self::Item> {
        let block = self.inner.next()?;
        let offset = self.offset;
        self.offset += block.len();
        Some((offset, block))
    }
}

impl Conflict {
    /// Given `entries` and `path_backing`, both values obtained from an [index](gix_index::State), use `start_index` and enumerate
    /// all conflict stages that still match `entry_path` to produce a conflict description.
    /// Also return the amount of extra-entries that were part of the conflict declaration (not counting the entry at `start_index`)
    ///
    /// If for some reason entry at `start_index` isn't in conflicting state, `None` is returned.
    pub fn try_from_entry(
        entries: &[gix_index::Entry],
        path_backing: &gix_index::PathStorageRef,
        start_index: usize,
        entry_path: &BStr,
    ) -> Option<(Self, usize)> {
        use Conflict::*;
        let mut mask = None::<u8>;

        let mut count = 0_usize;
        for stage in (start_index..(start_index + 3).min(entries.len())).filter_map(|idx| {
            let entry = &entries[idx];
            let stage = entry.stage_raw();
            (stage > 0 && entry.path_in(path_backing) == entry_path).then_some(stage)
        }) {
            // This could be `1 << (stage - 1)` but let's be specific.
            *mask.get_or_insert(0) |= match stage {
                1 => 0b001,
                2 => 0b010,
                3 => 0b100,
                _ => 0,
            };
            count += 1;
        }

        mask.map(|mask| {
            (
                match mask {
                    0b001 => BothDeleted,
                    0b010 => AddedByUs,
                    0b011 => DeletedByThem,
                    0b100 => AddedByThem,
                    0b101 => DeletedByUs,
                    0b110 => BothAdded,
                    0b111 => BothModified,
                    _ => unreachable!("BUG: bitshifts and typical entry layout doesn't allow for more"),
                },
                count - 1,
            )
        })
    }
}

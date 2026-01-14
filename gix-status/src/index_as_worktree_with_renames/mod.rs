//! Changes between the index and the worktree along with optional rename tracking.
mod types;
pub use types::{Context, DirwalkContext, Entry, Error, Options, Outcome, RewriteSource, Sorting, Summary, VisitEntry};

mod recorder;
pub use recorder::Recorder;

pub(super) mod function {
    use std::{borrow::Cow, path::Path};

    use bstr::ByteSlice;
    use gix_worktree::stack::State;

    use crate::{
        index_as_worktree::traits::{CompareBlobs, SubmoduleStatus},
        index_as_worktree_with_renames::{
            function::rewrite::ModificationOrDirwalkEntry, Context, Entry, Error, Options, Outcome, RewriteSource,
            VisitEntry,
        },
        is_dir_to_mode,
    };

    /// Similar to [`index_as_worktree(…)`](crate::index_as_worktree()), except that it will automatically
    /// track renames if enabled, while additionally providing information about untracked files
    /// (or more, depending on the configuration).
    ///
    /// * `index`
    ///     - used for checking modifications, and also for knowing which files are tracked during
    ///       the working-dir traversal.
    /// * `worktree`
    ///     - The root of the worktree, in a format that respects `core.precomposeUnicode`.
    /// * `collector`
    ///     - A [`VisitEntry`] implementation that sees the results of this operation.
    /// * `compare`
    ///     - An implementation to compare two blobs for equality, used during index modification checks.
    /// * `submodule`
    ///     - An implementation to determine the status of a submodule when encountered during
    ///       index modification checks.
    /// * `objects`
    ///     - A way to obtain objects from the git object database.
    /// * `progress`
    ///     - A way to send progress information for the index modification checks.
    /// * `ctx`
    ///    -  Additional information that will be accessed during index modification checks and traversal.
    /// * `options`
    ///    - a way to configure both paths of the operation.
    #[allow(clippy::too_many_arguments)]
    pub fn index_as_worktree_with_renames<'index, T, U, Find, E>(
        index: &'index gix_index::State,
        worktree: &Path,
        collector: &mut impl VisitEntry<'index, ContentChange = T, SubmoduleStatus = U>,
        compare: impl CompareBlobs<Output = T> + Send + Clone,
        submodule: impl SubmoduleStatus<Output = U, Error = E> + Send + Clone,
        objects: Find,
        progress: &mut dyn gix_features::progress::Progress,
        mut ctx: Context<'_>,
        options: Options<'_>,
    ) -> Result<Outcome, Error>
    where
        T: Send + Clone,
        U: Send + Clone,
        E: std::error::Error + Send + Sync + 'static,
        Find: gix_object::Find + gix_object::FindHeader + Send + Clone,
    {
        gix_features::parallel::threads(|scope| -> Result<Outcome, Error> {
            let (tx, rx) = std::sync::mpsc::channel();
            let walk_outcome = options
                .dirwalk
                .map(|options| {
                    gix_features::parallel::build_thread()
                        .name("gix_status::dirwalk".into())
                        .spawn_scoped(scope, {
                            let tx = tx.clone();
                            let mut collect = dirwalk::Delegate {
                                tx,
                                should_interrupt: ctx.should_interrupt,
                            };
                            let dirwalk_ctx = ctx.dirwalk;
                            let objects = objects.clone();
                            let mut excludes = match ctx.resource_cache.attr_stack.state() {
                                State::CreateDirectoryAndAttributesStack { .. } | State::AttributesStack(_) => None,
                                State::AttributesAndIgnoreStack { .. } | State::IgnoreStack(_) => {
                                    Some(ctx.resource_cache.attr_stack.clone())
                                }
                            };
                            let mut pathspec_attr_stack = ctx
                                .pathspec
                                .patterns()
                                .any(|p| !p.attributes.is_empty())
                                .then(|| ctx.resource_cache.attr_stack.clone());
                            let mut pathspec = ctx.pathspec.clone();
                            move || -> Result<_, Error> {
                                gix_dir::walk(
                                    worktree,
                                    gix_dir::walk::Context {
                                        should_interrupt: Some(ctx.should_interrupt),
                                        git_dir_realpath: dirwalk_ctx.git_dir_realpath,
                                        current_dir: dirwalk_ctx.current_dir,
                                        index,
                                        ignore_case_index_lookup: dirwalk_ctx.ignore_case_index_lookup,
                                        pathspec: &mut pathspec,
                                        pathspec_attributes: &mut |relative_path, case, is_dir, out| {
                                            let stack = pathspec_attr_stack
                                                .as_mut()
                                                .expect("can only be called if attributes are used in patterns");
                                            stack
                                                .set_case(case)
                                                .at_entry(relative_path, Some(is_dir_to_mode(is_dir)), &objects)
                                                .is_ok_and(|platform| platform.matching_attributes(out))
                                        },
                                        excludes: excludes.as_mut(),
                                        objects: &objects,
                                        explicit_traversal_root: Some(worktree),
                                    },
                                    options,
                                    &mut collect,
                                )
                                .map_err(Error::DirWalk)
                            }
                        })
                        .map_err(Error::SpawnThread)
                })
                .transpose()?;

            let entries = &index.entries()[index
                .prefixed_entries_range(ctx.pathspec.common_prefix())
                .unwrap_or(0..index.entries().len())];

            let filter = options.rewrites.is_some().then(|| {
                (
                    ctx.resource_cache.filter.worktree_filter.clone(),
                    ctx.resource_cache.attr_stack.clone(),
                )
            });
            let tracked_modifications_outcome = gix_features::parallel::build_thread()
                .name("gix_status::index_as_worktree".into())
                .spawn_scoped(scope, {
                    let mut collect = tracked_modifications::Delegate { tx };
                    let objects = objects.clone();
                    let stack = ctx.resource_cache.attr_stack.clone();
                    let filter = ctx.resource_cache.filter.worktree_filter.clone();
                    move || -> Result<_, Error> {
                        crate::index_as_worktree(
                            index,
                            worktree,
                            &mut collect,
                            compare,
                            submodule,
                            objects,
                            progress,
                            crate::index_as_worktree::Context {
                                pathspec: ctx.pathspec,
                                stack,
                                filter,
                                should_interrupt: ctx.should_interrupt,
                            },
                            options.tracked_file_modifications,
                        )
                        .map_err(Error::TrackedFileModifications)
                    }
                })
                .map_err(Error::SpawnThread)?;

            let tracker = options
                .rewrites
                .map(gix_diff::rewrites::Tracker::<ModificationOrDirwalkEntry<'index, T, U>>::new)
                .zip(filter);
            let rewrite_outcome = match tracker {
                Some((mut tracker, (mut filter, mut attrs))) => {
                    let mut entries_for_sorting = options.sorting.map(|_| Vec::new());
                    let mut buf = Vec::new();
                    for event in rx {
                        let (change, location) = match event {
                            Event::IndexEntry(record) => {
                                let location = Cow::Borrowed(record.relative_path);
                                (ModificationOrDirwalkEntry::Modification(record), location)
                            }
                            Event::DirEntry(entry, collapsed_directory_status) => {
                                let location = Cow::Owned(entry.rela_path.clone());
                                (
                                    ModificationOrDirwalkEntry::DirwalkEntry {
                                        id: rewrite::calculate_worktree_id(
                                            options.object_hash,
                                            worktree,
                                            entry.disk_kind,
                                            entry.rela_path.as_bstr(),
                                            &mut filter,
                                            &mut attrs,
                                            &objects,
                                            &mut buf,
                                            ctx.should_interrupt,
                                        )?,
                                        entry,
                                        collapsed_directory_status,
                                    },
                                    location,
                                )
                            }
                        };
                        if let Some(v) = entries_for_sorting.as_mut() {
                            v.push((change, location));
                        } else if let Some(change) = tracker.try_push_change(change, location.as_ref()) {
                            collector.visit_entry(rewrite::change_to_entry(change, entries));
                        }
                    }

                    let mut entries_for_sorting = entries_for_sorting.map(|mut v| {
                        v.sort_by(|a, b| a.1.cmp(&b.1));
                        let mut remaining = Vec::new();
                        for (change, location) in v {
                            if let Some(change) = tracker.try_push_change(change, location.as_ref()) {
                                remaining.push(rewrite::change_to_entry(change, entries));
                            }
                        }
                        remaining
                    });

                    let outcome = tracker.emit(
                        |dest, src| {
                            match src {
                                None => {
                                    let entry = rewrite::change_to_entry(dest.change, entries);
                                    if let Some(v) = entries_for_sorting.as_mut() {
                                        v.push(entry);
                                    } else {
                                        collector.visit_entry(entry);
                                    }
                                }
                                Some(src) => {
                                    let ModificationOrDirwalkEntry::DirwalkEntry {
                                        id,
                                        entry,
                                        collapsed_directory_status,
                                    } = dest.change
                                    else {
                                        unreachable!("BUG: only possible destinations are dirwalk entries (additions)");
                                    };
                                    let source = match src.change {
                                        ModificationOrDirwalkEntry::Modification(record) => {
                                            RewriteSource::RewriteFromIndex {
                                                index_entries: entries,
                                                source_entry: record.entry,
                                                source_entry_index: record.entry_index,
                                                source_rela_path: record.relative_path,
                                                source_status: record.status.clone(),
                                            }
                                        }
                                        ModificationOrDirwalkEntry::DirwalkEntry {
                                            id,
                                            entry,
                                            collapsed_directory_status,
                                        } => RewriteSource::CopyFromDirectoryEntry {
                                            source_dirwalk_entry: entry.clone(),
                                            source_dirwalk_entry_collapsed_directory_status:
                                                *collapsed_directory_status,
                                            source_dirwalk_entry_id: *id,
                                        },
                                    };

                                    let entry = Entry::Rewrite {
                                        source,
                                        dirwalk_entry: entry,
                                        dirwalk_entry_collapsed_directory_status: collapsed_directory_status,
                                        dirwalk_entry_id: id,
                                        diff: src.diff,
                                        copy: src.kind == gix_diff::rewrites::tracker::visit::SourceKind::Copy,
                                    };
                                    if let Some(v) = entries_for_sorting.as_mut() {
                                        v.push(entry);
                                    } else {
                                        collector.visit_entry(entry);
                                    }
                                }
                            }
                            std::ops::ControlFlow::Continue(())
                        },
                        &mut ctx.resource_cache,
                        &objects,
                        |_cb| {
                            // NOTE: to make this work, we'd want to wait the index modification check to complete.
                            //       Then it's possible to efficiently emit the tracked files along with what we already sent,
                            //       i.e. untracked and ignored files.
                            gix_features::trace::debug!("full-tree copy tracking isn't currently supported");
                            Ok::<_, std::io::Error>(())
                        },
                    )?;

                    if let Some(mut v) = entries_for_sorting {
                        v.sort_by(|a, b| a.destination_rela_path().cmp(b.destination_rela_path()));
                        for entry in v {
                            collector.visit_entry(entry);
                        }
                    }
                    Some(outcome)
                }
                None => {
                    let mut entries_for_sorting = options.sorting.map(|_| Vec::new());
                    for event in rx {
                        let entry = match event {
                            Event::IndexEntry(record) => Entry::Modification {
                                entries,
                                entry: record.entry,
                                entry_index: record.entry_index,
                                rela_path: record.relative_path,
                                status: record.status,
                            },
                            Event::DirEntry(entry, collapsed_directory_status) => Entry::DirectoryContents {
                                entry,
                                collapsed_directory_status,
                            },
                        };

                        if let Some(v) = entries_for_sorting.as_mut() {
                            v.push(entry);
                        } else {
                            collector.visit_entry(entry);
                        }
                    }

                    if let Some(mut v) = entries_for_sorting {
                        v.sort_by(|a, b| a.destination_rela_path().cmp(b.destination_rela_path()));
                        for entry in v {
                            collector.visit_entry(entry);
                        }
                    }
                    None
                }
            };

            let walk_outcome = walk_outcome
                .map(|handle| handle.join().expect("no panic"))
                .transpose()?;
            let tracked_modifications_outcome = tracked_modifications_outcome.join().expect("no panic")?;
            Ok(Outcome {
                dirwalk: walk_outcome.map(|t| t.0),
                tracked_file_modification: tracked_modifications_outcome,
                rewrites: rewrite_outcome,
            })
        })
    }

    enum Event<'index, T, U> {
        IndexEntry(crate::index_as_worktree::Record<'index, T, U>),
        DirEntry(gix_dir::Entry, Option<gix_dir::entry::Status>),
    }

    mod tracked_modifications {
        use bstr::BStr;
        use gix_index::Entry;

        use crate::{
            index_as_worktree::{EntryStatus, Record},
            index_as_worktree_with_renames::function::Event,
        };

        pub(super) struct Delegate<'index, T, U> {
            pub(super) tx: std::sync::mpsc::Sender<Event<'index, T, U>>,
        }

        impl<'index, T, U> crate::index_as_worktree::VisitEntry<'index> for Delegate<'index, T, U> {
            type ContentChange = T;
            type SubmoduleStatus = U;

            fn visit_entry(
                &mut self,
                _entries: &'index [Entry],
                entry: &'index Entry,
                entry_index: usize,
                rela_path: &'index BStr,
                status: EntryStatus<Self::ContentChange, Self::SubmoduleStatus>,
            ) {
                self.tx
                    .send(Event::IndexEntry(Record {
                        entry,
                        entry_index,
                        relative_path: rela_path,
                        status,
                    }))
                    .ok();
            }
        }
    }

    mod dirwalk {
        use std::sync::atomic::{AtomicBool, Ordering};

        use gix_dir::{entry::Status, walk::Action, EntryRef};

        use super::Event;

        pub(super) struct Delegate<'index, 'a, T, U> {
            pub(super) tx: std::sync::mpsc::Sender<Event<'index, T, U>>,
            pub(super) should_interrupt: &'a AtomicBool,
        }

        impl<T, U> gix_dir::walk::Delegate for Delegate<'_, '_, T, U> {
            fn emit(&mut self, entry: EntryRef<'_>, collapsed_directory_status: Option<Status>) -> Action {
                // Status never shows untracked entries of untrackable type
                if entry.disk_kind != Some(gix_dir::entry::Kind::Untrackable) {
                    let entry = entry.to_owned();
                    self.tx.send(Event::DirEntry(entry, collapsed_directory_status)).ok();
                }

                if self.should_interrupt.load(Ordering::Relaxed) {
                    std::ops::ControlFlow::Break(())
                } else {
                    std::ops::ControlFlow::Continue(())
                }
            }
        }
    }

    mod rewrite {
        use std::{io::Read, path::Path};

        use bstr::BStr;
        use gix_diff::{rewrites::tracker::ChangeKind, tree::visit::Relation};
        use gix_dir::entry::Kind;
        use gix_filter::pipeline::convert::ToGitOutcome;
        use gix_hash::oid;
        use gix_object::tree::EntryMode;

        use crate::{
            index_as_worktree::{Change, EntryStatus},
            index_as_worktree_with_renames::{Entry, Error},
        };

        #[derive(Clone)]
        pub enum ModificationOrDirwalkEntry<'index, T, U>
        where
            T: Clone,
            U: Clone,
        {
            Modification(crate::index_as_worktree::Record<'index, T, U>),
            DirwalkEntry {
                id: gix_hash::ObjectId,
                entry: gix_dir::Entry,
                collapsed_directory_status: Option<gix_dir::entry::Status>,
            },
        }

        impl<T, U> gix_diff::rewrites::tracker::Change for ModificationOrDirwalkEntry<'_, T, U>
        where
            T: Clone,
            U: Clone,
        {
            fn id(&self) -> &oid {
                match self {
                    ModificationOrDirwalkEntry::Modification(m) => &m.entry.id,
                    ModificationOrDirwalkEntry::DirwalkEntry { id, .. } => id,
                }
            }

            fn relation(&self) -> Option<Relation> {
                // TODO: figure out if index or worktree can provide containerization - worktree should be possible.
                //       index would take some processing.
                None
            }

            fn kind(&self) -> ChangeKind {
                match self {
                    ModificationOrDirwalkEntry::Modification(m) => match &m.status {
                        EntryStatus::Conflict { .. } | EntryStatus::IntentToAdd | EntryStatus::NeedsUpdate(_) => {
                            ChangeKind::Modification
                        }
                        EntryStatus::Change(c) => match c {
                            Change::Removed => ChangeKind::Deletion,
                            Change::Type { .. } | Change::Modification { .. } | Change::SubmoduleModification(_) => {
                                ChangeKind::Modification
                            }
                        },
                    },
                    ModificationOrDirwalkEntry::DirwalkEntry { .. } => ChangeKind::Addition,
                }
            }

            fn entry_mode(&self) -> EntryMode {
                match self {
                    ModificationOrDirwalkEntry::Modification(c) => c.entry.mode.to_tree_entry_mode(),
                    ModificationOrDirwalkEntry::DirwalkEntry { entry, .. } => entry.disk_kind.map(|kind| {
                        match kind {
                            Kind::Untrackable => {
                                // Trees are never tracked for rewrites, so we 'pretend'.
                                gix_object::tree::EntryKind::Tree
                            }
                            Kind::File => gix_object::tree::EntryKind::Blob,
                            Kind::Symlink => gix_object::tree::EntryKind::Link,
                            Kind::Repository | Kind::Directory => gix_object::tree::EntryKind::Tree,
                        }
                        .into()
                    }),
                }
                .unwrap_or(gix_object::tree::EntryKind::Blob.into())
            }

            fn id_and_entry_mode(&self) -> (&oid, EntryMode) {
                (self.id(), self.entry_mode())
            }
        }

        /// Note that for non-files, we always return a null-sha and assume that the rename-tracking
        /// does nothing for these anyway.
        #[allow(clippy::too_many_arguments)]
        pub(super) fn calculate_worktree_id(
            object_hash: gix_hash::Kind,
            worktree_root: &Path,
            disk_kind: Option<gix_dir::entry::Kind>,
            rela_path: &BStr,
            filter: &mut gix_filter::Pipeline,
            attrs: &mut gix_worktree::Stack,
            objects: &dyn gix_object::Find,
            buf: &mut Vec<u8>,
            should_interrupt: &std::sync::atomic::AtomicBool,
        ) -> Result<gix_hash::ObjectId, Error> {
            let Some(kind) = disk_kind else {
                return Ok(object_hash.null());
            };

            Ok(match kind {
                Kind::Untrackable => {
                    // Go along with unreadable files, they are passed along without rename tracking.
                    return Ok(object_hash.null());
                }
                Kind::File => {
                    let platform = attrs
                        .at_entry(rela_path, None, objects)
                        .map_err(Error::SetAttributeContext)?;
                    let rela_path = gix_path::from_bstr(rela_path);
                    let file_path = worktree_root.join(rela_path.as_ref());
                    let file = std::fs::File::open(&file_path).map_err(Error::OpenWorktreeFile)?;
                    let out = filter.convert_to_git(
                        file,
                        rela_path.as_ref(),
                        &mut |_path, attrs| {
                            platform.matching_attributes(attrs);
                        },
                        &mut |_buf| Ok(None),
                    )?;
                    match out {
                        ToGitOutcome::Unchanged(mut file) => gix_object::compute_stream_hash(
                            object_hash,
                            gix_object::Kind::Blob,
                            &mut file,
                            file_path.metadata().map_err(Error::OpenWorktreeFile)?.len(),
                            &mut gix_features::progress::Discard,
                            should_interrupt,
                        )
                        .map_err(Error::HashFile)?,
                        ToGitOutcome::Buffer(buf) => gix_object::compute_hash(object_hash, gix_object::Kind::Blob, buf)
                            .map_err(|err| Error::HashFile(err.into()))?,
                        ToGitOutcome::Process(mut stream) => {
                            buf.clear();
                            stream.read_to_end(buf).map_err(|err| Error::HashFile(err.into()))?;
                            gix_object::compute_hash(object_hash, gix_object::Kind::Blob, buf)
                                .map_err(|err| Error::HashFile(err.into()))?
                        }
                    }
                }
                Kind::Symlink => {
                    let path = worktree_root.join(gix_path::from_bstr(rela_path));
                    let target = gix_path::into_bstr(std::fs::read_link(path).map_err(Error::ReadLink)?);
                    gix_object::compute_hash(object_hash, gix_object::Kind::Blob, &target)
                        .map_err(|err| Error::HashFile(err.into()))?
                }
                Kind::Directory | Kind::Repository => object_hash.null(),
            })
        }

        #[inline]
        pub(super) fn change_to_entry<'index, T, U>(
            change: ModificationOrDirwalkEntry<'index, T, U>,
            entries: &'index [gix_index::Entry],
        ) -> Entry<'index, T, U>
        where
            T: Clone,
            U: Clone,
        {
            match change {
                ModificationOrDirwalkEntry::Modification(r) => Entry::Modification {
                    entries,
                    entry: r.entry,
                    entry_index: r.entry_index,
                    rela_path: r.relative_path,
                    status: r.status,
                },
                ModificationOrDirwalkEntry::DirwalkEntry {
                    id: _,
                    entry,
                    collapsed_directory_status,
                } => Entry::DirectoryContents {
                    entry,
                    collapsed_directory_status,
                },
            }
        }
    }
}

use std::{
    cell::RefCell,
    convert::Infallible,
    sync::atomic::{AtomicUsize, Ordering},
    time::Instant,
};

use anyhow::{anyhow, bail};
use gix::{
    bstr::{BStr, BString, ByteSlice},
    diff::{blob::platform::prepare_diff::Operation, rewrites::CopySource},
    features::progress,
    objs::find::Error,
    parallel::{InOrderIter, SequenceId},
    prelude::ObjectIdExt,
    Count, Progress,
};
use rusqlite::{params, Statement, Transaction};

use crate::query::Options;

pub fn update(
    repo: &gix::Repository,
    con: &mut rusqlite::Connection,
    progress: &mut impl gix::NestedProgress,
    mut err: impl std::io::Write,
    Options {
        object_cache_size_mb,
        find_copies_harder,
        threads,
    }: Options,
) -> anyhow::Result<Vec<gix::ObjectId>> {
    let commit_id = repo.head_id()?.detach();
    let threads = gix::features::parallel::num_threads(threads);

    let mut stat_progress = {
        let mut p = progress.add_child("extract stats");
        p.init(None, progress::count("commits"));
        p
    };
    let stat_counter = stat_progress.counter();

    let mut db_progress = {
        let mut p = progress.add_child("db cache");
        p.init(None, progress::count("events"));
        p
    };
    let commit_counter = db_progress.counter();

    let change_progress = {
        let mut p = progress.add_child("find changes");
        p.init(None, progress::count("modified files"));
        p
    };
    let change_counter = change_progress.counter();

    let lines_progress = {
        let mut p = progress.add_child("find changes");
        p.init(None, progress::count("diff lines"));
        p
    };
    let lines_counter = lines_progress.counter();

    let mut traverse_progress = progress.add_child("traverse commit graph");
    traverse_progress.init(None, progress::count("commits"));

    let out = std::thread::scope(|scope| -> anyhow::Result<_> {
        struct CommitDiffStats {
            /// The id of the commit which was diffed with its predecessor
            id: gix::hash::ObjectId,
            changes: Vec<FileChange>,
        }
        let start = Instant::now();
        let (tx_stats, rx_stats) = std::sync::mpsc::channel::<Result<(SequenceId, Vec<CommitDiffStats>), Infallible>>();

        let mut all_commits = Vec::with_capacity(con.query_row("SELECT  COUNT(hash) from commits", [], |r| {
            r.get::<_, i64>(0).map(|v| v as usize)
        })?);
        for item in con
            .prepare("SELECT hash from commits ORDER BY ROWID")?
            .query_map([], |r| {
                Ok(gix::ObjectId::try_from(r.get_ref(0)?.as_bytes()?)
                    .unwrap_or_else(|_| gix::ObjectId::null(gix::hash::Kind::Sha1)))
            })?
        {
            all_commits.push(item?);
        }
        let mut known_commits = all_commits.clone();
        known_commits.sort();

        let db_thread = scope.spawn({
            move || -> anyhow::Result<()> {
                let trans = con.transaction()?;
                {
                    let Updates {
                        mut new_commit,
                        mut insert_commit_file,
                        mut insert_commit_file_with_source,
                        mut insert_file_path,
                    } = Updates::new(&trans)?;
                    for stats in InOrderIter::from(rx_stats.into_iter()) {
                        for CommitDiffStats { id, changes } in stats.expect("infallible") {
                            new_commit.execute(params![id.as_bytes()])?;
                            for change in changes {
                                insert_file_path.execute(params![change.relpath.to_str_lossy()])?;
                                let (has_diff, lines) = change.lines.map(|l| (true, l)).unwrap_or_default();
                                if let Some(source_relpath) = change.source_relpath {
                                    insert_file_path.execute(params![source_relpath.to_str_lossy()])?;
                                    insert_commit_file_with_source.execute(params![
                                        id.as_bytes(),
                                        change.relpath.to_str_lossy(),
                                        has_diff,
                                        lines.added as i64,
                                        lines.removed as i64,
                                        lines.before as i64,
                                        lines.after as i64,
                                        change.mode as i64,
                                        source_relpath.to_str_lossy(),
                                    ])?;
                                } else {
                                    insert_commit_file.execute(params![
                                        id.as_bytes(),
                                        change.relpath.to_str_lossy(),
                                        has_diff,
                                        lines.added as i64,
                                        lines.removed as i64,
                                        lines.before as i64,
                                        lines.after as i64,
                                        change.mode as i64,
                                    ])?;
                                }
                            }
                            commit_counter.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
                trans.commit()?;
                Ok(())
            }
        });

        let rewrites = {
            // These are either configured, or we set them to the default. There is no turning them off.
            let (r, was_configured) = gix::diff::new_rewrites(&repo.config_snapshot(), true)?;
            if was_configured && r.is_none() {
                gix::trace::warn!("Rename tracking is disabled by configuration, but we enable it using the default");
            }
            let mut r = r.unwrap_or_default();
            r.copies = Some(gix::diff::rewrites::Copies {
                source: if find_copies_harder {
                    CopySource::FromSetOfModifiedFilesAndAllSources
                } else {
                    CopySource::FromSetOfModifiedFiles
                },
                percentage: None,
            });
            r
        };

        #[derive(Clone)]
        struct Task {
            commit: gix::hash::ObjectId,
            parent_commit: Option<gix::hash::ObjectId>,
            compute_stats: bool,
        }

        type Packet = (SequenceId, Vec<Task>);

        let (tx_tree_ids, stat_threads) = {
            let (tx, rx) = crossbeam_channel::unbounded::<Packet>();
            let stat_workers = (0..threads)
                .map(|_| {
                    scope.spawn({
                        let stat_counter = stat_counter.clone();
                        let change_counter = change_counter.clone();
                        let lines_counter = lines_counter.clone();
                        let tx_stats = tx_stats.clone();
                        let mut repo = repo.clone();
                        repo.object_cache_size_if_unset((object_cache_size_mb * 1024 * 1024) / threads);
                        let rx = rx.clone();
                        move || -> anyhow::Result<()> {
                            let mut rewrite_cache =
                                repo.diff_resource_cache(gix::diff::blob::pipeline::Mode::ToGit, Default::default())?;
                            let mut diff_cache = rewrite_cache.clone();
                            for (chunk_id, chunk) in rx {
                                let mut out_chunk = Vec::with_capacity(chunk.len());
                                for Task {
                                    parent_commit,
                                    commit,
                                    compute_stats,
                                } in chunk
                                {
                                    stat_counter.fetch_add(1, Ordering::SeqCst);
                                    if gix::interrupt::is_triggered() {
                                        return Ok(());
                                    }
                                    let mut out = Vec::new();
                                    if compute_stats {
                                        let from = match parent_commit {
                                            Some(id) => {
                                                match repo.find_object(id).ok().and_then(|c| c.peel_to_tree().ok()) {
                                                    Some(tree) => tree,
                                                    None => continue,
                                                }
                                            }
                                            None => repo.empty_tree(),
                                        };
                                        let to = match repo.find_object(commit).ok().and_then(|c| c.peel_to_tree().ok())
                                        {
                                            Some(c) => c,
                                            None => continue,
                                        };
                                        rewrite_cache.clear_resource_cache_keep_allocation();
                                        diff_cache.clear_resource_cache_keep_allocation();
                                        from.changes()?
                                            .options(|opts| {
                                                opts.track_path().track_rewrites(Some(rewrites));
                                            })
                                            .for_each_to_obtain_tree_with_cache(&to, &mut rewrite_cache, |change| {
                                                use gix::object::tree::diff::Change::*;
                                                change_counter.fetch_add(1, Ordering::SeqCst);
                                                match change {
                                                    Addition {
                                                        entry_mode,
                                                        id,
                                                        location,
                                                        ..
                                                    } => {
                                                        if entry_mode.is_blob_or_symlink() {
                                                            add_lines(&mut out, location, &lines_counter, id);
                                                        }
                                                    }
                                                    Modification {
                                                        entry_mode,
                                                        previous_entry_mode,
                                                        id,
                                                        previous_id,
                                                        location,
                                                    } => match (previous_entry_mode.is_blob(), entry_mode.is_blob()) {
                                                        (false, false) => {}
                                                        (false, true) => {
                                                            add_lines(&mut out, location, &lines_counter, id);
                                                        }
                                                        (true, false) => {
                                                            add_lines(&mut out, location, &lines_counter, previous_id);
                                                        }
                                                        (true, true) => {
                                                            if let Ok(cache) =
                                                                change.diff(&mut diff_cache).map(|p| p.resource_cache)
                                                            {
                                                                cache
                                                                    .options
                                                                    .skip_internal_diff_if_external_is_configured =
                                                                    false;
                                                                if let Ok(prep) = cache.prepare_diff() {
                                                                    let mut nl = 0;
                                                                    let tokens = prep.interned_input();
                                                                    match prep.operation {
                                                                        Operation::InternalDiff { algorithm } => {
                                                                            let counts = gix::diff::blob::diff(
                                                                                algorithm,
                                                                                &tokens,
                                                                                gix::diff::blob::sink::Counter::default(
                                                                                ),
                                                                            );
                                                                            nl += counts.insertions as usize
                                                                                + counts.removals as usize;
                                                                            let lines = LineStats {
                                                                                added: counts.insertions as usize,
                                                                                removed: counts.removals as usize,
                                                                                before: tokens.before.len(),
                                                                                after: tokens.after.len(),
                                                                            };
                                                                            lines_counter
                                                                                .fetch_add(nl, Ordering::SeqCst);
                                                                            out.push(FileChange {
                                                                                relpath: location.to_owned(),
                                                                                mode: FileMode::Modified,
                                                                                source_relpath: None,
                                                                                lines: Some(lines),
                                                                            });
                                                                        }
                                                                        Operation::ExternalCommand { .. } => {
                                                                            unreachable!("disabled above")
                                                                        }
                                                                        Operation::SourceOrDestinationIsBinary => {}
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    },
                                                    Deletion {
                                                        entry_mode,
                                                        id,
                                                        location,
                                                        ..
                                                    } => {
                                                        if entry_mode.is_blob_or_symlink() {
                                                            remove_lines(&mut out, location, &lines_counter, id);
                                                        }
                                                    }
                                                    Rewrite {
                                                        source_location,
                                                        diff,
                                                        copy,
                                                        location,
                                                        ..
                                                    } => {
                                                        out.push(FileChange {
                                                            relpath: location.to_owned(),
                                                            source_relpath: Some(source_location.to_owned()),
                                                            mode: if copy { FileMode::Copy } else { FileMode::Rename },
                                                            lines: diff.map(|d| LineStats {
                                                                added: d.insertions as usize,
                                                                removed: d.removals as usize,
                                                                before: d.before as usize,
                                                                after: d.after as usize,
                                                            }),
                                                        });
                                                    }
                                                }
                                                Ok::<_, Infallible>(Default::default())
                                            })?;
                                        out_chunk.push(CommitDiffStats {
                                            id: commit,
                                            changes: out,
                                        });
                                    } else {
                                        out_chunk.push(CommitDiffStats {
                                            id: commit,
                                            changes: Vec::new(),
                                        });
                                    }
                                }
                                if tx_stats.send(Ok((chunk_id, out_chunk))).is_err() {
                                    bail!("Thread failed to send result");
                                }
                            }
                            Ok(())
                        }
                    })
                })
                .collect::<Vec<_>>();
            (tx, stat_workers)
        };
        drop(tx_stats);

        #[derive(Clone)]
        struct Db<'a, Find: Clone> {
            inner: &'a Find,
            progress: &'a dyn gix::progress::Count,
            chunk: std::cell::RefCell<Vec<Task>>,
            chunk_id: std::cell::RefCell<SequenceId>,
            chunk_size: usize,
            tx: crossbeam_channel::Sender<Packet>,
            known_commits: &'a [gix::ObjectId],
        }

        impl<'a, Find> Db<'a, Find>
        where
            Find: gix::prelude::Find + Clone,
        {
            fn new(
                inner: &'a Find,
                progress: &'a dyn gix::progress::Count,
                chunk_size: usize,
                tx: crossbeam_channel::Sender<Packet>,
                known_commits: &'a [gix::ObjectId],
            ) -> Self {
                Self {
                    inner,
                    progress,
                    known_commits,
                    tx,
                    chunk_size,
                    chunk_id: 0.into(),
                    chunk: RefCell::new(Vec::with_capacity(chunk_size)),
                }
            }

            fn send_last_chunk(self) {
                self.tx.send((self.chunk_id.into_inner(), self.chunk.into_inner())).ok();
            }
        }

        impl<Find> gix::prelude::Find for Db<'_, Find>
        where
            Find: gix::prelude::Find + Clone,
        {
            fn try_find<'b>(&self, id: &gix::oid, buf: &'b mut Vec<u8>) -> Result<Option<gix::objs::Data<'b>>, Error> {
                let obj = self.inner.try_find(id, buf)?;
                let Some(obj) = obj else { return Ok(None) };
                if !obj.kind.is_commit() {
                    return Ok(None);
                }

                self.progress.inc();
                if self.known_commits.binary_search(&id.to_owned()).is_err() {
                    let res = {
                        let mut parents = gix::objs::CommitRefIter::from_bytes(obj.data).parent_ids();
                        let res = parents.next().map(|first_parent| (Some(first_parent), id.to_owned()));
                        match parents.next() {
                            Some(_) => None,
                            None => res,
                        }
                    };
                    if let Some((first_parent, commit)) = res {
                        self.chunk.borrow_mut().push(Task {
                            parent_commit: first_parent,
                            commit,
                            compute_stats: true,
                        });
                    } else {
                        self.chunk.borrow_mut().push(Task {
                            parent_commit: None,
                            commit: id.to_owned(),
                            compute_stats: false,
                        });
                    }
                    if self.chunk.borrow().len() == self.chunk_size {
                        let chunk =
                            std::mem::replace(&mut *self.chunk.borrow_mut(), Vec::with_capacity(self.chunk_size));
                        self.tx.send((*self.chunk_id.borrow(), chunk)).ok();
                        *self.chunk_id.borrow_mut() += 1;
                    }
                }
                Ok(Some(obj))
            }
        }

        let db = Db::new(&repo.objects, &traverse_progress, 50, tx_tree_ids, &known_commits);
        let commit_iter = gix::interrupt::Iter::new(commit_id.ancestors(&db), || anyhow!("Cancelled by user"));
        let mut commits = Vec::new();
        for c in commit_iter {
            match c?.map(|c| c.id) {
                Ok(c) => {
                    if known_commits.binary_search(&c).is_err() {
                        commits.push(c);
                    } else {
                        break;
                    }
                }
                Err(gix::traverse::commit::simple::Error::Find { .. }) => {
                    writeln!(err, "shallow repository - commit history is truncated").ok();
                    break;
                }
                Err(err) => return Err(err.into()),
            }
        }
        db.send_last_chunk();
        let saw_new_commits = !commits.is_empty();
        if saw_new_commits {
            traverse_progress.show_throughput(start);
        }
        drop(traverse_progress);

        let stat_max = Some(commits.len());
        stat_progress.set_max(stat_max);
        db_progress.set_max(stat_max);
        for handle in stat_threads {
            handle.join().expect("no panic")?;
            if gix::interrupt::is_triggered() {
                bail!("Cancelled by user");
            }
        }
        if saw_new_commits {
            stat_progress.show_throughput(start);
            change_progress.show_throughput(start);
            lines_progress.show_throughput(start);
        }

        db_thread.join().expect("no panic")?;
        if saw_new_commits {
            db_progress.show_throughput(start);
        } else {
            db_progress.info("up to date".into());
        }

        commits.extend(all_commits);
        Ok(commits)
    })?;

    Ok(out)
}

fn add_lines(out: &mut Vec<FileChange>, path: &BStr, lines_counter: &AtomicUsize, id: gix::Id<'_>) {
    if let Ok(blob) = id.object() {
        let nl = blob.data.lines_with_terminator().count();
        let mut lines = LineStats::default();
        lines.added += nl;
        lines.after = nl;
        lines_counter.fetch_add(nl, Ordering::SeqCst);
        out.push(FileChange {
            relpath: path.to_owned(),
            mode: FileMode::Added,
            source_relpath: None,
            lines: Some(lines),
        });
    }
}

fn remove_lines(out: &mut Vec<FileChange>, path: &BStr, lines_counter: &AtomicUsize, id: gix::Id<'_>) {
    if let Ok(blob) = id.object() {
        let mut lines = LineStats::default();
        let nl = blob.data.lines_with_terminator().count();
        lines.removed += nl;
        lines.before = nl;
        lines_counter.fetch_add(nl, Ordering::SeqCst);
        out.push(FileChange {
            relpath: path.to_owned(),
            mode: FileMode::Removed,
            source_relpath: None,
            lines: Some(lines),
        });
    }
}

#[derive(Debug, Copy, Clone)]
pub enum FileMode {
    Added = 1,
    Removed = 2,
    Modified = 3,
    Rename = 4,
    Copy = 5,
}

impl FileMode {
    pub fn as_str(&self) -> &'static str {
        use FileMode::*;
        match self {
            Added => "+",
            Removed => "-",
            Modified => "Δ",
            Rename => "➡",
            Copy => "⏸",
        }
    }
    pub fn from_usize(mode: usize) -> Option<Self> {
        use FileMode::*;
        match mode {
            1 => Added,
            2 => Removed,
            3 => Modified,
            4 => Rename,
            5 => Copy,
            _ => return None,
        }
        .into()
    }
}

#[derive(Debug)]
struct FileChange {
    relpath: BString,
    mode: FileMode,
    source_relpath: Option<BString>,
    lines: Option<LineStats>,
}

/// Line statistics for a particular commit.
#[derive(Debug, Default, Copy, Clone)]
struct LineStats {
    /// amount of added lines
    added: usize,
    /// amount of removed lines
    removed: usize,
    /// the amount of lines before the change.
    before: usize,
    /// the amount of lines after the change.
    after: usize,
}

struct Updates<'a> {
    new_commit: Statement<'a>,
    insert_commit_file: Statement<'a>,
    insert_commit_file_with_source: Statement<'a>,
    insert_file_path: Statement<'a>,
}

impl<'a> Updates<'a> {
    fn new(trans: &'a Transaction<'_>) -> rusqlite::Result<Self> {
        let new_commit = trans.prepare(
            r#"INSERT INTO
               commits(hash)
               VALUES(?)"#,
        )?;
        let insert_commit_file = trans.prepare(
            r#"
               INSERT INTO
               commit_file(hash, file_id, has_diff, lines_added, lines_removed, lines_before, lines_after, mode)
               VALUES(?, (SELECT files.file_id FROM files WHERE files.file_path = ?), ?, ?, ?, ?, ?, ?)
            "#,
        )?;
        let insert_commit_file_with_source = trans.prepare(
                r#"
               INSERT INTO
               commit_file(hash, file_id, has_diff, lines_added, lines_removed, lines_before, lines_after, mode, source_file_id)
               VALUES(?, (SELECT files.file_id FROM files WHERE files.file_path = ?), ?, ?, ?, ?, ?, ?, (SELECT files.file_id FROM files WHERE files.file_path = ?))
            "#,
            )?;

        let insert_file_path = trans.prepare(
            r#"
               INSERT OR IGNORE INTO
               files(file_path)
               VALUES(?)
            "#,
        )?;
        Ok(Updates {
            new_commit,
            insert_commit_file,
            insert_commit_file_with_source,
            insert_file_path,
        })
    }
}

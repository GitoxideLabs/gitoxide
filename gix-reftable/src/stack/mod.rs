use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    io::{Read as _, Write as _},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use bstr::BString;
use gix_hash::Kind;

use crate::{Header, Limits, LogRecord, LogValue, RefRecord, RefValue, Table, WriteOptions, Writer};

static UNIQUE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A contextual error produced while opening or mutating a reftable stack.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An immutable member could not be read, validated, or written.
    #[error(transparent)]
    Table(#[from] crate::Error),
    /// Stack filesystem I/O failed.
    #[error("I/O failed for {path}")]
    Io {
        /// The affected path.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
    /// Removing a partial staged table also failed after its write failed.
    #[error("failed to remove partial staged table {path} after its write failed: {cleanup}")]
    StagedTableCleanup {
        /// Path to the partial staged table.
        path: PathBuf,
        /// The original write or synchronization failure.
        #[source]
        source: std::io::Error,
        /// The subsequent cleanup failure.
        cleanup: std::io::Error,
    },
    /// The new generation is visible, but a post-commit durability or observer step failed.
    ///
    /// The operation must not be retried as a fresh mutation. The exact committed
    /// snapshot is retained so callers can continue from the published state.
    #[error("reftable stack publication committed, but a post-commit step failed for {path}")]
    Committed {
        /// The generation that was atomically published.
        snapshot: Box<Snapshot>,
        /// The resource whose post-commit step failed.
        path: PathBuf,
        /// The post-commit durability or observer error.
        #[source]
        source: std::io::Error,
    },
    /// A stack operation received input it cannot represent safely.
    #[error("invalid reftable stack input: {0}")]
    InvalidInput(&'static str),
    /// A configured stack resource limit was exceeded.
    #[error("reftable stack resource limit exceeded: {0}")]
    Limit(&'static str),
    /// A `tables.list` entry or ordering invariant is invalid.
    #[error("invalid reftable stack list {path} at line {line}: {message}")]
    InvalidList {
        /// Path to `tables.list`.
        path: PathBuf,
        /// One-based line number, or zero for a whole-list invariant.
        line: usize,
        /// What invariant was violated.
        message: &'static str,
    },
    /// A stack snapshot could not stabilize while publication was in progress.
    #[error("reftable stack at {path} did not stabilize after {attempts} attempts")]
    ConcurrentModification {
        /// Path to `tables.list`.
        path: PathBuf,
        /// Number of complete snapshot attempts.
        attempts: usize,
    },
    /// A lock-free operation could not publish because its selected stack prefix changed.
    #[error("reftable stack at {path} changed while an operation was in progress")]
    OutdatedStack {
        /// Path to `tables.list`.
        path: PathBuf,
    },
    /// A stack member uses a different object hash than its repository.
    #[error("reftable stack member {path} uses {actual}, expected {expected}")]
    HashMismatch {
        /// Path to the stack member.
        path: PathBuf,
        /// Repository object hash.
        expected: Kind,
        /// Member object hash.
        actual: Kind,
    },
    /// Acquiring a Git-style lock failed.
    #[error("failed to lock reftable resource {path}")]
    Lock {
        /// Resource whose `.lock` file could not be acquired.
        path: PathBuf,
        /// Underlying lock error.
        #[source]
        source: gix_lock::acquire::Error,
    },
}

/// Retry policy for opening a stable stack snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotOptions {
    /// Maximum complete `tables.list` and member-open attempts.
    pub max_attempts: usize,
    /// Maximum accepted `tables.list` size in bytes.
    pub max_list_size: usize,
    /// Maximum cumulative encoded size of all members in one generation.
    pub max_total_table_size: usize,
    /// Maximum cumulative number of reference and log records in one generation.
    pub max_total_records: usize,
}

impl Default for SnapshotOptions {
    fn default() -> Self {
        SnapshotOptions {
            max_attempts: 8,
            max_list_size: 16 * 1024 * 1024,
            max_total_table_size: 512 * 1024 * 1024,
            max_total_records: 16 * 1024 * 1024,
        }
    }
}

impl Error {
    /// Return the exact published snapshot when an error happened after commit.
    ///
    /// A caller receiving `Some` must not retry the mutation as a new operation.
    pub fn committed_snapshot(&self) -> Option<&Snapshot> {
        match self {
            Error::Committed { snapshot, .. } => Some(snapshot),
            _ => None,
        }
    }
}

/// Waiting policy for the authoritative `tables.list.lock`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LockOptions {
    /// How long to retry lock acquisition with randomized quadratic backoff.
    pub timeout: Duration,
}

impl Default for LockOptions {
    fn default() -> Self {
        LockOptions {
            timeout: Duration::from_millis(250),
        }
    }
}

/// Reflog retention inputs applied by full-stack compaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompactOptions {
    /// Drop entries older than this Unix timestamp after retaining `keep_latest_logs`.
    pub expire_logs_before: Option<u64>,
    /// Keep at least this many newest entries per reference regardless of age.
    pub keep_latest_logs: usize,
}

/// Public metadata for one listed immutable table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberInfo {
    /// Basename as recorded in `tables.list`.
    pub file_name: String,
    /// Validated immutable-table header.
    pub header: Header,
    /// Encoded table size in bytes.
    pub file_size: usize,
    /// Number of decoded reference and log records in the table.
    pub record_count: usize,
}

/// Statistics produced by a full stack verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Verification {
    /// Number of listed immutable tables.
    pub tables: usize,
    /// Number of visible, non-deleted references.
    pub references: usize,
    /// Number of visible, non-deleted reflog records.
    pub log_records: usize,
    /// Number of references with an existing reflog, including empty reflogs.
    pub reflogs: usize,
    /// Total bytes occupied by listed table files.
    pub table_bytes: usize,
    /// Oldest update index covered by the stack.
    pub min_update_index: Option<u64>,
    /// Newest update index covered by the stack.
    pub max_update_index: Option<u64>,
}

/// A table that could not be removed after it became unreachable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupFailure {
    /// Path whose removal failed.
    pub path: PathBuf,
    /// Portable category of the filesystem error.
    pub error_kind: std::io::ErrorKind,
    /// Display form of the filesystem error for diagnostics.
    pub message: String,
}

/// Result of removing safely identifiable abandoned stack artifacts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cleanup {
    /// Abandoned files that were removed.
    pub removed: Vec<PathBuf>,
    /// Abandoned files that could not be removed, commonly because another process has them open on Windows.
    pub retained: Vec<PathBuf>,
    /// Filesystem errors corresponding one-for-one with `retained`.
    pub failures: Vec<CleanupFailure>,
}

/// Result of full-stack compaction.
#[derive(Debug, Clone)]
pub struct CompactOutcome {
    /// The newly published one-table snapshot.
    pub snapshot: Snapshot,
    /// Obsolete members successfully removed after publication.
    pub removed: Vec<PathBuf>,
    /// Obsolete members still present but unreachable from `tables.list`.
    pub retained: Vec<PathBuf>,
    /// Filesystem errors corresponding one-for-one with `retained`.
    pub cleanup_failures: Vec<CleanupFailure>,
}

/// A configured reftable stack directory.
#[derive(Debug, Clone)]
pub struct Stack {
    directory: PathBuf,
    object_hash: Kind,
    snapshot_options: SnapshotOptions,
    limits: Limits,
}

/// An immutable, self-contained view of one `tables.list` generation.
#[derive(Debug, Clone)]
pub struct Snapshot {
    generation: Vec<u8>,
    members: Vec<MemberInfo>,
    refs: Vec<RefRecord>,
    logs: Vec<LogRecord>,
    log_records: Vec<LogRecord>,
    reflogs: BTreeSet<BString>,
}

/// A lock-held stack-addition session.
///
/// Higher layers can validate compare-and-swap predicates against [`Self::snapshot`]
/// and create records using [`Self::next_update_index`] before publishing them
/// with [`Self::commit`]. Dropping the session publishes nothing.
#[derive(Debug)]
pub struct LockedAddition {
    stack: Stack,
    snapshot: Snapshot,
    lock: gix_lock::File,
    next_update_index: u64,
}

/// A stable stack generation protected by the authoritative list lock.
///
/// This session is intended for operations that must inspect the immutable
/// member files named by a snapshot without racing publication or compaction.
/// Dropping it releases the lock without changing the stack.
#[derive(Debug)]
pub struct LockedSnapshot {
    snapshot: Snapshot,
    _lock: gix_lock::File,
}

impl Stack {
    /// Open an existing stack rooted at `directory` without changing the filesystem.
    pub fn open(
        directory: impl Into<PathBuf>,
        object_hash: Kind,
        snapshot_options: SnapshotOptions,
        limits: Limits,
    ) -> Result<Self, Error> {
        let directory = directory.into();
        let metadata = std::fs::metadata(&directory).map_err(|source| io_error(directory.clone(), source))?;
        if !metadata.is_dir() {
            return Err(Error::InvalidInput("the reftable stack path is not a directory"));
        }
        let stack = Stack {
            directory,
            object_hash,
            snapshot_options,
            limits,
        };
        stack.snapshot()?;
        Ok(stack)
    }

    /// Create the stack directory and an empty authoritative `tables.list` if needed,
    /// then open its current generation.
    pub fn create(
        directory: impl Into<PathBuf>,
        object_hash: Kind,
        snapshot_options: SnapshotOptions,
        limits: Limits,
    ) -> Result<Self, Error> {
        let directory = directory.into();
        std::fs::create_dir_all(&directory).map_err(|source| io_error(directory.clone(), source))?;
        let list_path = directory.join("tables.list");
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&list_path)
        {
            Ok(list) => {
                list.sync_all().map_err(|source| io_error(list_path.clone(), source))?;
                sync_directory(&directory).map_err(|source| io_error(directory.clone(), source))?;
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(source) => return Err(io_error(list_path, source)),
        }
        Stack::open(directory, object_hash, snapshot_options, limits)
    }

    /// Return the directory containing `tables.list` and immutable members.
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Return the repository object hash required for every member.
    pub fn object_hash(&self) -> Kind {
        self.object_hash
    }

    /// Open a stable, immutable snapshot, retrying publication races.
    pub fn snapshot(&self) -> Result<Snapshot, Error> {
        self.snapshot_with_observer(|_| Ok(()))
    }

    fn snapshot_with_observer(
        &self,
        mut observe_generation: impl FnMut(&[u8]) -> Result<(), Error>,
    ) -> Result<Snapshot, Error> {
        let attempts = self.snapshot_options.max_attempts.max(1);
        for _ in 0..attempts {
            let generation = self.read_list()?;
            let entries = parse_list(&self.list_path(), &generation)?;
            observe_generation(&generation)?;
            match self.open_generation(generation.clone(), entries) {
                Ok(snapshot) => {
                    if self.read_list()? == generation {
                        return Ok(snapshot);
                    }
                }
                Err(err) if is_missing_member(&err) => {
                    if self.read_list()? == generation {
                        return Err(err);
                    }
                }
                Err(err) => return Err(err),
            }
            std::thread::yield_now();
        }
        Err(Error::ConcurrentModification {
            path: self.list_path(),
            attempts,
        })
    }

    /// Return whether `snapshot` still names the authoritative generation.
    ///
    /// This performs a read-only comparison of `tables.list` and acquires no lock.
    pub fn generation_is_current(&self, snapshot: &Snapshot) -> Result<bool, Error> {
        Ok(self.read_list()? == snapshot.generation)
    }

    /// Acquire the authoritative list lock and open the generation it protects.
    pub fn begin_addition(&self, options: LockOptions) -> Result<LockedAddition, Error> {
        let LockedSnapshot { snapshot, _lock: lock } = self.lock_snapshot(options)?;
        let next_update_index = snapshot.members.last().map_or(Ok(1), |member| {
            member
                .header
                .max_update_index
                .checked_add(1)
                .ok_or(Error::InvalidInput("the stack update index is exhausted"))
        })?;
        Ok(LockedAddition {
            stack: self.clone(),
            snapshot,
            lock,
            next_update_index,
        })
    }

    /// Acquire the authoritative list lock and open its stable generation without reserving an update index.
    pub fn lock_snapshot(&self, options: LockOptions) -> Result<LockedSnapshot, Error> {
        let list_path = self.list_path();
        let lock =
            gix_lock::File::acquire_to_update_resource(&list_path, options.timeout.into(), None).map_err(|source| {
                Error::Lock {
                    path: list_path,
                    source,
                }
            })?;
        let snapshot = self.snapshot()?;
        Ok(LockedSnapshot { snapshot, _lock: lock })
    }

    /// Validate the list, every member, every index, and the merged logical view.
    pub fn verify(&self) -> Result<Verification, Error> {
        let snapshot = self.snapshot()?;
        Ok(Verification {
            tables: snapshot.members.len(),
            references: snapshot.refs.len(),
            log_records: snapshot.logs.len(),
            reflogs: snapshot.reflogs.len(),
            table_bytes: snapshot.members.iter().map(|member| member.file_size).sum(),
            min_update_index: snapshot.members.first().map(|member| member.header.min_update_index),
            max_update_index: snapshot.members.last().map(|member| member.header.max_update_index),
        })
    }

    /// Compact every listed member into one table and atomically publish it.
    pub fn compact(&self, options: CompactOptions, lock_options: LockOptions) -> Result<CompactOutcome, Error> {
        self.compact_with_observer(options, lock_options, || Ok(()))
    }

    fn compact_with_observer(
        &self,
        options: CompactOptions,
        lock_options: LockOptions,
        observe_unlocked: impl FnOnce() -> Result<(), Error>,
    ) -> Result<CompactOutcome, Error> {
        let _compaction_lock = self.acquire_compaction_lock(lock_options)?;
        let list_path = self.list_path();
        let lock = gix_lock::File::acquire_to_update_resource(&list_path, lock_options.timeout.into(), None).map_err(
            |source| Error::Lock {
                path: list_path.clone(),
                source,
            },
        )?;
        let snapshot = self.snapshot()?;
        if snapshot.members.is_empty() || (snapshot.members.len() == 1 && options.expire_logs_before.is_none()) {
            return Ok(CompactOutcome {
                snapshot,
                removed: Vec::new(),
                retained: Vec::new(),
                cleanup_failures: Vec::new(),
            });
        }

        let mut table_locks = Vec::new();
        table_locks
            .try_reserve(snapshot.members.len())
            .map_err(|_| Error::Limit("stack member lock count"))?;
        for member in &snapshot.members {
            let path = self.directory.join(&member.file_name);
            let marker = gix_lock::Marker::acquire_to_hold_resource(&path, gix_lock::acquire::Fail::Immediately, None)
                .map_err(|source| Error::Lock {
                    path: path.clone(),
                    source,
                })?;
            table_locks.push(marker);
        }
        drop(lock);

        let min = snapshot
            .members
            .first()
            .map(|member| member.header.min_update_index)
            .ok_or(Error::InvalidInput("cannot compact an empty stack"))?;
        let max = snapshot
            .members
            .last()
            .map(|member| member.header.max_update_index)
            .ok_or(Error::InvalidInput("cannot compact an empty stack"))?;
        let mut logs = retain_logs(&snapshot.logs, options);
        for name in &snapshot.reflogs {
            if !logs.iter().any(|record| &record.ref_name == name) {
                logs.push(LogRecord {
                    ref_name: name.clone(),
                    update_index: max,
                    value: LogValue::Placeholder,
                });
            }
        }
        logs.sort_by(|a, b| {
            a.ref_name
                .cmp(&b.ref_name)
                .then_with(|| b.update_index.cmp(&a.update_index))
        });
        let compacted_record_count = snapshot
            .refs
            .len()
            .checked_add(logs.len())
            .ok_or(Error::Limit("stack record count"))?;
        let list_edit = ListEdit::ReplacePrefix(snapshot.members.len());
        self.validate_edited_record_limit(&snapshot, compacted_record_count, list_edit)?;
        let bytes = self.write_table(&snapshot.refs, &logs, (min, max))?;
        let extension = if snapshot.refs.is_empty() && !logs.is_empty() {
            "log"
        } else {
            "ref"
        };
        let mut staged = self.create_staged_table(min, max, extension, &bytes, |staged| {
            self.validate_staged_generation(&snapshot, staged, list_edit)
        })?;
        observe_unlocked()?;

        let lock = gix_lock::File::acquire_to_update_resource(&list_path, lock_options.timeout.into(), None).map_err(
            |source| Error::Lock {
                path: list_path.clone(),
                source,
            },
        )?;
        let current = self.snapshot()?;
        let selected_len = snapshot.members.len();
        let prefix_unchanged = current.members.len() >= selected_len
            && current.members[..selected_len]
                .iter()
                .zip(&snapshot.members)
                .all(|(current, selected)| current.file_name == selected.file_name);
        if !prefix_unchanged {
            return Err(Error::OutdatedStack { path: list_path });
        }
        let published = self.publish_staged(
            lock,
            &current,
            &mut staged,
            ListEdit::ReplacePrefix(selected_len),
            |_| Ok(()),
        )?;
        let file_name = staged.file_name.clone();
        let old_paths = snapshot
            .members
            .iter()
            .map(|member| self.directory.join(&member.file_name))
            .collect::<Vec<_>>();
        drop(table_locks);
        let RemovalOutcome {
            removed,
            retained,
            failures,
        } = remove_paths(old_paths);
        debug_assert_eq!(
            published.members.first().map(|member| member.file_name.as_str()),
            Some(file_name.as_str()),
            "the compacted member is the first member of the published replacement generation"
        );
        Ok(CompactOutcome {
            snapshot: published,
            removed,
            retained,
            cleanup_failures: failures,
        })
    }

    /// Remove abandoned staged files and safe, complete tables not named by a lock-protected snapshot.
    ///
    /// Cleanup serializes with compaction and additions, so a generated `.*.ref.tmp`
    /// or `.*.log.tmp` file cannot still belong to a live Gitoxide publisher. Lock
    /// files are deliberately not guessed at: after a hard crash, an operator must
    /// verify that no process owns the resource before removing the stale `.lock`.
    /// An unlisted complete table newer than the listed stack maximum is also
    /// preserved for manual inspection, as required by Git's cleanup protocol.
    pub fn cleanup_abandoned(&self, options: LockOptions) -> Result<Cleanup, Error> {
        let _compaction_lock = self.acquire_compaction_lock(options)?;
        let list_path = self.list_path();
        let _lock =
            gix_lock::File::acquire_to_update_resource(&list_path, options.timeout.into(), None).map_err(|source| {
                Error::Lock {
                    path: list_path,
                    source,
                }
            })?;
        let snapshot = self.snapshot()?;
        let listed = snapshot
            .members
            .iter()
            .map(|member| member.file_name.as_str())
            .collect::<BTreeSet<_>>();
        let listed_max = snapshot.members.last().map(|member| member.header.max_update_index);
        let mut candidates = Vec::new();
        for entry in std::fs::read_dir(&self.directory).map_err(|source| io_error(self.directory.clone(), source))? {
            let entry = entry.map_err(|source| io_error(self.directory.clone(), source))?;
            let path = entry.path();
            if !entry
                .file_type()
                .map_err(|source| io_error(path.clone(), source))?
                .is_file()
            {
                continue;
            }
            let is_staged = is_staged_table(&path);
            let file_name = path.file_name().and_then(OsStr::to_str);
            let is_listed = file_name.is_some_and(|name| listed.contains(name));
            let is_safe_unlisted_table = file_name
                .and_then(table_name_range)
                .zip(listed_max)
                .is_some_and(|((_, table_max), stack_max)| table_max <= stack_max && !is_listed);
            if is_staged || is_safe_unlisted_table {
                candidates.push(path);
            }
        }
        candidates.sort();
        let RemovalOutcome {
            removed,
            retained,
            failures,
        } = remove_paths(candidates);
        Ok(Cleanup {
            removed,
            retained,
            failures,
        })
    }

    fn list_path(&self) -> PathBuf {
        self.directory.join("tables.list")
    }

    fn compaction_path(&self) -> PathBuf {
        self.directory.join("tables.compaction")
    }

    fn acquire_compaction_lock(&self, options: LockOptions) -> Result<gix_lock::Marker, Error> {
        let path = self.compaction_path();
        gix_lock::Marker::acquire_to_hold_resource(&path, options.timeout.into(), None)
            .map_err(|source| Error::Lock { path, source })
    }

    fn read_list(&self) -> Result<Vec<u8>, Error> {
        let path = self.list_path();
        let file = std::fs::File::open(&path).map_err(|source| io_error(path.clone(), source))?;
        if file.metadata().map_err(|source| io_error(path.clone(), source))?.len()
            > self.snapshot_options.max_list_size as u64
        {
            return Err(Error::Limit("tables.list size"));
        }
        let read_limit = self
            .snapshot_options
            .max_list_size
            .checked_add(1)
            .ok_or(Error::Limit("tables.list size"))?;
        let mut data = Vec::new();
        file.take(read_limit as u64)
            .read_to_end(&mut data)
            .map_err(|source| io_error(path, source))?;
        if data.len() > self.snapshot_options.max_list_size {
            return Err(Error::Limit("tables.list size"));
        }
        Ok(data)
    }

    fn open_generation(&self, generation: Vec<u8>, entries: Vec<ListEntry>) -> Result<Snapshot, Error> {
        let mut members = Vec::new();
        let mut tables = Vec::new();
        members
            .try_reserve(entries.len())
            .map_err(|_| Error::Limit("stack member count"))?;
        tables
            .try_reserve(entries.len())
            .map_err(|_| Error::Limit("stack member count"))?;
        let mut previous_max = None;
        let mut total_table_size = 0usize;
        let mut total_records = 0usize;
        for entry in entries {
            if previous_max.is_some_and(|previous| entry.min <= previous) {
                return Err(Error::InvalidList {
                    path: self.list_path(),
                    line: entry.line,
                    message: "member update-index ranges overlap or are out of order",
                });
            }
            let path = self.directory.join(&entry.file_name);
            let file_size = usize::try_from(
                std::fs::metadata(&path)
                    .map_err(|source| io_error(path.clone(), source))?
                    .len(),
            )
            .map_err(|_| Error::Limit("stack table bytes"))?;
            total_table_size = total_table_size
                .checked_add(file_size)
                .ok_or(Error::Limit("stack table bytes"))?;
            if total_table_size > self.snapshot_options.max_total_table_size {
                return Err(Error::Limit("stack table bytes"));
            }
            let remaining_records = self
                .snapshot_options
                .max_total_records
                .checked_sub(total_records)
                .ok_or(Error::Limit("stack record count"))?;
            let table = match Table::read_with_ref_log_limit(&path, self.limits, remaining_records) {
                Err(crate::Error::Parse { source, .. }) if matches!(source.as_ref(), crate::Error::Limit(message) if *message == "stack record count") =>
                {
                    return Err(Error::Limit("stack record count"));
                }
                result => result?,
            };
            let table_records = table
                .refs()
                .len()
                .checked_add(table.logs().len())
                .ok_or(Error::Limit("stack record count"))?;
            total_records = total_records
                .checked_add(table_records)
                .ok_or(Error::Limit("stack record count"))?;
            if total_records > self.snapshot_options.max_total_records {
                return Err(Error::Limit("stack record count"));
            }
            let header = table.header();
            if header.object_hash != self.object_hash {
                return Err(Error::HashMismatch {
                    path,
                    expected: self.object_hash,
                    actual: header.object_hash,
                });
            }
            if header.min_update_index != entry.min || header.max_update_index != entry.max {
                return Err(Error::InvalidList {
                    path: self.list_path(),
                    line: entry.line,
                    message: "filename range differs from the table header",
                });
            }
            if entry.extension == "log" && table.refs().next().is_some() {
                return Err(Error::InvalidList {
                    path: self.list_path(),
                    line: entry.line,
                    message: "a .log member contains reference records",
                });
            }
            previous_max = Some(entry.max);
            members.push(MemberInfo {
                file_name: entry.file_name,
                header,
                file_size,
                record_count: table_records,
            });
            tables.push(table);
        }
        let (refs, logs, log_records, reflogs) = merge_tables(&tables);
        Ok(Snapshot {
            generation,
            members,
            refs,
            logs,
            log_records,
            reflogs,
        })
    }

    fn write_table(
        &self,
        refs: &[RefRecord],
        logs: &[LogRecord],
        update_index_range: (u64, u64),
    ) -> Result<Vec<u8>, Error> {
        let record_count = refs
            .len()
            .checked_add(logs.len())
            .ok_or(Error::Limit("table record count"))?;
        if record_count > self.limits.max_records {
            return Err(Error::Limit("table record count"));
        }
        let options = WriteOptions {
            update_index_range: Some(update_index_range),
            ..WriteOptions::for_hash(self.object_hash)
        };
        let bytes = Writer::new(options).write(refs, logs)?;
        Table::from_bytes(&bytes, self.limits)?;
        Ok(bytes)
    }

    fn publish(
        &self,
        lock: gix_lock::File,
        request: PublishRequest<'_>,
        mut observe: impl FnMut(PublishStage) -> std::io::Result<()>,
    ) -> Result<Snapshot, Error> {
        let mut staged = self.create_staged_table(
            request.min,
            request.max,
            request.extension,
            &request.table_bytes,
            |staged| self.validate_staged_generation(request.prior, staged, request.list_edit),
        )?;
        observe(PublishStage::TableSynced).map_err(|source| io_error(staged.temp_path.clone(), source))?;
        self.publish_staged(lock, request.prior, &mut staged, request.list_edit, observe)
    }

    fn publish_staged(
        &self,
        mut lock: gix_lock::File,
        prior: &Snapshot,
        staged: &mut StagedTable,
        list_edit: ListEdit,
        mut observe: impl FnMut(PublishStage) -> std::io::Result<()>,
    ) -> Result<Snapshot, Error> {
        let generation = self.edited_generation(prior, &staged.file_name, list_edit)?;
        let entries = parse_list(&self.list_path(), &generation)?;
        self.validate_edited_generation_limits(prior, staged, list_edit)?;
        std::fs::rename(&staged.temp_path, &staged.final_path)
            .map_err(|source| io_error(staged.final_path.clone(), source))?;
        staged.persisted = true;
        sync_directory(&self.directory).map_err(|source| io_error(self.directory.clone(), source))?;
        observe(PublishStage::TablePublished).map_err(|source| io_error(staged.final_path.clone(), source))?;
        let published = self.open_generation(generation.clone(), entries)?;

        lock.write_all(&generation)
            .map_err(|source| io_error(lock.lock_path().to_owned(), source))?;
        lock.with_mut(|file| file.sync_all())
            .map_err(|source| io_error(lock.lock_path().to_owned(), source))?;
        observe(PublishStage::ListSynced).map_err(|source| io_error(lock.lock_path().to_owned(), source))?;
        let list_path = self.list_path();
        lock.commit().map_err(|err| io_error(list_path.clone(), err.error))?;
        if let Err(source) = sync_directory(&self.directory) {
            return Err(Error::Committed {
                snapshot: Box::new(published),
                path: self.directory.clone(),
                source,
            });
        }
        if let Err(source) = observe(PublishStage::ListPublished) {
            return Err(Error::Committed {
                snapshot: Box::new(published),
                path: list_path,
                source,
            });
        }
        Ok(published)
    }

    fn validate_edited_generation_limits(
        &self,
        prior: &Snapshot,
        staged: &StagedTable,
        list_edit: ListEdit,
    ) -> Result<(), Error> {
        let retained = retained_members(prior, list_edit)?;
        let table_size = retained
            .iter()
            .try_fold(staged.file_size, |total, member| total.checked_add(member.file_size));
        if table_size.is_none_or(|size| size > self.snapshot_options.max_total_table_size) {
            return Err(Error::Limit("stack table bytes"));
        }
        self.validate_edited_record_limit(prior, staged.record_count, list_edit)
    }

    fn validate_edited_record_limit(
        &self,
        prior: &Snapshot,
        new_record_count: usize,
        list_edit: ListEdit,
    ) -> Result<(), Error> {
        let record_count = retained_members(prior, list_edit)?
            .iter()
            .try_fold(new_record_count, |total, member| total.checked_add(member.record_count));
        if record_count.is_none_or(|count| count > self.snapshot_options.max_total_records) {
            return Err(Error::Limit("stack record count"));
        }
        Ok(())
    }

    fn validate_staged_generation(
        &self,
        prior: &Snapshot,
        staged: &StagedTable,
        list_edit: ListEdit,
    ) -> Result<(), Error> {
        self.edited_generation(prior, &staged.file_name, list_edit)?;
        self.validate_edited_generation_limits(prior, staged, list_edit)
    }

    fn edited_generation(&self, prior: &Snapshot, staged_name: &str, list_edit: ListEdit) -> Result<Vec<u8>, Error> {
        let mut names = match list_edit {
            ListEdit::Append => prior
                .members
                .iter()
                .map(|member| member.file_name.as_str())
                .collect::<Vec<_>>(),
            ListEdit::ReplacePrefix(count) => {
                if count > prior.members.len() {
                    return Err(Error::InvalidInput("replacement prefix exceeds the current stack"));
                }
                prior.members[count..]
                    .iter()
                    .map(|member| member.file_name.as_str())
                    .collect::<Vec<_>>()
            }
        };
        match list_edit {
            ListEdit::Append => names.push(staged_name),
            ListEdit::ReplacePrefix(_) => names.insert(0, staged_name),
        }
        let size = names
            .iter()
            .try_fold(0usize, |size, name| size.checked_add(name.len())?.checked_add(1));
        let size = size.ok_or(Error::Limit("tables.list size"))?;
        if size > self.snapshot_options.max_list_size {
            return Err(Error::Limit("tables.list size"));
        }
        let mut generation = Vec::new();
        generation
            .try_reserve_exact(size)
            .map_err(|_| Error::Limit("tables.list size"))?;
        for name in names {
            generation.extend_from_slice(name.as_bytes());
            generation.push(b'\n');
        }
        Ok(generation)
    }

    fn create_staged_table(
        &self,
        min: u64,
        max: u64,
        extension: &str,
        bytes: &[u8],
        mut validate: impl FnMut(&StagedTable) -> Result<(), Error>,
    ) -> Result<StagedTable, Error> {
        let table = Table::from_bytes(bytes, self.limits)?;
        if table.header().min_update_index != min || table.header().max_update_index != max {
            return Err(Error::InvalidInput("staged table range differs from its header"));
        }
        if table.header().object_hash != self.object_hash {
            return Err(Error::InvalidInput("staged table object hash differs from the stack"));
        }
        if extension == "log" && table.refs().next().is_some() {
            return Err(Error::InvalidInput("a .log member cannot contain reference records"));
        }
        let record_count = table
            .refs()
            .len()
            .checked_add(table.logs().len())
            .ok_or(Error::Limit("table record count"))?;
        for _ in 0..128 {
            let nonce = unique_nonce();
            let file_name = format!("0x{min:016x}-0x{max:016x}-{nonce:016x}.{extension}");
            let final_path = self.directory.join(&file_name);
            if final_path.exists() {
                continue;
            }
            let temp_path = self.directory.join(format!(".{file_name}.tmp"));
            let staged = StagedTable {
                file_name,
                temp_path,
                final_path,
                persisted: false,
                file_size: bytes.len(),
                record_count,
            };
            validate(&staged)?;
            let mut file = match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&staged.temp_path)
            {
                Ok(file) => file,
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(source) => return Err(io_error(staged.temp_path.clone(), source)),
            };
            if let Err(source) = file.write_all(bytes).and_then(|_| file.sync_all()) {
                drop(file);
                return Err(cleanup_after_failed_write(staged.temp_path.clone(), source, |path| {
                    std::fs::remove_file(path)
                }));
            }
            drop(file);
            return Ok(staged);
        }
        Err(Error::InvalidInput("could not allocate a unique table filename"))
    }
}

impl LockedSnapshot {
    /// Return the stable generation protected for this session's lifetime.
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
}

impl Snapshot {
    /// Return the exact bytes read from this generation's `tables.list`.
    pub fn generation(&self) -> &[u8] {
        &self.generation
    }

    /// Return listed members from oldest to newest.
    pub fn members(&self) -> &[MemberInfo] {
        &self.members
    }

    /// Iterate visible references in bytewise name order.
    pub fn refs(&self) -> impl ExactSizeIterator<Item = &RefRecord> {
        self.refs.iter()
    }

    /// Find one visible reference by exact name.
    pub fn find_ref(&self, name: &[u8]) -> Option<&RefRecord> {
        self.refs
            .binary_search_by(|record| record.name.as_slice().cmp(name))
            .ok()
            .map(|index| &self.refs[index])
    }

    /// Iterate all visible log records by name and descending update index.
    pub fn logs(&self) -> impl ExactSizeIterator<Item = &LogRecord> {
        self.logs.iter()
    }

    /// Collect visible log records for `name`, newest first.
    pub fn logs_for(&self, name: &[u8]) -> Vec<&LogRecord> {
        self.logs
            .iter()
            .filter(|record| record.ref_name.as_slice() == name)
            .collect()
    }

    /// Iterate visible log records, including empty-reflog placeholders.
    ///
    /// Log tombstones are already applied and are not returned. Most callers
    /// should use [`Self::logs`]; transaction adapters need this lower-level
    /// view to replace or delete every historical log key.
    pub fn log_records(&self) -> impl ExactSizeIterator<Item = &LogRecord> {
        self.log_records.iter()
    }

    /// Collect visible log records for `name`, including an empty-reflog placeholder.
    pub fn log_records_for(&self, name: &[u8]) -> Vec<&LogRecord> {
        self.log_records
            .iter()
            .filter(|record| record.ref_name.as_slice() == name)
            .collect()
    }

    /// Return whether `name` has a reflog, including an explicitly empty one.
    pub fn reflog_exists(&self, name: &[u8]) -> bool {
        self.reflogs.iter().any(|candidate| candidate.as_slice() == name)
    }

    /// Iterate names that have a reflog in bytewise order.
    pub fn reflogs(&self) -> impl ExactSizeIterator<Item = &BString> {
        self.reflogs.iter()
    }
}

impl LockedAddition {
    /// Return the immutable generation protected by the held list lock.
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }

    /// Return the update index reserved for this addition.
    pub fn next_update_index(&self) -> u64 {
        self.next_update_index
    }

    /// Write and atomically append one immutable transaction table.
    pub fn commit(self, refs: &[RefRecord], logs: &[LogRecord]) -> Result<Snapshot, Error> {
        if refs.is_empty() && logs.is_empty() {
            return Err(Error::InvalidInput("an addition must contain at least one record"));
        }
        if refs.iter().any(|record| record.update_index < self.next_update_index) {
            return Err(Error::InvalidInput(
                "reference records cannot predate the lock-selected update index",
            ));
        }
        let LockedAddition {
            stack,
            snapshot,
            lock,
            next_update_index,
        } = self;
        let record_count = refs
            .len()
            .checked_add(logs.len())
            .ok_or(Error::Limit("stack record count"))?;
        stack.validate_edited_record_limit(&snapshot, record_count, ListEdit::Append)?;
        let max_update_index = refs
            .iter()
            .map(|record| record.update_index)
            .chain(
                logs.iter()
                    .map(|record| record.update_index)
                    .filter(|index| *index >= next_update_index),
            )
            .max()
            .unwrap_or(next_update_index);
        let bytes = stack.write_table(refs, logs, (next_update_index, max_update_index))?;
        let extension = if refs.is_empty() { "log" } else { "ref" };
        stack.publish(
            lock,
            PublishRequest {
                prior: &snapshot,
                table_bytes: bytes,
                min: next_update_index,
                max: max_update_index,
                extension,
                list_edit: ListEdit::Append,
            },
            |_| Ok(()),
        )
    }
}

#[derive(Debug, Clone)]
struct ListEntry {
    file_name: String,
    min: u64,
    max: u64,
    extension: String,
    line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PublishStage {
    TableSynced,
    TablePublished,
    ListSynced,
    ListPublished,
}

struct PublishRequest<'a> {
    prior: &'a Snapshot,
    table_bytes: Vec<u8>,
    min: u64,
    max: u64,
    extension: &'a str,
    list_edit: ListEdit,
}

#[derive(Debug, Clone, Copy)]
enum ListEdit {
    Append,
    ReplacePrefix(usize),
}

fn retained_members(prior: &Snapshot, list_edit: ListEdit) -> Result<&[MemberInfo], Error> {
    match list_edit {
        ListEdit::Append => Ok(prior.members.as_slice()),
        ListEdit::ReplacePrefix(count) => prior
            .members
            .get(count..)
            .ok_or(Error::InvalidInput("replacement prefix exceeds the current stack")),
    }
}

struct StagedTable {
    file_name: String,
    temp_path: PathBuf,
    final_path: PathBuf,
    persisted: bool,
    file_size: usize,
    record_count: usize,
}

impl Drop for StagedTable {
    fn drop(&mut self) {
        if !self.persisted {
            match std::fs::remove_file(&self.temp_path) {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(_err) => gix_features::trace::warn!(
                    path = %self.temp_path.display(),
                    error = %_err,
                    "failed to remove an unpublished staged reftable"
                ),
            }
        }
    }
}

fn parse_list(path: &Path, data: &[u8]) -> Result<Vec<ListEntry>, Error> {
    let mut entries = Vec::new();
    let mut names = BTreeSet::new();
    let mut lines = data.split(|byte| *byte == b'\n').peekable();
    let mut line = 0;
    while let Some(raw_line) = lines.next() {
        line += 1;
        if raw_line.is_empty() && lines.peek().is_none() {
            continue;
        }
        let line_bytes = raw_line.strip_suffix(b"\r").unwrap_or(raw_line);
        if line_bytes.is_empty() {
            return Err(invalid_list(path, line, "blank lines are not allowed"));
        }
        let file_name =
            std::str::from_utf8(line_bytes).map_err(|_| invalid_list(path, line, "member names must be UTF-8"))?;
        if file_name.contains(['/', '\\']) || Path::new(file_name).file_name() != Some(OsStr::new(file_name)) {
            return Err(invalid_list(path, line, "member names must be safe basenames"));
        }
        if !names.insert(file_name.to_owned()) {
            return Err(invalid_list(path, line, "member names must be unique"));
        }
        let (stem, extension) = file_name
            .rsplit_once('.')
            .ok_or_else(|| invalid_list(path, line, "member name has no extension"))?;
        if !matches!(extension, "ref" | "log") {
            return Err(invalid_list(path, line, "member extension must be .ref or .log"));
        }
        let mut parts = stem.splitn(3, '-');
        let min = parse_hex(parts.next(), path, line)?;
        let max = parse_hex(parts.next(), path, line)?;
        let unique = parts
            .next()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| invalid_list(path, line, "member name has no unique suffix"))?;
        if unique.bytes().any(|byte| !byte.is_ascii_alphanumeric()) {
            return Err(invalid_list(path, line, "member suffix is not alphanumeric"));
        }
        if min > max {
            return Err(invalid_list(path, line, "member minimum exceeds its maximum"));
        }
        entries.push(ListEntry {
            file_name: file_name.to_owned(),
            min,
            max,
            extension: extension.to_owned(),
            line,
        });
    }
    Ok(entries)
}

fn parse_hex(value: Option<&str>, path: &Path, line: usize) -> Result<u64, Error> {
    let original = value.ok_or_else(|| invalid_list(path, line, "member range is not a hexadecimal u64"))?;
    let value = original
        .strip_prefix("0x")
        .or_else(|| original.strip_prefix("0X"))
        .unwrap_or(original);
    let value = (!value.is_empty() && value.len() <= 16)
        .then_some(value)
        .ok_or_else(|| invalid_list(path, line, "member range is not a hexadecimal u64"))?;
    u64::from_str_radix(value, 16).map_err(|_| invalid_list(path, line, "member range is not a hexadecimal u64"))
}

fn merge_tables(tables: &[Table]) -> (Vec<RefRecord>, Vec<LogRecord>, Vec<LogRecord>, BTreeSet<BString>) {
    let mut refs = BTreeMap::<BString, RefRecord>::new();
    let mut logs = BTreeMap::<(BString, Reverse<u64>), LogRecord>::new();
    for table in tables {
        for record in table.refs() {
            refs.insert(record.name.clone(), record.clone());
        }
        for record in table.logs() {
            logs.insert((record.ref_name.clone(), Reverse(record.update_index)), record.clone());
        }
    }
    let refs = refs
        .into_values()
        .filter(|record| !matches!(record.value, RefValue::Deletion))
        .collect();
    let visible_logs = logs
        .into_values()
        .filter(|record| !matches!(record.value, LogValue::Deletion))
        .collect::<Vec<_>>();
    let reflogs = visible_logs
        .iter()
        .map(|record| record.ref_name.clone())
        .collect::<BTreeSet<_>>();
    let logs = visible_logs
        .iter()
        .filter(|record| matches!(record.value, LogValue::Update { .. }))
        .cloned()
        .collect();
    (refs, logs, visible_logs, reflogs)
}

fn retain_logs(logs: &[LogRecord], options: CompactOptions) -> Vec<LogRecord> {
    let Some(expire_before) = options.expire_logs_before else {
        return logs.to_vec();
    };
    let mut seen = BTreeMap::<BString, usize>::new();
    logs.iter()
        .filter(|record| {
            let count = seen.entry(record.ref_name.clone()).or_default();
            *count += 1;
            if *count <= options.keep_latest_logs {
                return true;
            }
            match &record.value {
                LogValue::Update { time, .. } => *time >= expire_before,
                LogValue::Deletion | LogValue::Placeholder => false,
            }
        })
        .cloned()
        .collect()
}

fn unique_nonce() -> u64 {
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos() as u64);
    time ^ u64::from(std::process::id()).rotate_left(17) ^ UNIQUE_COUNTER.fetch_add(1, Ordering::Relaxed)
}

fn table_name_range(file_name: &str) -> Option<(u64, u64)> {
    let (stem, extension) = file_name.rsplit_once('.')?;
    if !matches!(extension, "ref" | "log") {
        return None;
    }
    let mut parts = stem.splitn(3, '-');
    let min = parse_table_name_hex(parts.next()?)?;
    let max = parse_table_name_hex(parts.next()?)?;
    let unique = parts.next()?;
    if unique.is_empty() || unique.bytes().any(|byte| !byte.is_ascii_alphanumeric()) || min > max {
        return None;
    }
    Some((min, max))
}

fn parse_table_name_hex(value: &str) -> Option<u64> {
    let value = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    if value.is_empty() || value.len() > 16 {
        return None;
    }
    u64::from_str_radix(value, 16).ok()
}

fn is_staged_table(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(OsStr::to_str) else {
        return false;
    };
    let Some(table_name) = file_name.strip_prefix('.').and_then(|name| name.strip_suffix(".tmp")) else {
        return false;
    };
    if table_name_range(table_name).is_none() {
        return false;
    }
    let Some((stem, _)) = table_name.rsplit_once('.') else {
        return false;
    };
    let mut parts = stem.splitn(3, '-');
    let exact_hex = |value: Option<&str>| {
        value.is_some_and(|value| {
            value.len() == 18 && value.starts_with("0x") && value[2..].bytes().all(|byte| byte.is_ascii_hexdigit())
        })
    };
    exact_hex(parts.next())
        && exact_hex(parts.next())
        && parts
            .next()
            .is_some_and(|nonce| nonce.len() == 16 && nonce.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

#[derive(Debug, Default)]
struct RemovalOutcome {
    removed: Vec<PathBuf>,
    retained: Vec<PathBuf>,
    failures: Vec<CleanupFailure>,
}

fn remove_paths(paths: Vec<PathBuf>) -> RemovalOutcome {
    remove_paths_with(paths, |path| std::fs::remove_file(path))
}

fn remove_paths_with(paths: Vec<PathBuf>, mut remove: impl FnMut(&Path) -> std::io::Result<()>) -> RemovalOutcome {
    let mut outcome = RemovalOutcome::default();
    for path in paths {
        match remove(&path) {
            Ok(()) => outcome.removed.push(path),
            Err(error) => {
                outcome.failures.push(CleanupFailure {
                    path: path.clone(),
                    error_kind: error.kind(),
                    message: error.to_string(),
                });
                outcome.retained.push(path);
            }
        }
    }
    outcome
}

fn sync_directory(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::fs::File::open(path)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

fn invalid_list(path: &Path, line: usize, message: &'static str) -> Error {
    Error::InvalidList {
        path: path.to_owned(),
        line,
        message,
    }
}

fn io_error(path: PathBuf, source: std::io::Error) -> Error {
    Error::Io { path, source }
}

fn is_missing_member(error: &Error) -> bool {
    match error {
        Error::Io { source, .. } | Error::Table(crate::Error::Io { source, .. }) => {
            source.kind() == std::io::ErrorKind::NotFound
        }
        _ => false,
    }
}

fn cleanup_after_failed_write(
    path: PathBuf,
    source: std::io::Error,
    remove: impl FnOnce(&Path) -> std::io::Result<()>,
) -> Error {
    match remove(&path) {
        Ok(()) => io_error(path, source),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => io_error(path, source),
        Err(cleanup) => Error::StagedTableCleanup { path, source, cleanup },
    }
}

#[cfg(test)]
mod tests {
    use std::{io, time::Duration};

    use bstr::BString;
    use gix_hash::{Kind, ObjectId};

    use super::*;

    fn record(index: u64, byte: u8) -> RefRecord {
        RefRecord {
            name: BString::from("refs/heads/main"),
            update_index: index,
            value: RefValue::Direct(ObjectId::from([byte; 20])),
        }
    }

    #[test]
    fn snapshot_retries_when_compaction_removes_a_member_from_the_captured_generation()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let stack = Stack::create(
            temp.path().join("reftable"),
            Kind::Sha1,
            SnapshotOptions::default(),
            Limits::default(),
        )?;
        let lock_options = LockOptions {
            timeout: Duration::from_secs(1),
        };
        stack.begin_addition(lock_options)?.commit(&[record(1, 1)], &[])?;
        stack.begin_addition(lock_options)?.commit(&[record(2, 2)], &[])?;

        let mut raced = false;
        let snapshot = stack.snapshot_with_observer(|_| {
            if !raced {
                raced = true;
                stack.compact(CompactOptions::default(), lock_options)?;
            }
            Ok(())
        })?;

        assert!(raced, "the test removes the captured members before they are opened");
        assert_eq!(
            snapshot.members().len(),
            1,
            "the retry observes the complete compacted generation"
        );
        assert_eq!(
            snapshot
                .find_ref(b"refs/heads/main")
                .expect("the compacted ref remains visible")
                .update_index,
            2,
            "the retry does not expose a stale or partial generation"
        );
        Ok(())
    }

    #[test]
    fn sharing_violations_defer_cleanup_without_stopping_other_deletions() {
        let blocked = PathBuf::from("held-open.ref");
        let removable = PathBuf::from("closed.ref");
        let mut attempts = Vec::new();

        let RemovalOutcome {
            removed,
            retained,
            failures,
        } = remove_paths_with(vec![blocked.clone(), removable.clone()], |path| {
            attempts.push(path.to_owned());
            if path == blocked {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "injected Windows sharing violation",
                ))
            } else {
                Ok(())
            }
        });

        assert_eq!(
            attempts,
            vec![blocked.clone(), removable.clone()],
            "cleanup attempts every obsolete member even when an open file cannot be deleted"
        );
        assert_eq!(
            removed,
            vec![removable],
            "members that are not held open are still removed"
        );
        assert_eq!(
            retained,
            vec![blocked.clone()],
            "a sharing violation leaves the obsolete member available for a later cleanup pass"
        );
        assert_eq!(failures.len(), 1, "the sharing violation remains diagnosable");
        assert_eq!(failures[0].path, blocked, "the failure identifies the retained path");
        assert_eq!(
            failures[0].error_kind,
            io::ErrorKind::PermissionDenied,
            "the failure retains its portable error category"
        );
        assert!(
            failures[0].message.contains("injected Windows sharing violation"),
            "the failure retains the underlying diagnostic"
        );
    }

    #[test]
    fn staged_write_errors_retain_a_subsequent_cleanup_failure() {
        let path = PathBuf::from("partial-table.tmp");
        let error = cleanup_after_failed_write(
            path.clone(),
            io::Error::new(io::ErrorKind::WriteZero, "injected staged write failure"),
            |_| {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "injected cleanup failure",
                ))
            },
        );

        match error {
            Error::StagedTableCleanup {
                path: actual_path,
                source,
                cleanup,
            } => {
                assert_eq!(actual_path, path, "the error identifies the partial staged table");
                assert_eq!(
                    source.kind(),
                    io::ErrorKind::WriteZero,
                    "the original staged write error remains the source"
                );
                assert_eq!(
                    cleanup.kind(),
                    io::ErrorKind::PermissionDenied,
                    "the cleanup error remains available for diagnosis"
                );
            }
            other => panic!("both failures should produce the combined cleanup error, got {other}"),
        }
    }

    #[test]
    fn every_publication_failure_point_leaves_a_complete_generation() -> Result<(), Box<dyn std::error::Error>> {
        for failed_stage in [
            PublishStage::TableSynced,
            PublishStage::TablePublished,
            PublishStage::ListSynced,
            PublishStage::ListPublished,
        ] {
            let temp = tempfile::tempdir()?;
            let stack = Stack::create(
                temp.path().join("reftable"),
                Kind::Sha1,
                SnapshotOptions::default(),
                Limits::default(),
            )?;
            stack
                .begin_addition(LockOptions {
                    timeout: Duration::ZERO,
                })?
                .commit(&[record(1, 1)], &[])?;

            let list_path = stack.list_path();
            let lock =
                gix_lock::File::acquire_to_update_resource(&list_path, gix_lock::acquire::Fail::Immediately, None)?;
            let prior = stack.snapshot()?;
            let bytes = stack.write_table(&[record(2, 2)], &[], (2, 2))?;
            let result = stack.publish(
                lock,
                PublishRequest {
                    prior: &prior,
                    table_bytes: bytes,
                    min: 2,
                    max: 2,
                    extension: "ref",
                    list_edit: ListEdit::Append,
                },
                |stage| {
                    if stage == failed_stage {
                        Err(io::Error::other("injected publication failure"))
                    } else {
                        Ok(())
                    }
                },
            );
            let error = result.expect_err("the selected stage reports its injected failure");
            if failed_stage == PublishStage::ListPublished {
                assert!(
                    matches!(&error, Error::Committed { .. }),
                    "a post-commit observer failure has an explicit committed outcome"
                );
                assert_eq!(
                    error
                        .committed_snapshot()
                        .and_then(|snapshot| snapshot.find_ref(b"refs/heads/main"))
                        .map(|record| record.update_index),
                    Some(2),
                    "the error retains the exact committed generation so callers must not retry"
                );
            } else {
                assert!(
                    error.committed_snapshot().is_none(),
                    "pre-commit failures remain distinguishable and retryable"
                );
            }

            let visible = stack.snapshot()?;
            let expected = if failed_stage == PublishStage::ListPublished {
                2
            } else {
                1
            };
            assert_eq!(
                visible.find_ref(b"refs/heads/main").map(|record| record.update_index),
                Some(expected),
                "readers see the complete old or complete new generation after {failed_stage:?}"
            );
            stack.cleanup_abandoned(LockOptions {
                timeout: Duration::ZERO,
            })?;
        }
        Ok(())
    }

    #[test]
    fn compaction_revalidates_and_retains_an_append_that_wins_the_unlocked_race()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let stack = Stack::create(
            temp.path().join("reftable"),
            Kind::Sha1,
            SnapshotOptions::default(),
            Limits::default(),
        )?;
        let lock_options = LockOptions {
            timeout: Duration::from_secs(1),
        };
        stack.begin_addition(lock_options)?.commit(&[record(1, 1)], &[])?;
        stack.begin_addition(lock_options)?.commit(&[record(2, 2)], &[])?;

        let appending_stack = stack.clone();
        let outcome = stack.compact_with_observer(CompactOptions::default(), lock_options, move || {
            let addition = appending_stack.begin_addition(lock_options)?;
            if addition.next_update_index() != 3 {
                return Err(Error::InvalidInput(
                    "the racing writer selected an unexpected update index",
                ));
            }
            addition.commit(&[record(3, 3)], &[])?;
            Ok(())
        })?;

        assert_eq!(
            outcome.snapshot.members.len(),
            2,
            "the compacted prefix precedes the racing append"
        );
        assert_eq!(
            outcome
                .snapshot
                .find_ref(b"refs/heads/main")
                .map(|record| record.update_index),
            Some(3),
            "the racing writer remains authoritative"
        );
        assert_eq!(
            outcome.removed.len(),
            2,
            "the successful retry removes both superseded members"
        );
        Ok(())
    }
}

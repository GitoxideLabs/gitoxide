use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use gix_object::bstr::{BStr, BString, ByteSlice};

use crate::{Category, FullName, FullNameRef, Namespace, Reference, Target, store::WriteReflog};

mod snapshot;
pub(crate) mod transaction;

#[derive(Debug, Clone)]
pub(crate) struct Store {
    git_dir: PathBuf,
    common_dir: Option<PathBuf>,
    object_hash: gix_hash::Kind,
    pub(crate) write_reflog: WriteReflog,
    precompose_unicode: bool,
    prohibit_windows_device_names: bool,
    pub(crate) namespace: Option<Namespace>,
    snapshot_options: gix_reftable::SnapshotOptions,
    limits: gix_reftable::Limits,
    main: gix_reftable::Stack,
    current: Option<gix_reftable::Stack>,
}

pub(crate) struct Snapshot<'store> {
    store: &'store Store,
    main: gix_reftable::Snapshot,
    current: Option<gix_reftable::Snapshot>,
    other: gix_features::threading::Mutable<BTreeMap<PathBuf, gix_reftable::Snapshot>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum StackLocation {
    Main,
    Current,
    Other(BString),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorktreeContext {
    Current,
    Main,
    Other(BString),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StackRole {
    Main,
    Worktree,
}

struct MaintenanceStack {
    stack: gix_reftable::Stack,
    role: StackRole,
}

#[derive(Debug, Clone)]
pub(crate) struct Route {
    pub(crate) location: StackLocation,
    pub(crate) context: WorktreeContext,
    pub(crate) public_name: FullName,
    pub(crate) local_name: FullName,
    pub(crate) stored_name: FullName,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error(transparent)]
    Stack(#[from] gix_reftable::StackError),
    #[error("reftable record contains invalid reference name {name:?}")]
    InvalidName {
        name: BString,
        #[source]
        source: crate::name::Error,
    },
    #[error("a visible stack snapshot unexpectedly exposed a deletion for {name:?}")]
    UnexpectedDeletion { name: BString },
    #[error("reftable transaction preprocessing failed")]
    Preprocess(#[source] std::io::Error),
    #[error("a transaction contains multiple edits for reftable name {name:?}")]
    DuplicateEdit { name: BString },
    #[error("reference {name:?} conflicts with reference {conflicting:?}")]
    NameConflict { name: BString, conflicting: BString },
    #[error("reftable record {name:?} in worktree stack {path} is not worktree-private")]
    MisplacedWorktreeRecord { name: BString, path: PathBuf },
    #[error("reference {name:?} was required to exist")]
    MustExist { name: BString },
    #[error("reference {name:?} was required not to exist")]
    MustNotExist { name: BString },
    #[error("reference {name:?} has value {actual}, expected {expected}")]
    OutOfDate {
        name: BString,
        expected: Target,
        actual: Target,
    },
    #[error("MustNotExist is not a valid deletion predicate for {name:?}")]
    InvalidDeletePredicate { name: BString },
    #[error("a reflog update requires a committer")]
    MissingCommitter,
    #[error("reflog identity contains an invalid byte")]
    InvalidIdentity,
    #[error("reflog messages must not contain newlines")]
    InvalidLogMessage,
    #[error("reflog timestamp cannot be represented by reftable")]
    InvalidLogTime,
    #[error("an object needed to peel reference {name:?} could not be read")]
    PeelObject {
        name: BString,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },
    #[error("object {object_id} needed to peel reference {name:?} does not exist")]
    MissingObject {
        name: BString,
        object_id: gix_hash::ObjectId,
    },
    #[error("tag object {object_id} referenced by {name:?} is malformed")]
    MalformedTag {
        name: BString,
        object_id: gix_hash::ObjectId,
    },
    #[error("a reftable transaction was already prepared")]
    AlreadyPrepared,
    #[error("a reftable transaction was not prepared")]
    NotPrepared,
    #[error("reftable transaction routing did not stabilize")]
    RoutingDidNotStabilize,
    #[error("reftable reference {name:?} has invalid symbolic target {target:?}")]
    InvalidSymbolicTarget {
        name: BString,
        target: BString,
        #[source]
        source: crate::name::Error,
    },
    #[error("could not enumerate linked-worktree reftable storage at {path}")]
    EnumerateWorktrees {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("unsafe linked-worktree reftable storage at {path}: {reason}")]
    UnsafeWorktreeStorage { path: PathBuf, reason: &'static str },
    #[error("illegal use of reserved Windows device name in worktree name {name:?}")]
    WindowsDeviceWorktreeName { name: BString },
}

impl Store {
    pub(crate) fn create(
        git_dir: PathBuf,
        object_hash: gix_hash::Kind,
        initial_head: FullName,
        mut opts: crate::store::init::Options,
    ) -> Result<Self, Error> {
        let snapshot_options = gix_reftable::SnapshotOptions::default();
        let limits = gix_reftable::Limits::default();
        let main = gix_reftable::Stack::create(git_dir.join("reftable"), object_hash, snapshot_options, limits)?;
        let requested_write_reflog = opts.write_reflog;
        opts.write_reflog = WriteReflog::Disable;
        let mut store = Store {
            git_dir,
            common_dir: None,
            object_hash,
            write_reflog: opts.write_reflog,
            precompose_unicode: opts.precompose_unicode,
            prohibit_windows_device_names: opts.prohibit_windows_device_names,
            namespace: None,
            snapshot_options,
            limits,
            main,
            current: None,
        };
        transaction::Transaction::new(&store)
            .prepare(
                [crate::transaction::RefEdit {
                    change: crate::transaction::Change::Update {
                        log: crate::transaction::LogChange {
                            mode: crate::transaction::RefLog::AndReference,
                            force_create_reflog: false,
                            message: BString::default(),
                        },
                        expected: crate::transaction::PreviousValue::MustNotExist,
                        new: Target::Symbolic(initial_head),
                    },
                    name: "HEAD".try_into().expect("HEAD is always a valid reference name"),
                    deref: false,
                }],
                gix_lock::acquire::Fail::Immediately,
                gix_lock::acquire::Fail::Immediately,
            )?
            .commit(None)?;
        store.write_reflog = requested_write_reflog;
        Ok(store)
    }

    pub(crate) fn open(
        git_dir: PathBuf,
        common_dir: Option<PathBuf>,
        object_hash: gix_hash::Kind,
        opts: crate::store::init::Options,
    ) -> Result<Self, Error> {
        let snapshot_options = gix_reftable::SnapshotOptions::default();
        let limits = gix_reftable::Limits::default();
        let common = common_dir.as_deref().unwrap_or(&git_dir);
        let main = gix_reftable::Stack::open(common.join("reftable"), object_hash, snapshot_options, limits)?;
        let current = common_dir
            .as_ref()
            .map(|_| gix_reftable::Stack::open(git_dir.join("reftable"), object_hash, snapshot_options, limits))
            .transpose()?;
        Ok(Store {
            git_dir,
            common_dir,
            object_hash,
            write_reflog: opts.write_reflog,
            precompose_unicode: opts.precompose_unicode,
            prohibit_windows_device_names: opts.prohibit_windows_device_names,
            namespace: None,
            snapshot_options,
            limits,
            main,
            current,
        })
    }

    pub(crate) fn git_dir(&self) -> &Path {
        &self.git_dir
    }

    pub(crate) fn common_dir(&self) -> Option<&Path> {
        self.common_dir.as_deref()
    }

    pub(crate) fn common_dir_resolved(&self) -> &Path {
        self.common_dir.as_deref().unwrap_or(&self.git_dir)
    }

    pub(crate) fn object_hash(&self) -> gix_hash::Kind {
        self.object_hash
    }

    pub(crate) fn snapshot(&self) -> Result<Snapshot<'_>, Error> {
        Ok(Snapshot {
            store: self,
            main: self.main.snapshot()?,
            current: self.current.as_ref().map(gix_reftable::Stack::snapshot).transpose()?,
            other: gix_features::threading::Mutable::default(),
        })
    }

    pub(crate) fn verify(&self) -> Result<(), Error> {
        for maintenance in self.maintenance_stacks()? {
            self.verify_stack(&maintenance)?;
        }
        Ok(())
    }

    fn verify_stack(&self, maintenance: &MaintenanceStack) -> Result<(), Error> {
        let stack = &maintenance.stack;
        let attempts = self.snapshot_options.max_attempts.max(1);
        for _ in 0..attempts {
            let snapshot = stack.snapshot()?;
            let result = self.verify_stack_snapshot(stack, &snapshot, maintenance.role);
            let generation_is_current = stack.generation_is_current(&snapshot)?;
            match result {
                Ok(()) if generation_is_current => return Ok(()),
                Err(err) if generation_is_current => return Err(err),
                Ok(()) | Err(_) => std::thread::yield_now(),
            }
        }
        Err(gix_reftable::StackError::ConcurrentModification {
            path: stack.directory().join("tables.list"),
            attempts,
        }
        .into())
    }

    fn verify_stack_snapshot(
        &self,
        stack: &gix_reftable::Stack,
        snapshot: &gix_reftable::Snapshot,
        role: StackRole,
    ) -> Result<(), Error> {
        let visible_names = snapshot
            .refs()
            .map(|record| record.name.clone())
            .collect::<BTreeSet<_>>();
        validate_name_conflicts(&visible_names)?;
        for member in snapshot.members() {
            let table = gix_reftable::Table::read(stack.directory().join(&member.file_name), self.limits)
                .map_err(gix_reftable::StackError::from)?;
            for record in table.refs() {
                self.verify_record_name(stack, &record.name, role)?;
                if let gix_reftable::RefValue::Symbolic(target) = &record.value {
                    crate::FullName::try_from(target.clone()).map_err(|source| Error::InvalidSymbolicTarget {
                        name: record.name.clone(),
                        target: target.clone(),
                        source,
                    })?;
                }
            }
            for record in table.logs() {
                self.verify_record_name(stack, &record.ref_name, role)?;
            }
        }
        Ok(())
    }

    fn verify_record_name(&self, stack: &gix_reftable::Stack, name: &BString, role: StackRole) -> Result<(), Error> {
        crate::FullName::try_from(name.clone()).map_err(|source| Error::InvalidName {
            name: name.clone(),
            source,
        })?;
        if role == StackRole::Worktree && worktree_privacy(name.as_ref()) != Some(true) {
            return Err(Error::MisplacedWorktreeRecord {
                name: name.clone(),
                path: stack.directory().to_owned(),
            });
        }
        Ok(())
    }

    pub(crate) fn optimize(
        &self,
        options: crate::store::maintenance::Options,
        lock_fail: gix_lock::acquire::Fail,
    ) -> Result<(), Error> {
        let lock_options = gix_reftable::LockOptions {
            timeout: match lock_fail {
                gix_lock::acquire::Fail::Immediately => std::time::Duration::ZERO,
                gix_lock::acquire::Fail::AfterDurationWithBackoff(duration) => duration,
            },
        };
        let compact_options = gix_reftable::CompactOptions {
            expire_logs_before: options.expire_reflogs_before,
            keep_latest_logs: options.keep_latest_reflog_entries,
        };
        for maintenance in self.maintenance_stacks()? {
            maintenance.stack.compact(compact_options, lock_options)?;
            if options.cleanup_abandoned {
                maintenance.stack.cleanup_abandoned(lock_options)?;
            }
        }
        Ok(())
    }

    fn maintenance_stacks(&self) -> Result<Vec<MaintenanceStack>, Error> {
        let mut stacks = BTreeMap::<PathBuf, MaintenanceStack>::new();
        stacks.insert(
            self.main.directory().to_owned(),
            MaintenanceStack {
                stack: self.main.clone(),
                role: StackRole::Main,
            },
        );
        if let Some(current) = &self.current {
            stacks
                .entry(current.directory().to_owned())
                .or_insert_with(|| MaintenanceStack {
                    stack: current.clone(),
                    role: StackRole::Worktree,
                });
        }

        let Some(worktrees_dir) = self.canonical_worktrees_dir()? else {
            return Ok(stacks.into_values().collect());
        };
        let entries = std::fs::read_dir(&worktrees_dir).map_err(|source| Error::EnumerateWorktrees {
            path: worktrees_dir.clone(),
            source,
        })?;
        for entry in entries {
            let entry = entry.map_err(|source| Error::EnumerateWorktrees {
                path: worktrees_dir.clone(),
                source,
            })?;
            let file_type = entry.file_type().map_err(|source| Error::EnumerateWorktrees {
                path: entry.path(),
                source,
            })?;
            if file_type.is_symlink() {
                return Err(Error::UnsafeWorktreeStorage {
                    path: entry.path(),
                    reason: "the worktree entry is a symbolic link",
                });
            }
            if !file_type.is_dir() {
                continue;
            }
            let path = entry.path().join("reftable");
            let Some(path) = self.canonical_stack_path(&worktrees_dir, path)? else {
                continue;
            };
            let stack = gix_reftable::Stack::open(path.clone(), self.object_hash, self.snapshot_options, self.limits)?;
            stacks.entry(path).or_insert(MaintenanceStack {
                stack,
                role: StackRole::Worktree,
            });
        }
        Ok(stacks.into_values().collect())
    }

    fn canonical_worktrees_dir(&self) -> Result<Option<PathBuf>, Error> {
        let path = self.common_dir_resolved().join("worktrees");
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(Error::EnumerateWorktrees { path, source }),
        };
        if metadata.file_type().is_symlink() {
            return Err(Error::UnsafeWorktreeStorage {
                path,
                reason: "the worktrees directory is a symbolic link",
            });
        }
        if !metadata.is_dir() {
            return Err(Error::UnsafeWorktreeStorage {
                path,
                reason: "the worktrees path is not a directory",
            });
        }
        std::fs::canonicalize(&path)
            .map(Some)
            .map_err(|source| Error::EnumerateWorktrees { path, source })
    }

    fn canonical_stack_path(&self, worktrees_dir: &Path, path: PathBuf) -> Result<Option<PathBuf>, Error> {
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(Error::EnumerateWorktrees { path, source }),
        };
        if metadata.file_type().is_symlink() {
            return Err(Error::UnsafeWorktreeStorage {
                path,
                reason: "the reftable stack is a symbolic link",
            });
        }
        if !metadata.is_dir() {
            return Err(Error::UnsafeWorktreeStorage {
                path,
                reason: "the reftable stack is not a directory",
            });
        }
        let canonical = std::fs::canonicalize(&path).map_err(|source| Error::EnumerateWorktrees {
            path: path.clone(),
            source,
        })?;
        if !canonical.starts_with(worktrees_dir) {
            return Err(Error::UnsafeWorktreeStorage {
                path: canonical,
                reason: "the reftable stack resolves outside the worktrees directory",
            });
        }
        Ok(Some(canonical))
    }

    fn other_stack_path(&self, name: &BStr) -> Result<PathBuf, Error> {
        if self.prohibit_windows_device_names && gix_validate::path::component_is_windows_device(name) {
            return Err(Error::WindowsDeviceWorktreeName { name: name.to_owned() });
        }
        let name = if self.precompose_unicode {
            gix_utils::str::precompose_bstr(Cow::Borrowed(name))
        } else {
            Cow::Borrowed(name)
        };
        let component = gix_path::from_bstr(name.as_ref());
        let mut components = component.components();
        if !matches!(components.next(), Some(std::path::Component::Normal(_))) || components.next().is_some() {
            return Err(Error::UnsafeWorktreeStorage {
                path: component.into_owned(),
                reason: "the worktree name is not one path component",
            });
        }
        let Some(worktrees_dir) = self.canonical_worktrees_dir()? else {
            return Err(Error::UnsafeWorktreeStorage {
                path: self.common_dir_resolved().join("worktrees"),
                reason: "the worktrees directory does not exist",
            });
        };
        let worktree_dir = worktrees_dir.join(component.as_ref());
        let metadata = std::fs::symlink_metadata(&worktree_dir).map_err(|source| Error::EnumerateWorktrees {
            path: worktree_dir.clone(),
            source,
        })?;
        if metadata.file_type().is_symlink() {
            return Err(Error::UnsafeWorktreeStorage {
                path: worktree_dir,
                reason: "the worktree entry is a symbolic link",
            });
        }
        if !metadata.is_dir() {
            return Err(Error::UnsafeWorktreeStorage {
                path: worktree_dir,
                reason: "the worktree entry is not a directory",
            });
        }
        let path = worktree_dir.join("reftable");
        self.canonical_stack_path(&worktrees_dir, path)?
            .ok_or_else(|| Error::UnsafeWorktreeStorage {
                path: worktree_dir.join("reftable"),
                reason: "the reftable stack does not exist",
            })
    }

    pub(crate) fn stack(&self, location: &StackLocation) -> Result<gix_reftable::Stack, Error> {
        match location {
            StackLocation::Main => Ok(self.main.clone()),
            StackLocation::Current => Ok(self.current.as_ref().unwrap_or(&self.main).clone()),
            StackLocation::Other(name) => Ok(gix_reftable::Stack::open(
                self.other_stack_path(name.as_ref())?,
                self.object_hash,
                self.snapshot_options,
                self.limits,
            )?),
        }
    }

    pub(crate) fn stack_path(&self, location: &StackLocation) -> Result<PathBuf, Error> {
        match location {
            StackLocation::Main => Ok(self.main.directory().to_owned()),
            StackLocation::Current => Ok(self.current.as_ref().unwrap_or(&self.main).directory().to_owned()),
            StackLocation::Other(name) => self.other_stack_path(name.as_ref()),
        }
    }

    pub(crate) fn route(&self, name: &FullNameRef) -> Route {
        let public_name = name.to_owned();
        let (location, context, local_name) = match name.category_and_short_name() {
            Some((Category::MainPseudoRef | Category::MainRef, short)) => (
                StackLocation::Main,
                WorktreeContext::Main,
                FullNameRef::new_unchecked(short).to_owned(),
            ),
            Some((Category::LinkedPseudoRef { name: worktree }, short)) => (
                StackLocation::Other(worktree.to_owned()),
                WorktreeContext::Other(worktree.to_owned()),
                FullNameRef::new_unchecked(short).to_owned(),
            ),
            Some((Category::LinkedRef { name: worktree }, short)) => {
                let short = FullNameRef::new_unchecked(short);
                if short.category().is_some_and(|category| category.is_worktree_private()) {
                    (
                        StackLocation::Other(worktree.to_owned()),
                        WorktreeContext::Other(worktree.to_owned()),
                        short.to_owned(),
                    )
                } else {
                    (StackLocation::Main, WorktreeContext::Current, short.to_owned())
                }
            }
            Some((category, _)) if category.is_worktree_private() => (
                if self.current.is_some() {
                    StackLocation::Current
                } else {
                    StackLocation::Main
                },
                WorktreeContext::Current,
                name.to_owned(),
            ),
            _ => (StackLocation::Main, WorktreeContext::Current, name.to_owned()),
        };
        let mut stored_name = local_name.clone();
        if let Some(namespace) = &self.namespace {
            stored_name.prefix_namespace(namespace);
        }
        Route {
            location,
            context,
            public_name,
            local_name,
            stored_name,
        }
    }

    pub(crate) fn reference_from_record(
        &self,
        record: &gix_reftable::RefRecord,
        public_name: FullName,
        context: &WorktreeContext,
    ) -> Result<Reference, Error> {
        let (target, peeled_object_id) = match &record.value {
            gix_reftable::RefValue::Deletion => {
                return Err(Error::UnexpectedDeletion {
                    name: record.name.clone(),
                });
            }
            gix_reftable::RefValue::Direct(target_object_id) => (Target::Object(*target_object_id), None),
            gix_reftable::RefValue::Peeled {
                target: target_object_id,
                peeled: peeled_object_id,
            } => (Target::Object(*target_object_id), Some(*peeled_object_id)),
            gix_reftable::RefValue::Symbolic(target) => {
                let mut target = FullName::try_from(target.clone()).map_err(|source| Error::InvalidName {
                    name: target.clone(),
                    source,
                })?;
                if let Some(namespace) = &self.namespace {
                    target.strip_namespace(namespace);
                }
                target = qualify_private_name(target, context)?;
                (Target::Symbolic(target), None)
            }
        };
        Ok(Reference {
            name: public_name,
            target,
            peeled: peeled_object_id,
        })
    }
}

fn validate_name_conflicts(names: &BTreeSet<BString>) -> Result<(), Error> {
    for name in names {
        for separator in name
            .iter()
            .enumerate()
            .filter_map(|(index, byte)| (*byte == b'/').then_some(index))
        {
            let ancestor = name[..separator].as_bstr();
            if let Some(conflicting) = names.get(ancestor) {
                return Err(Error::NameConflict {
                    name: name.clone(),
                    conflicting: conflicting.clone(),
                });
            }
        }
    }
    Ok(())
}

fn worktree_privacy(mut name: &BStr) -> Option<bool> {
    while let Some(namespaced) = name.strip_prefix(b"refs/namespaces/") {
        let separator = namespaced.find_byte(b'/')?;
        name = namespaced[separator + 1..].as_bstr();
    }
    FullName::try_from(name.to_owned())
        .ok()
        .map(|name| name.category().is_some_and(|category| category.is_worktree_private()))
}

fn qualify_private_name(name: FullName, context: &WorktreeContext) -> Result<FullName, Error> {
    if !name.category().is_some_and(|category| category.is_worktree_private()) {
        return Ok(name);
    }
    let mut qualified = BString::default();
    match context {
        WorktreeContext::Current => return Ok(name),
        WorktreeContext::Main => qualified.extend_from_slice(b"main-worktree/"),
        WorktreeContext::Other(worktree) => {
            qualified.extend_from_slice(b"worktrees/");
            qualified.extend_from_slice(worktree);
            qualified.push(b'/');
        }
    }
    qualified.extend_from_slice(name.as_bstr());
    FullName::try_from(qualified.clone()).map_err(|source| Error::InvalidName {
        name: qualified,
        source,
    })
}

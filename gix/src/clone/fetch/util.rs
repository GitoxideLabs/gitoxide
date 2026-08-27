use std::io::Write;

use gix_ref::{
    Category, FullNameRef, PartialName,
    transaction::{LogChange, RefLog},
};

use super::Error;
use crate::{
    Repository,
    bstr::{BStr, BString, ByteSlice},
};

#[expect(
    clippy::result_large_err,
    reason = "will be removed once `gix-error` is used consistently"
)]
pub fn upsert_remote_in_local_config(
    remote: &mut crate::Remote<'_>,
    remote_name: BString,
) -> Result<gix_config::File, Error> {
    let config_path = remote.repo.git_dir().join("config");
    let mut local_config = gix_config::File::from_path_no_includes(config_path.clone(), gix_config::Source::Local)?;
    let mut resolved_config = remote.repo.config.resolved.as_ref().clone();
    remote.save_as_to(remote_name.clone(), &mut local_config)?;
    remote.save_as_to(remote_name, &mut resolved_config)?;

    let mut lock =
        gix_lock::File::acquire_to_update_resource(&config_path, gix_lock::acquire::Fail::Immediately, None)?;
    local_config.write_to(&mut lock)?;
    lock.with_mut(|file| file.sync_all())?;
    lock.commit()?;
    Ok(resolved_config)
}

/// Reconfigure the freshly initialized repository `repo` to use `object_hash` and reopen it.
///
/// A files repository has no hash-dependent reference data at this point. A reftable repository
/// contains only its symbolic `HEAD`; replace that pristine seed stack with one encoded for the
/// negotiated hash before any fetched reference is written. The replacement passes through an
/// empty, hash-independent stack generation so every crash point pairs the on-disk configuration
/// with either matching reference data or no reference data at all.
/// File contents are synchronized on every platform. Directory-entry durability across a sudden
/// power loss is additionally synchronized on Unix; portable Rust offers no equivalent guarantee
/// on other targets, where the same logical publication sequence remains atomic but metadata flushes
/// are best-effort.
///
/// Existing local configuration, including the remote section written during clone setup, is
/// preserved. The returned repository is reopened so all hash-dependent state uses the new format.
#[cfg(feature = "sha256")]
pub(super) fn reinitialize_with_object_hash(
    repo: &crate::Repository,
    object_hash: gix_hash::Kind,
) -> Result<crate::Repository, Error> {
    let git_dir = repo.git_dir();
    #[cfg(all(test, feature = "blocking-network-client"))]
    if take_incomplete_handoff_rollback_injection() {
        return Err(with_rollback(
            handoff_io(
                "inject a reftable object-format handoff failure",
                git_dir,
                std::io::Error::other("injected handoff failure"),
            ),
            Err(handoff_io(
                "inject the corresponding reftable handoff rollback failure",
                git_dir,
                std::io::Error::other("injected rollback failure"),
            )),
        ));
    }
    let config_path = git_dir.join("config");
    let original_config = std::fs::read(&config_path)
        .map_err(|source| handoff_io("read the original repository configuration", &config_path, source))?;

    let mut config = gix_config::File::from_path_no_includes(config_path.clone(), gix_config::Source::Local)?;
    let is_sha256 = object_hash == gix_hash::Kind::Sha256;
    let is_reftable = config
        .string("extensions.refStorage")
        .is_some_and(|value| value == b"reftable");
    config
        .section_mut("core", None)
        .expect("freshly initialized repository has a core section")
        .set(
            "repositoryformatversion",
            if is_sha256 || is_reftable { "1" } else { "0" },
        )?;
    if is_sha256 {
        config
            .section_mut_or_create_new("extensions", None)
            .expect("valid section name")
            .set("objectformat", object_hash.to_string())?;
    } else if let Ok(mut extensions) = config.section_mut("extensions", None) {
        while extensions.remove("objectformat").is_some() {}
    }
    let mut updated_config = Vec::new();
    config.write_to_filter(&mut updated_config, |section| {
        section.meta().source == gix_config::Source::Local
    })?;

    if let Some(mut handoff) = is_reftable
        .then(|| ReftableHandoff::new(repo, object_hash))
        .transpose()?
    {
        handoff.detach_previous_generation()?;
        if let Err(failure) = publish_resource(
            &config_path,
            &updated_config,
            "publish the negotiated repository configuration",
        ) {
            let rollback = if failure.published {
                handoff.rollback_after_config_publication(&config_path, &original_config)
            } else {
                handoff.restore_previous_generation()
            };
            return Err(with_rollback(failure.error, rollback));
        }
        if let Err(original) = handoff.publish_new_generation() {
            let rollback = handoff.rollback_after_config_publication(&config_path, &original_config);
            return Err(with_rollback(original, rollback));
        }
        let reopened = match crate::ThreadSafeRepository::open_opts(git_dir, repo.options.clone()) {
            Ok(repo) => repo.to_thread_local(),
            Err(source) => {
                let rollback = handoff.rollback_after_config_publication(&config_path, &original_config);
                return Err(with_rollback(Error::ReopenWithObjectHash(source), rollback));
            }
        };
        handoff.finish();
        return Ok(reopened);
    } else if let Err(failure) = publish_resource(
        &config_path,
        &updated_config,
        "publish the negotiated repository configuration",
    ) {
        return Err(failure.error);
    }

    Ok(crate::ThreadSafeRepository::open_opts(git_dir, repo.options.clone())?.to_thread_local())
}

#[cfg(feature = "sha256")]
struct CleanupDirectory {
    path: std::path::PathBuf,
}

#[cfg(feature = "sha256")]
impl Drop for CleanupDirectory {
    fn drop(&mut self) {
        match std::fs::remove_dir_all(&self.path) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(_err) => gix_trace::warn!(
                "could not remove reftable handoff staging directory at '{}': {_err}",
                self.path.display()
            ),
        }
    }
}

#[cfg(feature = "sha256")]
struct ReftableHandoff {
    _cleanup: CleanupDirectory,
    list_lock: gix_lock::Marker,
    canonical_directory: std::path::PathBuf,
    staged_directory: std::path::PathBuf,
    previous_list: Vec<u8>,
    previous_members: Vec<String>,
    new_list: Vec<u8>,
    new_members: Vec<String>,
    installed_members: Vec<std::path::PathBuf>,
}

#[cfg(feature = "sha256")]
impl ReftableHandoff {
    fn new(repo: &crate::Repository, object_hash: gix_hash::Kind) -> Result<Self, Error> {
        Self::new_with_observer(repo, object_hash, || {})
    }

    fn new_with_observer(
        repo: &crate::Repository,
        object_hash: gix_hash::Kind,
        observe_pristine: impl FnOnce(),
    ) -> Result<Self, Error> {
        let canonical_directory = repo.git_dir().join("reftable");
        let previous_list_path = canonical_directory.join("tables.list");
        let list_lock =
            gix_lock::Marker::acquire_to_hold_resource(&previous_list_path, gix_lock::acquire::Fail::Immediately, None)
                .map_err(|source| Error::ReftableHandoffLock {
                    source,
                    operation: "protect the pristine stack generation",
                    path: previous_list_path.clone(),
                })?;
        let head = repo.refs.find("HEAD")?;
        let initial_head = head
            .target
            .try_name()
            .map(ToOwned::to_owned)
            .ok_or(Error::ReftableHeadNotSymbolic)?;
        if repo
            .refs
            .is_pristine(initial_head.as_ref())
            .map_err(|source| Error::InspectReftablePristine { source })?
            != Some(true)
        {
            return Err(Error::ReftableNotPristine);
        }
        observe_pristine();

        let cleanup = CleanupDirectory {
            // Keep hard-crash debris out of the repository. Ordinary failures and
            // catchable termination still remove this directory through `Drop`.
            path: unique_reftable_stage()?,
        };
        let staged_git_dir = cleanup.path.join("new");
        std::fs::create_dir(&staged_git_dir)
            .map_err(|source| handoff_io("create a staging directory", &staged_git_dir, source))?;
        drop(gix_ref::Store::create_reftable(
            staged_git_dir.clone(),
            object_hash,
            initial_head,
        )?);

        let staged_directory = staged_git_dir.join("reftable");
        let new_list_path = staged_directory.join("tables.list");
        let previous_list = std::fs::read(&previous_list_path)
            .map_err(|source| handoff_io("read the previous stack generation", &previous_list_path, source))?;
        let new_list = std::fs::read(&new_list_path)
            .map_err(|source| handoff_io("read the negotiated stack generation", &new_list_path, source))?;
        let previous_members = listed_members(&previous_list, &previous_list_path)?;
        let new_members = listed_members(&new_list, &new_list_path)?;
        Ok(ReftableHandoff {
            _cleanup: cleanup,
            list_lock,
            canonical_directory,
            staged_directory,
            previous_list,
            previous_members,
            new_list,
            new_members,
            installed_members: Vec::new(),
        })
    }

    fn detach_previous_generation(&mut self) -> Result<(), Error> {
        let list_path = self.canonical_directory.join("tables.list");
        match publish_locked_resource(&self.list_lock, &list_path, &[], "publish an empty stack generation") {
            Ok(()) => Ok(()),
            Err(failure) if failure.published => {
                let original = failure.error;
                let rollback = self.restore_previous_generation();
                Err(with_rollback(original, rollback))
            }
            Err(failure) => Err(failure.error),
        }
    }

    fn install_new_members(&mut self) -> Result<(), Error> {
        for member in &self.new_members {
            let source_path = self.staged_directory.join(member);
            let target_path = self.canonical_directory.join(member);
            let mut source = std::fs::File::open(&source_path)
                .map_err(|source| handoff_io("open a staged stack member", &source_path, source))?;
            let mut target = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target_path)
                .map_err(|source| handoff_io("create a negotiated stack member", &target_path, source))?;
            self.installed_members.push(target_path.clone());
            std::io::copy(&mut source, &mut target)
                .and_then(|_| target.sync_all())
                .map_err(|source| handoff_io("copy and synchronize a negotiated stack member", &target_path, source))?;
        }
        sync_directory(&self.canonical_directory).map_err(|source| {
            handoff_io(
                "synchronize negotiated stack members",
                &self.canonical_directory,
                source,
            )
        })
    }

    fn publish_new_generation(&mut self) -> Result<(), Error> {
        self.install_new_members()?;
        let list_path = self.canonical_directory.join("tables.list");
        publish_locked_resource(
            &self.list_lock,
            &list_path,
            &self.new_list,
            "publish the negotiated stack generation",
        )
        .map_err(|failure| failure.error)
    }

    fn restore_previous_generation(&mut self) -> Result<(), Error> {
        let list_path = self.canonical_directory.join("tables.list");
        publish_locked_resource(
            &self.list_lock,
            &list_path,
            &self.previous_list,
            "restore the previous stack generation",
        )
        .map_err(|failure| failure.error)
    }

    fn rollback_after_config_publication(
        &mut self,
        config_path: &std::path::Path,
        original_config: &[u8],
    ) -> Result<(), Error> {
        self.rollback_after_config_publication_with(config_path, original_config, |_, _| Ok(()))
    }

    fn rollback_after_config_publication_with(
        &mut self,
        config_path: &std::path::Path,
        original_config: &[u8],
        mut after_publish: impl FnMut(&std::path::Path, &'static str) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let list_path = self.canonical_directory.join("tables.list");
        let operation = "restore an empty stack generation before rollback";
        let mut rollback_failure = match observe_publication(
            publish_locked_resource(&self.list_lock, &list_path, &[], operation),
            &list_path,
            operation,
            &mut after_publish,
        ) {
            Ok(()) => None,
            Err(failure) if failure.published => Some(failure.error),
            Err(failure) => return Err(failure.error),
        };
        if let Err(failure) = self.remove_installed_members() {
            rollback_failure = Some(combine_failures(rollback_failure, failure));
        }
        let operation = "restore the original repository configuration";
        match observe_publication(
            publish_resource(config_path, original_config, operation),
            config_path,
            operation,
            &mut after_publish,
        ) {
            Ok(()) => {}
            Err(failure) if failure.published => {
                rollback_failure = Some(combine_failures(rollback_failure, failure.error));
            }
            Err(failure) => return Err(combine_failures(rollback_failure, failure.error)),
        }
        let operation = "restore the previous stack generation";
        if let Err(failure) = observe_publication(
            publish_locked_resource(&self.list_lock, &list_path, &self.previous_list, operation),
            &list_path,
            operation,
            &mut after_publish,
        ) {
            rollback_failure = Some(combine_failures(rollback_failure, failure.error));
        }
        match rollback_failure {
            Some(failure) => Err(failure),
            None => Ok(()),
        }
    }

    fn remove_installed_members(&mut self) -> Result<(), Error> {
        let mut failure = None;
        for path in self.installed_members.drain(..) {
            match std::fs::remove_file(&path) {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(source) => {
                    let error = handoff_io("remove a negotiated stack member during rollback", &path, source);
                    failure = Some(combine_failures(failure, error));
                }
            }
        }
        match failure {
            Some(failure) => Err(failure),
            None => Ok(()),
        }
    }

    fn finish(self) {
        for member in &self.previous_members {
            if !self.new_members.contains(member) {
                let path = self.canonical_directory.join(member);
                match std::fs::remove_file(&path) {
                    Ok(()) => {}
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                    Err(_err) => gix_trace::warn!(
                        "could not remove obsolete reftable stack member at '{}': {_err}",
                        path.display()
                    ),
                }
            }
        }
    }
}

#[cfg(feature = "sha256")]
fn unique_reftable_stage() -> Result<std::path::PathBuf, Error> {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_STAGE: AtomicU64 = AtomicU64::new(0);
    let stage_root = std::env::temp_dir();
    let started_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    for _ in 0..16 {
        let path = stage_root.join(format!(
            "gix-reftable-stage-{}-{started_at}-{}",
            std::process::id(),
            NEXT_STAGE.fetch_add(1, Ordering::Relaxed)
        ));
        match std::fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(source) => return Err(handoff_io("create a unique staging directory", &path, source)),
        }
    }
    let path = stage_root.join("gix-reftable-stage");
    Err(handoff_io(
        "create a unique staging directory",
        &path,
        std::io::Error::new(std::io::ErrorKind::AlreadyExists, "all staging names were occupied"),
    ))
}

#[cfg(feature = "sha256")]
fn handoff_io(operation: &'static str, path: &std::path::Path, source: std::io::Error) -> Error {
    Error::ReftableHandoffIo {
        source,
        operation,
        path: path.to_owned(),
    }
}

#[cfg(feature = "sha256")]
struct PublicationFailure {
    error: Error,
    published: bool,
}

#[cfg(feature = "sha256")]
fn publish_resource(path: &std::path::Path, bytes: &[u8], operation: &'static str) -> Result<(), PublicationFailure> {
    let mut lock = gix_lock::File::acquire_to_update_resource(path, gix_lock::acquire::Fail::Immediately, None)
        .map_err(|source| PublicationFailure {
            error: Error::ReftableHandoffLock {
                source,
                operation,
                path: path.to_owned(),
            },
            published: false,
        })?;
    lock.write_all(bytes)
        .and_then(|_| lock.with_mut(|file| file.sync_all()))
        .map_err(|source| PublicationFailure {
            error: handoff_io(operation, lock.lock_path(), source),
            published: false,
        })?;
    lock.commit().map_err(|err| PublicationFailure {
        error: handoff_io(operation, path, err.error),
        published: false,
    })?;
    let parent = path.parent().expect("a repository resource always has a parent");
    sync_directory(parent).map_err(|source| PublicationFailure {
        error: handoff_io(operation, path, source),
        published: true,
    })?;
    Ok(())
}

#[cfg(feature = "sha256")]
fn publish_locked_resource(
    lock: &gix_lock::Marker,
    path: &std::path::Path,
    bytes: &[u8],
    operation: &'static str,
) -> Result<(), PublicationFailure> {
    debug_assert_eq!(
        lock.resource_path(),
        path,
        "the marker must lock the resource being published"
    );
    let parent = path.parent().expect("a repository resource always has a parent");
    let permissions = std::fs::metadata(path)
        .map(|metadata| metadata.permissions())
        .map_err(|source| PublicationFailure {
            error: handoff_io(operation, path, source),
            published: false,
        })?;
    let mut tempfile = gix_tempfile::new(
        parent,
        gix_tempfile::ContainingDirectory::Exists,
        gix_tempfile::AutoRemove::Tempfile,
    )
    .map_err(|source| PublicationFailure {
        error: handoff_io(operation, parent, source),
        published: false,
    })?;
    tempfile
        .with_mut(|file| file.as_file_mut().set_permissions(permissions))
        .and_then(|result| result)
        .and_then(|_| tempfile.write_all(bytes))
        .and_then(|_| {
            tempfile
                .with_mut(|file| file.as_file_mut().sync_all())
                .and_then(|result| result)
        })
        .map_err(|source| PublicationFailure {
            error: handoff_io(operation, path, source),
            published: false,
        })?;
    tempfile.persist(path).map_err(|err| PublicationFailure {
        error: handoff_io(operation, path, err.error),
        published: false,
    })?;
    sync_directory(parent).map_err(|source| PublicationFailure {
        error: handoff_io(operation, path, source),
        published: true,
    })?;
    Ok(())
}

#[cfg(feature = "sha256")]
fn observe_publication(
    publication: Result<(), PublicationFailure>,
    path: &std::path::Path,
    operation: &'static str,
    observer: &mut impl FnMut(&std::path::Path, &'static str) -> Result<(), Error>,
) -> Result<(), PublicationFailure> {
    publication?;
    observer(path, operation).map_err(|error| PublicationFailure { error, published: true })
}

#[cfg(feature = "sha256")]
fn listed_members(bytes: &[u8], list_path: &std::path::Path) -> Result<Vec<String>, Error> {
    let text = std::str::from_utf8(bytes).map_err(|source| {
        handoff_io(
            "validate a staged stack generation",
            list_path,
            std::io::Error::new(std::io::ErrorKind::InvalidData, source),
        )
    })?;
    text.lines()
        .map(|name| {
            let path = std::path::Path::new(name);
            if name.is_empty() || name.contains(['/', '\\']) || path.file_name() != Some(std::ffi::OsStr::new(name)) {
                return Err(handoff_io(
                    "validate a staged stack member name",
                    list_path,
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid reftable member name"),
                ));
            }
            Ok(name.to_owned())
        })
        .collect()
}

#[cfg(feature = "sha256")]
fn sync_directory(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::fs::File::open(path)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        // There is no portable directory-fsync operation here. Atomic renames still
        // protect logical readers, but sudden-power-loss durability is best-effort.
        let _ = path;
        Ok(())
    }
}

#[cfg(feature = "sha256")]
fn with_rollback(original: Error, rollback: Result<(), Error>) -> Error {
    match rollback {
        Ok(()) => original,
        Err(rollback) => Error::ReftableHandoffRollback {
            original: Box::new(original),
            rollback: Box::new(rollback),
        },
    }
}

#[cfg(feature = "sha256")]
fn combine_failures(previous: Option<Error>, next: Error) -> Error {
    match previous {
        Some(previous) => with_rollback(previous, Err(next)),
        None => next,
    }
}

#[cfg(all(test, feature = "blocking-network-client"))]
std::thread_local! {
    static INJECT_INCOMPLETE_HANDOFF_ROLLBACK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(all(test, feature = "blocking-network-client"))]
pub(super) struct IncompleteHandoffRollbackInjection;

#[cfg(all(test, feature = "blocking-network-client"))]
impl Drop for IncompleteHandoffRollbackInjection {
    fn drop(&mut self) {
        INJECT_INCOMPLETE_HANDOFF_ROLLBACK.with(|inject| inject.set(false));
    }
}

#[cfg(all(test, feature = "blocking-network-client"))]
pub(super) fn inject_incomplete_handoff_rollback_once() -> IncompleteHandoffRollbackInjection {
    INJECT_INCOMPLETE_HANDOFF_ROLLBACK.with(|inject| {
        assert!(
            !inject.replace(true),
            "only one incomplete handoff rollback injection may be pending per test thread"
        );
    });
    IncompleteHandoffRollbackInjection
}

#[cfg(all(test, feature = "blocking-network-client"))]
fn take_incomplete_handoff_rollback_injection() -> bool {
    INJECT_INCOMPLETE_HANDOFF_ROLLBACK.with(std::cell::Cell::take)
}

#[cfg(all(test, feature = "sha1", feature = "sha256"))]
mod reftable_handoff_tests {
    use super::*;
    use gix_testtools::tempfile;

    fn assert_repository_state(
        git_dir: &std::path::Path,
        expected_hash: gix_hash::Kind,
        expect_head: bool,
        stage: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = crate::ThreadSafeRepository::open_opts(git_dir, crate::open::Options::isolated())?.to_thread_local();
        assert_eq!(
            repo.object_hash(),
            expected_hash,
            "{stage}: the repository configuration selects the expected object hash"
        );
        assert_eq!(
            repo.refs.try_find("HEAD")?.is_some(),
            expect_head,
            "{stage}: the published stack has the expected symbolic HEAD visibility"
        );
        Ok(())
    }

    struct RollbackFixture {
        _temp: tempfile::TempDir,
        git_dir: std::path::PathBuf,
        config_path: std::path::PathBuf,
        original_config: Vec<u8>,
        handoff: ReftableHandoff,
    }

    fn handoff_ready_for_rollback() -> Result<RollbackFixture, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let repo: crate::Repository = crate::ThreadSafeRepository::init(
            temp.path(),
            crate::create::Kind::Bare,
            crate::create::Options {
                object_hash: Some(gix_hash::Kind::Sha1),
                reference_storage: crate::create::ReferenceStorage::Reftable,
                ..Default::default()
            },
        )?
        .into();
        let git_dir = repo.git_dir().to_owned();
        let config_path = git_dir.join("config");
        let original_config = std::fs::read(&config_path)?;
        let mut config = gix_config::File::from_path_no_includes(config_path.clone(), gix_config::Source::Local)?;
        config
            .section_mut("core", None)
            .expect("new repositories have a core section")
            .set("repositoryformatversion", "1")?;
        config
            .section_mut_or_create_new("extensions", None)
            .expect("extensions is a valid section name")
            .set("objectformat", "sha256")?;
        let mut sha256_config = Vec::new();
        config.write_to_filter(&mut sha256_config, |section| {
            section.meta().source == gix_config::Source::Local
        })?;

        let mut handoff = ReftableHandoff::new(&repo, gix_hash::Kind::Sha256)?;
        handoff.detach_previous_generation()?;
        publish_resource(&config_path, &sha256_config, "publish the test configuration")
            .map_err(|failure| failure.error)?;
        handoff.install_new_members()?;
        Ok(RollbackFixture {
            _temp: temp,
            git_dir,
            config_path,
            original_config,
            handoff,
        })
    }

    #[test]
    fn every_handoff_stage_is_hash_compatible() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let repo: crate::Repository = crate::ThreadSafeRepository::init(
            temp.path(),
            crate::create::Kind::Bare,
            crate::create::Options {
                object_hash: Some(gix_hash::Kind::Sha1),
                reference_storage: crate::create::ReferenceStorage::Reftable,
                ..Default::default()
            },
        )?
        .into();
        let git_dir = repo.git_dir().to_owned();
        let config_path = git_dir.join("config");
        let mut config = gix_config::File::from_path_no_includes(config_path.clone(), gix_config::Source::Local)?;
        config
            .section_mut("core", None)
            .expect("new repositories have a core section")
            .set("repositoryformatversion", "1")?;
        config
            .section_mut_or_create_new("extensions", None)
            .expect("extensions is a valid section name")
            .set("objectformat", "sha256")?;
        let mut sha256_config = Vec::new();
        config.write_to_filter(&mut sha256_config, |section| {
            section.meta().source == gix_config::Source::Local
        })?;

        let mut handoff = ReftableHandoff::new(&repo, gix_hash::Kind::Sha256)?;
        assert_eq!(
            handoff._cleanup.path.parent(),
            Some(std::env::temp_dir().as_path()),
            "handoff staging lives in the system temporary directory"
        );
        assert!(
            !handoff._cleanup.path.starts_with(&git_dir),
            "a hard crash cannot strand handoff staging inside the repository"
        );
        assert_repository_state(&git_dir, gix_hash::Kind::Sha1, true, "initial generation")?;

        handoff.detach_previous_generation()?;
        assert_repository_state(&git_dir, gix_hash::Kind::Sha1, false, "previous generation detached")?;

        publish_resource(&config_path, &sha256_config, "publish the test configuration")
            .map_err(|failure| failure.error)?;
        assert_repository_state(&git_dir, gix_hash::Kind::Sha256, false, "new configuration published")?;

        handoff.install_new_members()?;
        assert_repository_state(
            &git_dir,
            gix_hash::Kind::Sha256,
            false,
            "new members installed but unlisted",
        )?;

        let list_path = git_dir.join("reftable/tables.list");
        publish_locked_resource(
            &handoff.list_lock,
            &list_path,
            &handoff.new_list,
            "publish the test stack generation",
        )
        .map_err(|failure| failure.error)?;
        assert_repository_state(&git_dir, gix_hash::Kind::Sha256, true, "new generation published")?;
        handoff.finish();
        Ok(())
    }

    #[test]
    fn pristine_validation_is_covered_by_the_authoritative_list_lock() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let repo: crate::Repository = crate::ThreadSafeRepository::init(
            temp.path(),
            crate::create::Kind::Bare,
            crate::create::Options {
                object_hash: Some(gix_hash::Kind::Sha1),
                reference_storage: crate::create::ReferenceStorage::Reftable,
                ..Default::default()
            },
        )?
        .into();
        let list_path = repo.git_dir().join("reftable/tables.list");

        let _handoff = ReftableHandoff::new_with_observer(&repo, gix_hash::Kind::Sha256, || {
            let competing_writer =
                gix_lock::File::acquire_to_update_resource(&list_path, gix_lock::acquire::Fail::Immediately, None);
            assert!(
                matches!(
                    competing_writer,
                    Err(gix_lock::acquire::Error::PermanentlyLocked { .. })
                ),
                "the authoritative list lock must cover the pristine check and subsequent handoff"
            );
        })?;
        Ok(())
    }

    #[test]
    fn rollback_reports_member_cleanup_failure_after_restoring_logical_state() -> Result<(), Box<dyn std::error::Error>>
    {
        let temp = tempfile::tempdir()?;
        let repo: crate::Repository = crate::ThreadSafeRepository::init(
            temp.path(),
            crate::create::Kind::Bare,
            crate::create::Options {
                object_hash: Some(gix_hash::Kind::Sha1),
                reference_storage: crate::create::ReferenceStorage::Reftable,
                ..Default::default()
            },
        )?
        .into();
        let git_dir = repo.git_dir().to_owned();
        let config_path = git_dir.join("config");
        let original_config = std::fs::read(&config_path)?;
        let mut config = gix_config::File::from_path_no_includes(config_path.clone(), gix_config::Source::Local)?;
        config
            .section_mut("core", None)
            .expect("new repositories have a core section")
            .set("repositoryformatversion", "1")?;
        config
            .section_mut_or_create_new("extensions", None)
            .expect("extensions is a valid section name")
            .set("objectformat", "sha256")?;
        let mut sha256_config = Vec::new();
        config.write_to_filter(&mut sha256_config, |section| {
            section.meta().source == gix_config::Source::Local
        })?;

        let mut handoff = ReftableHandoff::new(&repo, gix_hash::Kind::Sha256)?;
        handoff.detach_previous_generation()?;
        publish_resource(&config_path, &sha256_config, "publish the test configuration")
            .map_err(|failure| failure.error)?;
        handoff.install_new_members()?;

        let installed_path = handoff
            .installed_members
            .first()
            .expect("a new reftable stack contains at least one member")
            .clone();
        std::fs::remove_file(&installed_path)?;
        std::fs::create_dir(&installed_path)?;

        let error = handoff
            .rollback_after_config_publication(&config_path, &original_config)
            .expect_err("an installed member that cannot be removed makes rollback incomplete");
        assert!(
            matches!(
                error,
                Error::ReftableHandoffIo {
                    operation: "remove a negotiated stack member during rollback",
                    ..
                }
            ),
            "the cleanup failure identifies the failed rollback operation: {error}"
        );
        assert_repository_state(
            &git_dir,
            gix_hash::Kind::Sha1,
            true,
            "logical state restored despite cleanup failure",
        )?;
        Ok(())
    }

    #[test]
    fn rollback_continues_after_post_commit_directory_sync_failures() -> Result<(), Box<dyn std::error::Error>> {
        for failed_operation in [
            "restore an empty stack generation before rollback",
            "restore the original repository configuration",
        ] {
            let RollbackFixture {
                _temp,
                git_dir,
                config_path,
                original_config,
                mut handoff,
            } = handoff_ready_for_rollback()?;
            let error = handoff
                .rollback_after_config_publication_with(&config_path, &original_config, |path, operation| {
                    if operation == failed_operation {
                        return Err(handoff_io(
                            operation,
                            path,
                            std::io::Error::other("injected post-commit directory sync failure"),
                        ));
                    }
                    Ok(())
                })
                .expect_err("the post-commit durability failure remains visible");
            assert!(
                matches!(
                    error,
                    Error::ReftableHandoffIo { operation, .. } if operation == failed_operation
                ),
                "the reported error identifies the failed durability step: {error}"
            );
            assert_repository_state(
                &git_dir,
                gix_hash::Kind::Sha1,
                true,
                "logical rollback completes after a published durability error",
            )?;
        }
        Ok(())
    }
}

fn overwrite_local_config(config: &gix_config::File) -> std::io::Result<()> {
    assert_eq!(
        config.meta().source,
        gix_config::Source::Local,
        "made for appending to local configuration file"
    );
    let mut local_config = std::fs::OpenOptions::new()
        .create(false)
        .write(true)
        .open(config.meta().path.as_deref().expect("local config with path set"))?;
    local_config.write_all(config.detect_newline_style())?;
    config.write_to_filter(&mut local_config, |s| s.meta().source == gix_config::Source::Local)
}

/// HEAD cannot be written by means of refspec by design, so we have to do it manually here. Also create the pointed-to ref
/// if we have to, as it might not have been naturally included in the ref-specs.
/// Lastly, use `ref_name` if it was provided instead, and let `HEAD` point to it.
pub fn update_head(
    repo: &mut Repository,
    ref_map: &crate::remote::fetch::RefMap,
    reflog_message: &BStr,
    remote_name: &BStr,
    ref_name: Option<&PartialName>,
    revision: Option<&gix_refspec::RefSpec>,
) -> Result<(), Error> {
    use gix_ref::transaction::{PreviousValue, RefEdit};
    let revision_head_id = revision
        .map(|revision| -> Result<gix_hash::ObjectId, Error> {
            let mapping = find_revision(ref_map, revision)?;
            let id = mapping.remote.peeled_id().ok_or_else(|| Error::RevisionMissing {
                wanted: revision.to_ref().source().expect("validated revision").to_owned(),
            })?;
            Ok(repo.find_object(id)?.peel_to_commit()?.id)
        })
        .transpose()?;
    let head_info = match revision_head_id.as_ref() {
        Some(id) => Some((Some(id.as_ref()), None)),
        None => match ref_name {
            Some(ref_name) => {
                let (target, full_ref_name) = find_custom_refname(ref_map, ref_name)?;
                Some((Some(target), Some(full_ref_name)))
            }
            None => ref_map.remote_refs.iter().find_map(|r| {
                Some(match r {
                    gix_protocol::handshake::Ref::Symbolic {
                        full_ref_name,
                        target,
                        tag: _,
                        object,
                    } if full_ref_name == "HEAD" => (Some(object.as_ref()), Some(target.as_bstr())),
                    gix_protocol::handshake::Ref::Direct { full_ref_name, object } if full_ref_name == "HEAD" => {
                        (Some(object.as_ref()), None)
                    }
                    gix_protocol::handshake::Ref::Unborn { full_ref_name, target } if full_ref_name == "HEAD" => {
                        (None, Some(target.as_bstr()))
                    }
                    _ => return None,
                })
            }),
        },
    };
    let Some((head_peeled_id, head_ref)) = head_info else {
        return Ok(());
    };

    let head: gix_ref::FullName = "HEAD".try_into().expect("valid");
    let reflog_message = || LogChange {
        mode: RefLog::AndReference,
        force_create_reflog: false,
        message: reflog_message.to_owned(),
    };
    match head_ref {
        Some(referent) => {
            let referent: gix_ref::FullName = referent.try_into().map_err(|err| Error::InvalidHeadRef {
                head_ref_name: referent.to_owned(),
                source: err,
            })?;
            repo.refs
                .transaction()
                .write_strategy(gix_ref::store::transaction::WriteStrategy::Compact {
                    objects: Box::new(&repo.objects),
                    remove_separate_source: false,
                })
                .prepare(
                    {
                        let mut edits = vec![RefEdit::update_with_log(
                            head.clone(),
                            referent.clone(),
                            PreviousValue::Any,
                            reflog_message(),
                        )];
                        if let Some(head_peeled_id) = head_peeled_id {
                            edits.push(RefEdit::update_with_log(
                                referent.clone(),
                                head_peeled_id.to_owned(),
                                PreviousValue::Any,
                                reflog_message(),
                            ));
                        }
                        edits
                    },
                    gix_lock::acquire::Fail::Immediately,
                    gix_lock::acquire::Fail::Immediately,
                )
                .map_err(crate::reference::edit::Error::from)?
                .commit(
                    repo.committer()
                        .transpose()
                        .map_err(|err| Error::HeadUpdate(crate::reference::edit::Error::ParseCommitterTime(err)))?,
                )
                .map_err(crate::reference::edit::Error::from)?;

            if let Some(head_peeled_id) = head_peeled_id {
                let mut log = reflog_message();
                log.mode = RefLog::Only;
                repo.edit_reference(RefEdit::update_with_log(
                    head,
                    head_peeled_id.to_owned(),
                    PreviousValue::Any,
                    log,
                ))?;
            }

            setup_branch_config(repo, referent.as_ref(), head_peeled_id, remote_name)?;
        }
        None => {
            repo.edit_reference(RefEdit::update_with_log(
                head,
                head_peeled_id
                    .expect("detached heads always point to something")
                    .to_owned(),
                PreviousValue::Any,
                reflog_message(),
            ))?;
        }
    }
    Ok(())
}

/// Find the mapping produced by the exact refspec used to request `revision`.
///
/// Returns [`Error::RevisionMissing`] if the remote did not map that refspec.
pub(super) fn find_revision<'a>(
    ref_map: &'a crate::remote::fetch::RefMap,
    revision: &gix_refspec::RefSpec,
) -> Result<&'a gix_protocol::fetch::refmap::Mapping, Error> {
    ref_map
        .mappings
        .iter()
        .find(|mapping| {
            mapping
                .spec_index
                .get(&ref_map.refspecs, &ref_map.extra_refspecs)
                .is_some_and(|spec| spec == revision)
        })
        .ok_or_else(|| Error::RevisionMissing {
            wanted: revision.to_ref().source().expect("validated revision").to_owned(),
        })
}

/// Resolve `ref_name` to its object ID and full name among the mapped remote references.
///
/// Full names match directly. Partial names prefer branches over tags, then use normal refspec matching.
/// Returns [`Error::RefNameMissing`] or [`Error::RefNameAmbiguous`] when there is no unique match.
pub(super) fn find_custom_refname<'a>(
    ref_map: &'a crate::remote::fetch::RefMap,
    ref_name: &PartialName,
) -> Result<(&'a gix_hash::oid, &'a BStr), Error> {
    let group = gix_refspec::MatchGroup::from_fetch_specs(Some(
        gix_refspec::parse(ref_name.as_ref().as_bstr(), gix_refspec::parse::Operation::Fetch)
            .expect("partial names are valid refs"),
    ));
    let filtered_items: Vec<_> = ref_map
        .mappings
        .iter()
        .filter_map(|m| m.remote.as_name().zip(m.remote.as_id()))
        .map(|(full_ref_name, target)| gix_refspec::match_group::Item {
            full_ref_name,
            target,
            object: None,
        })
        .collect();

    let requested_name = ref_name.as_ref().as_bstr();
    let find_item = |name: &BStr| filtered_items.iter().find(|item| item.full_ref_name == name).copied();
    // Preserve gix's documented full-ref support, then match git clone --branch by trying heads before tags.
    if let Some(item) = find_item(requested_name) {
        return Ok((item.target, item.full_ref_name));
    }
    if !requested_name.starts_with(b"refs/") {
        let branch_name = Category::LocalBranch.to_full_name(requested_name)?;
        if let Some(item) = find_item(branch_name.as_bstr()) {
            return Ok((item.target, item.full_ref_name));
        }

        let tag_name = Category::Tag.to_full_name(requested_name)?;
        if let Some(item) = find_item(tag_name.as_bstr()) {
            return Ok((item.target, item.full_ref_name));
        }
    }

    let res = group.match_lhs(filtered_items.iter().copied());
    match res.mappings.len() {
        0 => Err(Error::RefNameMissing {
            wanted: ref_name.clone(),
        }),
        1 => {
            let item = filtered_items[res.mappings[0]
                .item_index
                .expect("we map by name only and have no object-id in refspec")];
            Ok((item.target, item.full_ref_name))
        }
        _ => Err(Error::RefNameAmbiguous {
            wanted: ref_name.clone(),
            candidates: res
                .mappings
                .into_iter()
                .filter_map(|m| match m.lhs {
                    gix_refspec::match_group::SourceRef::FullName(name) => Some(name.into_owned()),
                    gix_refspec::match_group::SourceRef::ObjectId(_) => None,
                })
                .collect(),
        }),
    }
}

/// Set up the remote configuration for `branch` so that it points to itself, but on the remote, if and only if currently
/// saved refspecs are able to match it.
/// For that we reload the remote of `remote_name` and use its `ref_specs` for match.
fn setup_branch_config(
    repo: &mut Repository,
    branch: &FullNameRef,
    branch_id: Option<&gix_hash::oid>,
    remote_name: &BStr,
) -> Result<(), Error> {
    let short_name = match branch.category_and_short_name() {
        Some((gix_ref::Category::LocalBranch, shortened)) => match shortened.to_str() {
            Ok(s) => s,
            Err(_) => return Ok(()),
        },
        _ => return Ok(()),
    };
    let remote = repo
        .find_remote(remote_name)
        .expect("remote was just created and must be visible in config");
    let group = gix_refspec::MatchGroup::from_fetch_specs(remote.fetch_specs.iter().map(gix_refspec::RefSpec::to_ref));
    let null = gix_hash::ObjectId::null(repo.object_hash());
    let res = group.match_lhs(
        Some(gix_refspec::match_group::Item {
            full_ref_name: branch.as_bstr(),
            target: branch_id.unwrap_or(&null),
            object: None,
        })
        .into_iter(),
    );
    if !res.mappings.is_empty() {
        let mut config = repo.config_snapshot_mut();
        let mut section = config
            .new_section("branch", short_name)
            .expect("section header name is always valid per naming rules, our input branch name is valid");
        section.push("remote", remote_name)?;
        section.push("merge", branch.as_bstr())?;
        overwrite_local_config(&config)?;
        config.commit().expect("configuration we set is valid");
    }
    Ok(())
}

//! A transaction for editing one physical configuration file.

use std::{
    io::Read,
    ops::{Deref, DerefMut},
};

use super::FileTransaction;

/// The error produced when opening or committing a [`FileTransaction`].
#[derive(Debug, thiserror::Error)]
#[expect(missing_docs)]
pub enum Error {
    #[error(transparent)]
    LockTimeout(#[from] super::lock_timeout::Error),
    #[error(transparent)]
    InvalidSharedRepository(#[from] crate::config::key::GenericErrorWithValue),
    #[error("Could not acquire the lock for the configuration file")]
    AcquireLock(#[from] gix_lock::acquire::Error),
    #[error("Could not read metadata of the configuration file at {path:?}")]
    Metadata {
        source: std::io::Error,
        path: std::path::PathBuf,
    },
    #[error("Could not read the configuration file at {path:?}")]
    Read {
        source: std::io::Error,
        path: std::path::PathBuf,
    },
    #[error("Could not parse the configuration file at {path:?}")]
    Parse {
        source: gix_config::file::init::Error,
        path: std::path::PathBuf,
    },
    #[error("Could not write the configuration file at {path:?}")]
    Write {
        source: std::io::Error,
        path: std::path::PathBuf,
    },
    #[error("Could not commit the configuration file lock")]
    CommitLock(#[from] gix_lock::commit::Error<gix_lock::File>),
}

impl FileTransaction {
    pub(crate) fn open(
        path: std::path::PathBuf,
        trust: gix_sec::Trust,
        lock_mode: gix_lock::acquire::Fail,
        shared_repository_permissions: i32,
    ) -> Result<Self, Error> {
        let adjust_permissions =
            |permissions| gix_fs::adjust_shared_repository_permissions(permissions, shared_repository_permissions);
        let adjust_permissions: Option<&dyn Fn(std::fs::Permissions) -> std::fs::Permissions> =
            (shared_repository_permissions != 0).then_some(&adjust_permissions);
        let mut lock = gix_lock::File::acquire(
            &path,
            lock_mode,
            None,
            Some(&gix_lock::acquire::resolve_symlink),
            adjust_permissions,
        )?;
        let source = gix_config::Source::Local;
        let path = lock.resource_path();
        let config = match std::fs::File::open(&path) {
            Ok(mut file) => {
                let permissions = file
                    .metadata()
                    .map_err(|source| Error::Metadata {
                        source,
                        path: path.clone(),
                    })?
                    .permissions();
                lock.with_mut(|file| file.set_permissions(permissions))
                    .map_err(|source| Error::Write {
                        source,
                        path: path.clone(),
                    })?;
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes).map_err(|source| Error::Read {
                    source,
                    path: path.clone(),
                })?;
                gix_config::File::from_bytes_no_includes(
                    &bytes,
                    gix_config::file::Metadata::from(source).at(path.clone()).with(trust),
                    Default::default(),
                )
                .map_err(|source| Error::Parse {
                    source,
                    path: path.clone(),
                })?
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                gix_config::File::new(gix_config::file::Metadata::from(source).at(path).with(trust))
            }
            Err(source) => return Err(Error::Read { source, path }),
        };
        Ok(FileTransaction { lock, config })
    }

    /// Write this physical file atomically without changing any repository instance.
    pub fn commit(mut self) -> Result<(), Error> {
        let path = self.lock.resource_path();
        self.config
            .write_to(&mut self.lock)
            .map_err(|source| Error::Write { source, path })?;
        self.lock.commit()?;
        Ok(())
    }
}

/// Resolve Git's effective `core.sharedRepository` permission policy.
///
/// # Lookup peculiarities
///
/// The normal caller passes the repository's already-resolved configuration snapshot, not the physical file that
/// [`FileTransaction`] is about to edit. The snapshot contains expanded includes and configuration scopes in precedence
/// order. Consequently, changing `core.sharedRepository` through that transaction doesn't affect the permissions of the same
/// transaction; the repository must be reloaded before a later transaction observes the new value.
///
/// Section and key matching is ASCII-case-insensitive. Sections are visited in merged load order, but only when `filter`
/// accepts their metadata and the header is exactly `[core]`, without a subsection. Within each eligible section,
/// [`gix_config::file::SectionRef::value_implicit()`] selects the last `sharedRepository` entry. Sections without such an
/// entry are skipped, and the last value remaining across all eligible sections wins, matching Git's single-value config
/// lookup. Thus a later rejected section, `[core "subsection"]`, or `[core]` section without this key doesn't mask an
/// earlier accepted value.
///
/// An absent key means the default `umask` policy. A bare `sharedRepository` entry is different: it is an implicit boolean
/// `true`, equivalent to `group`.
///
/// Values are parsed by [`crate::config::tree::core::SharedRepository::try_into_shared_repository()`].
pub(crate) fn shared_repository_permissions(
    config: &gix_config::File,
    filter: fn(&gix_config::file::Metadata) -> bool,
) -> Result<i32, Error> {
    let value = config.sections_by_name_and_filter("core", filter).and_then(|sections| {
        sections
            .filter(|section| section.header().subsection_name().is_none())
            .filter_map(|section| section.value_implicit("sharedRepository"))
            .last()
    });
    let Some(value) = value else { return Ok(0) };
    crate::config::tree::Core::SHARED_REPOSITORY
        .try_into_shared_repository(value)
        .map_err(Into::into)
}

impl Deref for FileTransaction {
    type Target = gix_config::File;

    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

impl DerefMut for FileTransaction {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.config
    }
}

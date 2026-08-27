#![allow(clippy::result_large_err)]

use super::{Error, util};
use crate::config::{
    cache::util::{ApplyLeniency, ApplyLeniencyDefaultValue},
    tree::{Core, Extensions, gitoxide},
};

enum ConfigAvailability {
    Available,
    Missing {
        path: std::path::PathBuf,
    },
    Unreadable {
        source: std::io::Error,
        path: std::path::PathBuf,
    },
}

/// A utility to deal with the cyclic dependency between the ref store and the configuration. The ref-store needs the
/// object hash kind, and the configuration needs the current branch name to resolve conditional includes with `onbranch`.
pub(crate) struct StageOne {
    pub git_dir_config: gix_config::File,
    pub buf: Vec<u8>,

    pub is_bare: Option<bool>,
    pub lossy: bool,
    pub object_hash: gix_hash::Kind,
    pub reference_storage: crate::create::ReferenceStorage,
    pub reflog: Option<gix_ref::store::WriteReflog>,
    pub precompose_unicode: bool,
    pub protect_windows: bool,
}

/// Initialization
impl StageOne {
    pub fn new(
        common_dir: &std::path::Path,
        git_dir: &std::path::Path,
        git_dir_trust: gix_sec::Trust,
        lossy: bool,
        lenient: bool,
    ) -> Result<Self, Error> {
        let mut buf = Vec::with_capacity(512);
        let (mut config, config_availability) = load_config(
            common_dir.join("config"),
            &mut buf,
            gix_config::Source::Local,
            git_dir_trust,
            lossy,
            lenient,
        )?;

        let is_bare = util::config_bool_opt(&config, &Core::BARE, "core.bare", lenient)?;
        let repo_format_version = Core::REPOSITORY_FORMAT_VERSION
            .try_into_usize(config.integer("core.repositoryFormatVersion"))?
            .unwrap_or_default();
        let object_format = config.string(Extensions::OBJECT_FORMAT);
        let ref_storage = config.string(Extensions::REF_STORAGE);
        if ref_storage.is_none() {
            match config_availability {
                ConfigAvailability::Available => {}
                ConfigAvailability::Missing { path: config_path } => {
                    if let Some(storage_path) = find_reftable_storage(common_dir, git_dir)? {
                        return Err(Error::ReftableStorageWithoutConfig {
                            config_path,
                            storage_path,
                        });
                    }
                }
                ConfigAvailability::Unreadable {
                    source,
                    path: config_path,
                } => {
                    if let Some(storage_path) = find_reftable_storage(common_dir, git_dir)? {
                        return Err(Error::ReftableStorageWithUnreadableConfig {
                            source,
                            config_path,
                            storage_path,
                        });
                    }
                }
            }
        }
        let (object_hash, reference_storage) = match repo_format_version {
            0 if object_format.is_some() => return Err(Error::ObjectFormatRequiresV1),
            0 if ref_storage.is_some() => return Err(Error::RefStorageRequiresV1),
            0 => (legacy_object_hash()?, crate::create::ReferenceStorage::Files),
            1 => (
                object_format
                    .map(|format| Extensions::OBJECT_FORMAT.try_into_object_format(format))
                    .transpose()?
                    .map_or_else(legacy_object_hash, Ok)?,
                ref_storage
                    .map(|storage| Extensions::REF_STORAGE.try_into_reference_storage(storage))
                    .transpose()?
                    .unwrap_or_default(),
            ),
            version => return Err(Error::UnsupportedRepositoryFormatVersion { version }),
        };

        let extension_worktree = util::config_bool(
            &config,
            &Extensions::WORKTREE_CONFIG,
            "extensions.worktreeConfig",
            false,
            lenient,
        )?;
        if extension_worktree {
            let (worktree_config, _) = load_config(
                git_dir.join("config.worktree"),
                &mut buf,
                gix_config::Source::Worktree,
                git_dir_trust,
                lossy,
                lenient,
            )?;
            config.append(worktree_config)?;
        }
        let precompose_unicode = Core::PRECOMPOSE_UNICODE
            .enrich_error(config.boolean(Core::PRECOMPOSE_UNICODE))
            .with_leniency(lenient)
            .map_err(Error::ConfigBoolean)?
            .unwrap_or_default();

        const IS_WINDOWS: bool = cfg!(windows);
        let protect_windows = gitoxide::Core::PROTECT_WINDOWS
            .enrich_error(config.boolean(gitoxide::Core::PROTECT_WINDOWS))
            .with_lenient_default_value(lenient, Some(IS_WINDOWS))?
            .unwrap_or(IS_WINDOWS);

        let reflog = util::query_refupdates(&config, lenient)?;
        Ok(StageOne {
            git_dir_config: config,
            buf,
            is_bare,
            lossy,
            object_hash,
            reference_storage,
            reflog,
            precompose_unicode,
            protect_windows,
        })
    }
}

/// Return the object hash for a repository that does not set `extensions.objectFormat`.
///
/// Git interprets a missing objectFormat as the original Sha1 layout, so we return
/// gix_hash::Kind::Sha1 whenever this build can handle it.
/// In Sha256-only builds we cannot open such a repository, so return an error instead.
fn legacy_object_hash() -> Result<gix_hash::Kind, Error> {
    #[cfg(feature = "sha1")]
    {
        Ok(gix_hash::Kind::Sha1)
    }
    #[cfg(not(feature = "sha1"))]
    {
        Err(Error::UnsupportedObjectFormat { name: "sha1".into() })
    }
}

fn find_reftable_storage(
    common_dir: &std::path::Path,
    git_dir: &std::path::Path,
) -> Result<Option<std::path::PathBuf>, Error> {
    let common_reftable = common_dir.join("reftable");
    if let Some(path) = existing_reftable_storage(common_reftable)? {
        return Ok(Some(path));
    }
    if git_dir != common_dir {
        let current_reftable = git_dir.join("reftable");
        if let Some(path) = existing_reftable_storage(current_reftable)? {
            return Ok(Some(path));
        }
    }

    let worktrees_dir = common_dir.join("worktrees");
    let metadata = match std::fs::symlink_metadata(&worktrees_dir) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(Error::Io {
                source,
                path: worktrees_dir.clone(),
            });
        }
    };
    if metadata.file_type().is_symlink() {
        return Err(Error::UnsafeReftableWorktreeStorage {
            path: worktrees_dir,
            reason: "the worktrees directory is a symbolic link",
        });
    }
    if !metadata.is_dir() {
        return Err(Error::UnsafeReftableWorktreeStorage {
            path: worktrees_dir,
            reason: "the worktrees path is not a directory",
        });
    }
    let worktrees_dir = std::fs::canonicalize(&worktrees_dir).map_err(|source| Error::Io {
        source,
        path: worktrees_dir,
    })?;
    let entries = std::fs::read_dir(&worktrees_dir).map_err(|source| Error::Io {
        source,
        path: worktrees_dir.clone(),
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| Error::Io {
            source,
            path: worktrees_dir.clone(),
        })?;
        let entry_path = entry.path();
        let file_type = entry.file_type().map_err(|source| Error::Io {
            source,
            path: entry_path.clone(),
        })?;
        if file_type.is_symlink() {
            return Err(Error::UnsafeReftableWorktreeStorage {
                path: entry_path,
                reason: "the worktree entry is a symbolic link",
            });
        }
        if !file_type.is_dir() {
            continue;
        }
        if let Some(path) = existing_worktree_reftable_storage(&worktrees_dir, entry_path.join("reftable"))? {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

fn existing_worktree_reftable_storage(
    worktrees_dir: &std::path::Path,
    path: std::path::PathBuf,
) -> Result<Option<std::path::PathBuf>, Error> {
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(Error::Io { source, path }),
    };
    if metadata.file_type().is_symlink() {
        return Err(Error::UnsafeReftableWorktreeStorage {
            path,
            reason: "the reftable stack is a symbolic link",
        });
    }
    if !metadata.is_dir() {
        return Err(Error::UnsafeReftableWorktreeStorage {
            path,
            reason: "the reftable stack is not a directory",
        });
    }
    let canonical = std::fs::canonicalize(&path).map_err(|source| Error::Io {
        source,
        path: path.clone(),
    })?;
    if !canonical.starts_with(worktrees_dir) {
        return Err(Error::UnsafeReftableWorktreeStorage {
            path: canonical,
            reason: "the reftable stack resolves outside the worktrees directory",
        });
    }
    Ok(Some(canonical))
}

fn existing_reftable_storage(path: std::path::PathBuf) -> Result<Option<std::path::PathBuf>, Error> {
    match std::fs::symlink_metadata(&path) {
        Ok(_) => Ok(Some(path)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(Error::Io { source, path }),
    }
}

fn load_config(
    config_path: std::path::PathBuf,
    buf: &mut Vec<u8>,
    source: gix_config::Source,
    git_dir_trust: gix_sec::Trust,
    lossy: bool,
    lenient: bool,
) -> Result<(gix_config::File, ConfigAvailability), Error> {
    let metadata = gix_config::file::Metadata::from(source)
        .at(&config_path)
        .with(git_dir_trust);
    let mut file = match std::fs::File::open(&config_path) {
        Ok(f) => f,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok((
                gix_config::File::new(metadata),
                ConfigAvailability::Missing { path: config_path },
            ));
        }
        Err(err) => {
            if lenient {
                gix_trace::warn!(
                    "ignoring I/O error while reading configuration at '{}': {err:#?}",
                    config_path.display()
                );
                return Ok((
                    gix_config::File::new(metadata),
                    ConfigAvailability::Unreadable {
                        source: err,
                        path: config_path,
                    },
                ));
            } else {
                return Err(Error::Io {
                    source: err,
                    path: config_path,
                });
            }
        }
    };

    buf.clear();
    let mut availability = ConfigAvailability::Available;
    if let Err(err) = std::io::copy(&mut file, buf) {
        if lenient {
            gix_trace::warn!(
                "ignoring I/O error while reading configuration at '{}': {err:#?}",
                config_path.display()
            );
            buf.clear();
            availability = ConfigAvailability::Unreadable {
                source: err,
                path: config_path,
            };
        } else {
            return Err(Error::Io {
                source: err,
                path: config_path,
            });
        }
    }

    let config = gix_config::File::from_bytes_owned(
        buf,
        metadata,
        gix_config::file::init::Options {
            includes: gix_config::file::includes::Options::no_follow(),
            ..util::base_options(lossy, lenient)
        },
    )?;

    Ok((config, availability))
}

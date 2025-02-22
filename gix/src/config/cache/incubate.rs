#![allow(clippy::result_large_err)]

use super::{util, Error};
use crate::config::cache::util::{ApplyLeniency, ApplyLeniencyDefaultValue};
use crate::config::tree::{gitoxide, Core, Extensions};

/// A utility to deal with the cyclic dependency between the ref store and the configuration. The ref-store needs the
/// object hash kind, and the configuration needs the current branch name to resolve conditional includes with `onbranch`.
pub(crate) struct StageOne {
    pub git_dir_config: gix_config::File<'static>,
    pub buf: Vec<u8>,

    pub is_bare: bool,
    pub lossy: Option<bool>,
    pub object_hash: gix_hash::Kind,
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
        lossy: Option<bool>,
        lenient: bool,
    ) -> Result<Self, Error> {
        let mut buf = Vec::with_capacity(512);
        let mut config = load_config(
            common_dir.join("config"),
            &mut buf,
            gix_config::Source::Local,
            git_dir_trust,
            lossy,
            lenient,
        )?;

        // Note that we assume the repo is bare by default unless we are told otherwise. This is relevant if
        // the repo doesn't have a configuration file.
        let is_bare = util::config_bool(&config, &Core::BARE, "core.bare", true, lenient)?;
        let repo_format_version = config
            .integer("core.repositoryFormatVersion")
            .map(|version| Core::REPOSITORY_FORMAT_VERSION.try_into_usize(version))
            .transpose()?
            .unwrap_or_default();
        let object_hash = (repo_format_version != 1)
            .then_some(Ok(gix_hash::Kind::Sha1))
            .or_else(|| {
                config
                    .string(Extensions::OBJECT_FORMAT)
                    .map(|format| Extensions::OBJECT_FORMAT.try_into_object_format(format))
            })
            .transpose()?
            .unwrap_or(gix_hash::Kind::Sha1);

        let extension_worktree = util::config_bool(
            &config,
            &Extensions::WORKTREE_CONFIG,
            "extensions.worktreeConfig",
            false,
            lenient,
        )?;
        if extension_worktree {
            let worktree_config = load_config(
                git_dir.join("config.worktree"),
                &mut buf,
                gix_config::Source::Worktree,
                git_dir_trust,
                lossy,
                lenient,
            )?;
            config.append(worktree_config);
        }
        let precompose_unicode = config
            .boolean(&Core::PRECOMPOSE_UNICODE)
            .map(|v| Core::PRECOMPOSE_UNICODE.enrich_error(v))
            .transpose()
            .with_leniency(lenient)
            .map_err(Error::ConfigBoolean)?
            .unwrap_or_default();

        const IS_WINDOWS: bool = cfg!(windows);
        let protect_windows = gitoxide::Core::PROTECT_WINDOWS
            .enrich_error(
                config
                    .boolean(gitoxide::Core::PROTECT_WINDOWS)
                    .unwrap_or(Ok(IS_WINDOWS)),
            )
            .with_lenient_default_value(lenient, IS_WINDOWS)?;

        let reflog = util::query_refupdates(&config, lenient)?;
        Ok(StageOne {
            git_dir_config: config,
            buf,
            is_bare,
            lossy,
            object_hash,
            reflog,
            precompose_unicode,
            protect_windows,
        })
    }
}

fn load_config(
    config_path: std::path::PathBuf,
    buf: &mut Vec<u8>,
    source: gix_config::Source,
    git_dir_trust: gix_sec::Trust,
    lossy: Option<bool>,
    lenient: bool,
) -> Result<gix_config::File<'static>, Error> {
    let metadata = gix_config::file::Metadata::from(source)
        .at(&config_path)
        .with(git_dir_trust);
    let mut file = match std::fs::File::open(&config_path) {
        Ok(f) => f,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(gix_config::File::new(metadata)),
        Err(err) => {
            let err = Error::Io {
                source: err,
                path: config_path,
            };
            if lenient {
                gix_trace::warn!("ignoring: {err:#?}");
                return Ok(gix_config::File::new(metadata));
            } else {
                return Err(err);
            }
        }
    };

    buf.clear();
    if let Err(err) = std::io::copy(&mut file, buf) {
        let err = Error::Io {
            source: err,
            path: config_path,
        };
        if lenient {
            gix_trace::warn!("ignoring: {err:#?}");
            buf.clear();
        } else {
            return Err(err);
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

    Ok(config)
}

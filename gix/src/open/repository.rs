#![allow(clippy::result_large_err)]
use gix_config::file::Metadata;
use gix_features::threading::OwnShared;
use gix_object::bstr::ByteSlice;
use gix_path::RelativePath;
use std::path::Path;
use std::{
    borrow::Cow,
    collections::{BTreeMap, btree_map::Entry},
    ffi::OsStr,
    path::PathBuf,
};

use super::{Error, Options};
use crate::{
    ThreadSafeRepository,
    bstr::BString,
    config,
    config::{
        cache::interpolate_context,
        tree::{Core, Key, Safe, gitoxide},
    },
    open::Permissions,
};

#[derive(Default, Clone)]
pub(crate) struct EnvironmentOverrides {
    /// An override of the worktree typically from the environment, and overrides even worktree dirs set as parameter.
    ///
    /// This emulates the way git handles this override.
    worktree_dir: Option<PathBuf>,
    /// An override for the .git directory, typically from the environment.
    ///
    /// If set, the passed in `git_dir` parameter will be ignored in favor of this one.
    git_dir: Option<PathBuf>,
}

impl EnvironmentOverrides {
    fn from_env() -> Result<Self, gix_sec::permission::Error<std::path::PathBuf>> {
        let mut worktree_dir = None;
        if let Some(path) = std::env::var_os(Core::WORKTREE.the_environment_override()) {
            worktree_dir = PathBuf::from(path).into();
        }
        let mut git_dir = None;
        if let Some(path) = std::env::var_os("GIT_DIR") {
            git_dir = PathBuf::from(path).into();
        }
        Ok(EnvironmentOverrides { worktree_dir, git_dir })
    }
}

impl ThreadSafeRepository {
    /// Open a git repository at the given `path`, possibly expanding it to `path/.git` if `path` is a work tree dir.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, Error> {
        Self::open_opts(path, Options::default())
    }

    /// Open a git repository at the given `path`, possibly expanding it to `path/.git` if `path` is a work tree dir, and use
    /// `options` for fine-grained control.
    ///
    /// Note that you should use [`crate::discover()`] if security should be adjusted by ownership.
    ///
    /// ### Differences to `git2::Repository::open_ext()`
    ///
    /// Whereas `open_ext()` is the jack-of-all-trades that can do anything depending on its options, `gix` will always differentiate
    /// between discovering git repositories by searching, and opening a well-known repository by work tree or `.git` repository.
    ///
    /// Note that opening a repository for implementing custom hooks is also handle specifically in
    /// [`open_with_environment_overrides()`][Self::open_with_environment_overrides()].
    pub fn open_opts(path: impl Into<PathBuf>, mut options: Options) -> Result<Self, Error> {
        let _span = gix_trace::coarse!("ThreadSafeRepository::open()");
        let (path, kind) = {
            let path = path.into();
            let looks_like_dot_git_dir = path.ends_with(gix_discover::DOT_GIT_DIR);
            let maybe_git_repo_path = if !options.open_path_as_is && !looks_like_dot_git_dir {
                Some(path.join(gix_discover::DOT_GIT_DIR))
            } else {
                None
            };
            match maybe_git_repo_path {
                Some(candidate) => match gix_discover::is_git(&candidate) {
                    Ok(kind) => (candidate, kind),
                    Err(_) => match gix_discover::is_git(&path) {
                        Ok(kind) => (path, kind),
                        Err(err) => return Err(Error::NotARepository { source: err, path }),
                    },
                },
                None => match gix_discover::is_git(&path) {
                    Ok(kind) => (path, kind),
                    Err(err) => {
                        return Err(Error::NotARepository { source: err, path });
                    }
                },
            }
        };

        // To be altered later based on `core.precomposeUnicode`.
        let cwd = gix_fs::current_dir(false)?;
        let (git_dir, worktree_dir) = gix_discover::repository::Path::from_dot_git_dir(path, kind, &cwd)
            .expect("we have sanitized path with is_git()")
            .into_repository_and_work_tree_directories();
        if options.git_dir_trust.is_none() {
            options.git_dir_trust = gix_sec::Trust::from_path_ownership(&git_dir)?.into();
        }
        options.current_dir = Some(cwd);
        ThreadSafeRepository::open_from_paths(git_dir, worktree_dir, options)
    }

    /// Try to open a git repository in `fallback_directory` (can be worktree or `.git` directory) only if there is no override
    /// of the `gitdir` using git environment variables.
    ///
    /// Use the `trust_map` to apply options depending in the trust level for `directory` or the directory it's overridden with.
    /// The `.git` directory whether given or computed is used for trust checks.
    ///
    /// Note that this will read various `GIT_*` environment variables to check for overrides, and is probably most useful when implementing
    /// custom hooks.
    // TODO: tests, with hooks, GIT_QUARANTINE for ref-log and transaction control (needs gix-sec support to remove write access in gix-ref)
    // TODO: The following vars should end up as overrides of the respective configuration values (see git-config).
    //       GIT_PROXY_SSL_CERT, GIT_PROXY_SSL_KEY, GIT_PROXY_SSL_CERT_PASSWORD_PROTECTED.
    //       GIT_PROXY_SSL_CAINFO, GIT_SSL_CIPHER_LIST, GIT_HTTP_MAX_REQUESTS, GIT_CURL_FTP_NO_EPSV,
    #[doc(alias = "open_from_env", alias = "git2")]
    pub fn open_with_environment_overrides(
        fallback_directory: impl Into<PathBuf>,
        trust_map: gix_sec::trust::Mapping<Options>,
    ) -> Result<Self, Error> {
        let _span = gix_trace::coarse!("ThreadSafeRepository::open_with_environment_overrides()");
        let overrides = EnvironmentOverrides::from_env()?;
        let (path, path_kind): (PathBuf, _) = match overrides.git_dir {
            Some(git_dir) => gix_discover::is_git(&git_dir)
                .map_err(|err| Error::NotARepository {
                    source: err,
                    path: git_dir.clone(),
                })
                .map(|kind| (git_dir, kind))?,
            None => {
                let fallback_directory = fallback_directory.into();
                gix_discover::is_git(&fallback_directory)
                    .map_err(|err| Error::NotARepository {
                        source: err,
                        path: fallback_directory.clone(),
                    })
                    .map(|kind| (fallback_directory, kind))?
            }
        };

        // To be altered later based on `core.precomposeUnicode`.
        let cwd = gix_fs::current_dir(false)?;
        let (git_dir, worktree_dir) = gix_discover::repository::Path::from_dot_git_dir(path, path_kind, &cwd)
            .expect("we have sanitized path with is_git()")
            .into_repository_and_work_tree_directories();
        let worktree_dir = worktree_dir.or(overrides.worktree_dir);

        let git_dir_trust = gix_sec::Trust::from_path_ownership(&git_dir)?;
        let mut options = trust_map.into_value_by_level(git_dir_trust);
        options.git_dir_trust = git_dir_trust.into();
        options.current_dir = Some(cwd);
        ThreadSafeRepository::open_from_paths(git_dir, worktree_dir, options)
    }

    pub(crate) fn open_from_paths(
        mut git_dir: PathBuf,
        mut worktree_dir: Option<PathBuf>,
        mut options: Options,
    ) -> Result<Self, Error> {
        let _span = gix_trace::detail!("open_from_paths()");
        options.open_path_as_is = false;
        let Options {
            ref mut git_dir_trust,
            object_store_slots,
            filter_config_section,
            lossy_config,
            lenient_config,
            bail_if_untrusted,
            open_path_as_is: _,
            permissions:
                Permissions {
                    ref env,
                    config,
                    attributes,
                },
            ref api_config_overrides,
            ref cli_config_overrides,
            use_repository_local_environment,
            ref mut current_dir,
        } = options;
        let git_dir_trust = git_dir_trust.as_mut().expect("trust must be determined by now");

        let mut common_dir = gix_discover::path::from_plain_file(git_dir.join("commondir").as_ref())
            .transpose()?
            .map(|cd| git_dir.join(cd));
        let repo_config = config::cache::StageOne::new(
            common_dir.as_deref().unwrap_or(&git_dir),
            git_dir.as_ref(),
            *git_dir_trust,
            lossy_config,
            lenient_config,
        )?;

        if repo_config.precompose_unicode {
            git_dir = gix_utils::str::precompose_path(git_dir.into()).into_owned();
            if let Some(common_dir) = common_dir.as_mut() {
                if let Cow::Owned(precomposed) = gix_utils::str::precompose_path((&*common_dir).into()) {
                    *common_dir = precomposed;
                }
            }
            if let Some(worktree_dir) = worktree_dir.as_mut() {
                if let Cow::Owned(precomposed) = gix_utils::str::precompose_path((&*worktree_dir).into()) {
                    *worktree_dir = precomposed;
                }
            }
        }
        let common_dir_ref = common_dir.as_deref().unwrap_or(&git_dir);

        let current_dir = {
            let current_dir_ref = current_dir.as_mut().expect("BUG: current_dir must be set by caller");
            if repo_config.precompose_unicode {
                if let Cow::Owned(precomposed) = gix_utils::str::precompose_path((&*current_dir_ref).into()) {
                    *current_dir_ref = precomposed;
                }
            }
            current_dir_ref.as_path()
        };

        let mut refs = {
            let reflog = repo_config.reflog.unwrap_or(gix_ref::store::WriteReflog::Disable);
            let object_hash = repo_config.object_hash;
            let ref_store_init_opts = gix_ref::store::init::Options {
                write_reflog: reflog,
                precompose_unicode: repo_config.precompose_unicode,
                prohibit_windows_device_names: repo_config.protect_windows,
            };
            match &common_dir {
                Some(common_dir) => crate::RefStore::for_linked_worktree_opts(
                    git_dir.to_owned(),
                    common_dir.into(),
                    object_hash,
                    ref_store_init_opts,
                ),
                None => crate::RefStore::at_opts(git_dir.to_owned(), object_hash, ref_store_init_opts),
            }
        };
        let head = refs.find("HEAD").ok();
        let git_install_dir = crate::path::install_dir().ok();
        let home = gix_path::env::home_dir().and_then(|home| env.home.check_opt(home));

        let mut filter_config_section = filter_config_section.unwrap_or(config::section::is_trusted);
        let mut config = config::Cache::from_stage_one(
            repo_config,
            common_dir_ref,
            head.as_ref().and_then(|head| head.target.try_name()),
            filter_config_section,
            git_install_dir.as_deref(),
            home.as_deref(),
            *env,
            attributes,
            config,
            lenient_config,
            api_config_overrides,
            cli_config_overrides,
            use_repository_local_environment,
        )?;
        // Git's precedence is: GIT_WORK_TREE, core.bare, core.worktree, inferred worktree.
        let configured_worktree = config
            .resolved
            .raw_value_with_section_filter(Core::WORKTREE, |section| {
                is_eligible_worktree_config_section(section, &git_dir, current_dir, &mut filter_config_section)
            })
            .ok()
            .map(|(value, section)| (gix_config::Path::from(value), section.meta().source));
        let worktree_from_environment = configured_worktree
            .as_ref()
            .is_some_and(|(_, source)| *source == gix_config::Source::EnvOverride);
        let may_use_configured_worktree = config.is_bare == Some(false) || worktree_from_environment;

        if let Some((worktree, source)) = configured_worktree.filter(|_| may_use_configured_worktree) {
            let original = worktree.clone();
            let worktree = worktree
                .interpolate(interpolate_context(git_install_dir.as_deref(), home.as_deref()))
                .map_err(|err| config::Error::PathInterpolation {
                    path: original.value,
                    source: err,
                })?;
            let worktree = match source {
                gix_config::Source::Env
                | gix_config::Source::Cli
                | gix_config::Source::Api
                | gix_config::Source::EnvOverride => worktree,
                _ => worktree_dir_from_repository_config(&git_dir, worktree, current_dir),
            };
            worktree_dir = if worktree_from_environment {
                Some(gix_path::normalize_saturating(worktree.into(), current_dir).into_owned())
            } else {
                gix_path::normalize(worktree.into(), current_dir).map(Cow::into_owned)
            };
            #[allow(unused_variables, reason = "Used when tracing is enabled at compile time.")]
            if let Some(worktree_path) = worktree_dir.as_deref().filter(|wtd| !wtd.is_dir()) {
                gix_trace::warn!(
                    "The configured worktree path '{}' is not a directory or doesn't exist - `core.worktree` may be misleading",
                    worktree_path.display()
                );
            }
            if worktree_from_environment {
                config.is_bare = Some(false);
            }
        } else if !config.lenient_config
            && config.is_bare == Some(false)
            && config
                .resolved
                .boolean_filter(Core::WORKTREE, |section| {
                    is_eligible_worktree_config_section(section, &git_dir, current_dir, &mut filter_config_section)
                })
                .map_err(|err| {
                    Error::from(config::Error::ConfigTypedString(
                        config::key::GenericErrorWithValue::from(&Core::WORKTREE).with_source(err),
                    ))
                })?
                .is_some()
        {
            return Err(Error::from(config::Error::ConfigTypedString(
                config::key::GenericErrorWithValue::from(&Core::WORKTREE),
            )));
        }

        // Without an explicit path, a non-bare `.git` directory implies its parent as worktree.
        if worktree_dir.is_none()
            && config.is_bare == Some(false)
            && refs.git_dir().file_name() == Some(OsStr::new(gix_discover::DOT_GIT_DIR))
        {
            worktree_dir = Some(git_dir.parent().expect("parent is always available").to_owned());
        }
        let is_linked_worktree = refs.git_dir().parent().and_then(Path::file_name) == Some("worktrees".as_ref());
        if config.is_bare == Some(true) && !is_linked_worktree {
            // Linked worktrees may inherit core.bare=true from their common repository; all other
            // worktrees are suppressed by an explicit bare configuration.
            worktree_dir = None;
        }

        // TODO: Testing - it's hard to get non-ownership reliably and without root.
        //       For now tested manually with https://github.com/GitoxideLabs/gitoxide/issues/1912
        if *git_dir_trust != gix_sec::Trust::Full
            || worktree_dir
                .as_deref()
                .is_some_and(|wd| !gix_sec::identity::is_path_owned_by_current_user(wd).unwrap_or(false))
        {
            let safe_dirs: Vec<BString> = config
                .resolved
                .strings_filter(Safe::DIRECTORY, &mut Safe::directory_filter)
                .unwrap_or_default()
                .into_iter()
                .collect();
            let test_dir = worktree_dir.as_deref().unwrap_or(git_dir.as_path());
            let res = check_safe_directories(
                test_dir,
                git_install_dir.as_deref(),
                current_dir,
                home.as_deref(),
                &safe_dirs,
            );
            if res.is_ok() {
                *git_dir_trust = gix_sec::Trust::Full;
            } else if bail_if_untrusted {
                res?;
            } else {
                // This is how the worktree-trust can reduce the git-dir trust.
                *git_dir_trust = gix_sec::Trust::Reduced;
            }

            let Ok(mut resolved) = gix_features::threading::OwnShared::try_unwrap(config.resolved) else {
                unreachable!("Shared ownership was just established, with one reference")
            };
            let section_ids: Vec<_> = resolved.section_ids().collect();
            let mut is_valid_by_path = BTreeMap::new();
            for id in section_ids {
                let Some(mut section) = resolved.section_mut_by_id(id) else {
                    continue;
                };
                let section_trusted_by_default = Safe::directory_filter(section.meta());
                if section_trusted_by_default || section.meta().trust == gix_sec::Trust::Full {
                    continue;
                }
                let Some(meta_path) = section.meta().path.as_deref() else {
                    continue;
                };
                match is_valid_by_path.entry(meta_path.to_owned()) {
                    Entry::Occupied(entry) => {
                        if *entry.get() {
                            section.set_trust(gix_sec::Trust::Full);
                        } else {
                            continue;
                        }
                    }
                    Entry::Vacant(entry) => {
                        let config_file_is_safe = (meta_path.strip_prefix(test_dir).is_ok()
                            && *git_dir_trust == gix_sec::Trust::Full)
                            || check_safe_directories(
                                meta_path,
                                git_install_dir.as_deref(),
                                current_dir,
                                home.as_deref(),
                                &safe_dirs,
                            )
                            .is_ok();

                        entry.insert(config_file_is_safe);
                        if config_file_is_safe {
                            section.set_trust(gix_sec::Trust::Full);
                        }
                    }
                }
            }
            config.resolved = resolved.into();
        }

        let index_path = match config
            .resolved
            .string_filter(gitoxide::Core::INDEX_FILE, &mut filter_config_section)
        {
            Some(value) => {
                gitoxide::Core::INDEX_FILE.validate(value.as_bstr()).map_err(|_| {
                    config::Error::ConfigTypedString(config::key::GenericErrorWithValue::from_value(
                        &gitoxide::Core::INDEX_FILE,
                        value.clone(),
                    ))
                })?;
                gix_path::from_bstr(value).into_owned()
            }
            None => git_dir.join("index"),
        };

        refs.write_reflog = config::cache::util::reflog_or_default(config.reflog, worktree_dir.is_some());
        refs.namespace.clone_from(&config.refs_namespace);
        let prefix = replacement_objects_refs_prefix(&config.resolved, lenient_config, filter_config_section)?;

        if *git_dir_trust == gix_sec::Trust::Reduced && config.alloc_limit_bytes.is_none() {
            let alloc_limit_if_reduced_trust =
                match gitoxide::Objects::ALLOC_LIMIT_IF_REDUCED_TRUST.try_into_usize(config.resolved.integer_filter(
                    gitoxide::Objects::ALLOC_LIMIT_IF_REDUCED_TRUST,
                    &mut filter_config_section,
                )) {
                    Ok(Some(value)) => value,
                    Ok(None) => gitoxide::Objects::ALLOC_LIMIT_IF_REDUCED_TRUST_DEFAULT,
                    Err(_) if config.lenient_config => gitoxide::Objects::ALLOC_LIMIT_IF_REDUCED_TRUST_DEFAULT,
                    Err(err) => return Err(Error::from(config::Error::from(err))),
                };
            if alloc_limit_if_reduced_trust != 0 {
                config.alloc_limit_bytes = Some(alloc_limit_if_reduced_trust);
                gix_trace::info!(
                    concat!(
                        "Applied a default allocation limit of {alloc_limit_bytes} ",
                        "bytes while opening reduced-trust repository '{git_dir}'. ",
                        "Set `gitoxide.objects.allocLimitIfReducedTrust=0` to disable this fallback",
                    ),
                    alloc_limit_bytes = alloc_limit_if_reduced_trust,
                    git_dir = git_dir.display(),
                );
            }
        }

        let replacements = match prefix {
            Some(prefix) => {
                let prefix: &RelativePath = prefix.as_bstr().try_into()?;

                Some(prefix).and_then(|prefix| {
                    let _span = gix_trace::detail!("find replacement objects");
                    let platform = refs.iter().ok()?;
                    let iter = platform.prefixed(prefix).ok()?;
                    let replacements = iter
                        .filter_map(Result::ok)
                        .filter_map(|r: gix_ref::Reference| {
                            let target = r.target.try_id()?.to_owned();
                            let source =
                                gix_hash::ObjectId::from_hex(r.name.as_bstr().strip_prefix(prefix.as_ref())?).ok()?;
                            Some((source, target))
                        })
                        .collect::<Vec<_>>();
                    Some(replacements)
                })
            }
            None => None,
        };
        let replacements = replacements.unwrap_or_default();

        Ok(ThreadSafeRepository {
            objects: OwnShared::new(gix_odb::Store::at_opts(
                common_dir_ref.join("objects"),
                config.object_hash,
                &mut replacements.into_iter(),
                gix_odb::store::init::Options {
                    slots: object_store_slots,
                    use_multi_pack_index: config.use_multi_pack_index,
                    alloc_limit_bytes: config.alloc_limit_bytes,
                    loose_compression: config.loose_compression,
                    current_dir: current_dir.to_owned().into(),
                },
            )?),
            common_dir,
            refs,
            work_tree: worktree_dir,
            index_path,
            config,
            // used when spawning new repositories off this one when following worktrees
            linked_worktree_options: options,
            #[cfg(feature = "index")]
            index: gix_fs::SharedFileSnapshotMut::new().into(),
            shallow_commits: gix_fs::SharedFileSnapshotMut::new().into(),
            #[cfg(feature = "attributes")]
            modules: gix_fs::SharedFileSnapshotMut::new().into(),
        })
    }
}

/// Return whether `section` may provide `core.worktree` while opening `git_dir`.
///
/// The section must pass the caller's trust filter. `GIT_WORK_TREE` has no configuration-file
/// path and is accepted directly; all other values must come from within this repository to keep
/// a parent repository's `core.worktree` from leaking into submodules.
fn is_eligible_worktree_config_section(
    section: &Metadata,
    git_dir: &Path,
    current_dir: &Path,
    filter_config_section: &mut fn(&Metadata) -> bool,
) -> bool {
    if !filter_config_section(section) {
        return false;
    }
    if section.source == gix_config::Source::EnvOverride {
        return true;
    }
    // Ignore worktree settings from another repository, as can happen while opening submodules.
    section
        .path
        .as_deref()
        .and_then(|path| gix_path::normalize(path.into(), current_dir))
        .is_some_and(|config_path| config_path.starts_with(git_dir))
}

/// Return the worktree directory implied by the `core.worktree` value `wt_path` from repository-owned
/// configuration, resolved against `git_dir`.
///
/// Git resolves symlinks in the `.git` directory before interpreting relative worktree paths, which matters
/// when the `.git` directory itself is reached through a symlink: traversing `..` from the symlink and from
/// its target yields different directories. When both ways of resolving `wt_path` denote the same directory
/// on disk, however - for instance if only an ancestor of the repository is a symlink, like the temporary
/// directory on macOS - prefer the symlink-preserving path so all paths of the opened repository remain
/// consistent with the path the repository was opened with.
fn worktree_dir_from_repository_config(git_dir: &Path, wt_path: PathBuf, current_dir: &Path) -> PathBuf {
    fn realpath(path: &Path, current_dir: &Path) -> Option<PathBuf> {
        gix_path::realpath_opts(path, current_dir, crate::path::realpath::MAX_SYMLINKS).ok()
    }
    if wt_path.is_absolute() {
        return wt_path;
    }
    let logical_git_dir = gix_path::normalize(
        Cow::Owned(if git_dir.is_relative() {
            current_dir.join(git_dir)
        } else {
            git_dir.to_owned()
        }),
        current_dir,
    )
    .map(Cow::into_owned);
    let symlink_preserving = git_dir.join(&wt_path);
    let real_git_dir = match (realpath(git_dir, current_dir), logical_git_dir) {
        (Some(real_git_dir), Some(logical_git_dir)) if real_git_dir != logical_git_dir => real_git_dir,
        // There is no symlink to account for - keep existing paths stable.
        _ => return symlink_preserving,
    };
    let resolved = real_git_dir.join(&wt_path);
    let denotes_same_directory = gix_path::normalize(Cow::Borrowed(symlink_preserving.as_path()), current_dir)
        .and_then(|normalized| realpath(&normalized, current_dir))
        .zip(realpath(&resolved, current_dir))
        .is_some_and(|(symlink_preserving, resolved)| symlink_preserving == resolved);
    if denotes_same_directory {
        symlink_preserving
    } else {
        resolved
    }
}

// TODO: tests
fn replacement_objects_refs_prefix(
    config: &gix_config::File,
    lenient: bool,
    mut filter_config_section: fn(&gix_config::file::Metadata) -> bool,
) -> Result<Option<BString>, Error> {
    let is_disabled = config::shared::is_replace_refs_enabled(config, lenient, filter_config_section)
        .map_err(config::Error::ConfigBoolean)?
        .unwrap_or(true);

    if is_disabled {
        return Ok(None);
    }

    let ref_base = {
        let key = "gitoxide.objects.replaceRefBase";
        debug_assert_eq!(gitoxide::Objects::REPLACE_REF_BASE.logical_name(), key);
        config
            .string_filter(key, &mut filter_config_section)
            .unwrap_or_else(|| gitoxide::Objects::REPLACE_REF_BASE.default_value_or_panic().into())
    };
    Ok(Some(ref_base))
}

fn check_safe_directories(
    path_to_test: &std::path::Path,
    git_install_dir: Option<&std::path::Path>,
    current_dir: &std::path::Path,
    home: Option<&std::path::Path>,
    safe_dirs: &[BString],
) -> Result<(), Error> {
    let mut is_safe = false;
    let realpath_or_original = |path: &std::path::Path| {
        std::fs::canonicalize(path)
            .or_else(|_| gix_path::realpath_opts(path, current_dir, gix_path::realpath::MAX_SYMLINKS))
            .unwrap_or_else(|_| path.to_owned())
    };
    let path_to_test = realpath_or_original(path_to_test);
    for safe_dir in safe_dirs {
        let safe_dir = safe_dir.as_bstr();
        if safe_dir == "*" {
            is_safe = true;
            continue;
        }
        if safe_dir.is_empty() {
            is_safe = false;
            continue;
        }
        if !is_safe {
            let safe_dir =
                match gix_config::Path::from(safe_dir).interpolate(interpolate_context(git_install_dir, home)) {
                    Ok(path) => path,
                    Err(_) => gix_path::from_bstr(safe_dir).into_owned(),
                };
            if !safe_dir.is_absolute() {
                gix_trace::warn!(
                    "safe.directory '{safe_dir}' not absolute",
                    safe_dir = safe_dir.display()
                );
                continue;
            }
            if safe_dir.ends_with("*") {
                let safe_dir = safe_dir.parent().expect("* is last component");
                if path_to_test.strip_prefix(realpath_or_original(safe_dir)).is_ok() {
                    is_safe = true;
                }
            } else if realpath_or_original(&safe_dir) == path_to_test {
                is_safe = true;
            }
        }
    }
    if is_safe {
        Ok(())
    } else {
        Err(Error::UnsafeGitDir { path: path_to_test })
    }
}

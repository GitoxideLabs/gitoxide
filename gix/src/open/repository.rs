#![allow(clippy::result_large_err)]
use super::{Error, Options};
use crate::{
    bstr::BString,
    config,
    config::{
        cache::interpolate_context,
        tree::{gitoxide, Core, Key, Safe},
    },
    open::Permissions,
    ThreadSafeRepository,
};
use gix_features::threading::OwnShared;
use gix_object::bstr::ByteSlice;
use gix_path::relative_path::RelativePath;
use std::ffi::OsStr;
use std::{borrow::Cow, path::PathBuf};

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
            let looks_like_git_dir =
                path.ends_with(gix_discover::DOT_GIT_DIR) || path.extension() == Some(std::ffi::OsStr::new("git"));
            let candidate = if !options.open_path_as_is && !looks_like_git_dir {
                Cow::Owned(path.join(gix_discover::DOT_GIT_DIR))
            } else {
                Cow::Borrowed(&path)
            };
            match gix_discover::is_git(candidate.as_ref()) {
                Ok(kind) => (candidate.into_owned(), kind),
                Err(err) => {
                    if options.open_path_as_is || matches!(candidate, Cow::Borrowed(_)) {
                        return Err(Error::NotARepository {
                            source: err,
                            path: candidate.into_owned(),
                        });
                    }
                    match gix_discover::is_git(&path) {
                        Ok(kind) => (path, kind),
                        Err(err) => return Err(Error::NotARepository { source: err, path }),
                    }
                }
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
        options.current_dir = Some(cwd);
        ThreadSafeRepository::open_from_paths(git_dir, worktree_dir, options)
    }

    pub(crate) fn open_from_paths(
        mut git_dir: PathBuf,
        mut worktree_dir: Option<PathBuf>,
        mut options: Options,
    ) -> Result<Self, Error> {
        let _span = gix_trace::detail!("open_from_paths()");
        let Options {
            git_dir_trust,
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
            ref mut current_dir,
        } = options;
        let git_dir_trust = git_dir_trust.expect("trust must be determined by now");

        let mut common_dir = gix_discover::path::from_plain_file(git_dir.join("commondir").as_ref())
            .transpose()?
            .map(|cd| git_dir.join(cd));
        let repo_config = config::cache::StageOne::new(
            common_dir.as_deref().unwrap_or(&git_dir),
            git_dir.as_ref(),
            git_dir_trust,
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
                object_hash,
                precompose_unicode: repo_config.precompose_unicode,
                prohibit_windows_device_names: repo_config.protect_windows,
            };
            match &common_dir {
                Some(common_dir) => {
                    crate::RefStore::for_linked_worktree(git_dir.to_owned(), common_dir.into(), ref_store_init_opts)
                }
                None => crate::RefStore::at(git_dir.to_owned(), ref_store_init_opts),
            }
        };
        let head = refs.find("HEAD").ok();
        let git_install_dir = crate::path::install_dir().ok();
        let home = gix_path::env::home_dir().and_then(|home| env.home.check_opt(home));

        let mut filter_config_section = filter_config_section.unwrap_or(config::section::is_trusted);
        let config = config::Cache::from_stage_one(
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
        )?;

        if bail_if_untrusted && git_dir_trust != gix_sec::Trust::Full {
            check_safe_directories(
                &git_dir,
                git_install_dir.as_deref(),
                current_dir,
                home.as_deref(),
                &config,
            )?;
        }

        // core.worktree might be used to overwrite the worktree directory
        if !config.is_bare {
            let mut key_source = None;
            let worktree_path = config
                .resolved
                .path_filter(Core::WORKTREE, {
                    |section| {
                        if !filter_config_section(section) {
                            return false;
                        }
                        // ignore worktree settings that aren't from our repository. This can happen
                        // with worktrees of submodules for instance.
                        let is_config_in_our_repo = section
                            .path
                            .as_deref()
                            .and_then(|p| gix_path::normalize(p.into(), current_dir))
                            .is_some_and(|config_path| config_path.starts_with(&git_dir));
                        if !is_config_in_our_repo {
                            return false;
                        }
                        key_source = Some(section.source);
                        true
                    }
                })
                .zip(key_source);
            if let Some((wt, key_source)) = worktree_path {
                let wt_clone = wt.clone();
                let wt_path = wt
                    .interpolate(interpolate_context(git_install_dir.as_deref(), home.as_deref()))
                    .map_err(|err| config::Error::PathInterpolation {
                        path: wt_clone.value.into_owned(),
                        source: err,
                    })?;
                let wt_path = match key_source {
                    gix_config::Source::Env
                    | gix_config::Source::Cli
                    | gix_config::Source::Api
                    | gix_config::Source::EnvOverride => wt_path,
                    _ => git_dir.join(wt_path).into(),
                };
                worktree_dir = gix_path::normalize(wt_path, current_dir).map(Cow::into_owned);
                #[allow(unused_variables)]
                if let Some(worktree_path) = worktree_dir.as_deref().filter(|wtd| !wtd.is_dir()) {
                    gix_trace::warn!("The configured worktree path '{}' is not a directory or doesn't exist - `core.worktree` may be misleading", worktree_path.display());
                }
            } else if !config.lenient_config
                && config
                    .resolved
                    .boolean_filter(Core::WORKTREE, &mut filter_config_section)
                    .is_some()
            {
                return Err(Error::from(config::Error::ConfigTypedString(
                    config::key::GenericErrorWithValue::from(&Core::WORKTREE),
                )));
            }
        }

        {
            let looks_like_standard_git_dir =
                || refs.git_dir().file_name() == Some(OsStr::new(gix_discover::DOT_GIT_DIR));
            match worktree_dir {
                None if !config.is_bare && looks_like_standard_git_dir() => {
                    worktree_dir = Some(git_dir.parent().expect("parent is always available").to_owned());
                }
                Some(_) => {
                    // We may assume that the presence of a worktree-dir means it's not bare, but only if there
                    // is no configuration saying otherwise.
                    // Thus, if we are here and the common-dir config claims it's bare and we have inferred a worktree anyway,
                    // forget about it.
                    if looks_like_standard_git_dir()
                        && config
                            .resolved
                            .boolean_filter("core.bare", |md| md.source == gix_config::Source::Local)
                            .transpose()
                            .ok()
                            .flatten()
                            .is_some()
                        && config.is_bare
                    {
                        worktree_dir = None;
                    }
                }
                None => {}
            }
        }

        refs.write_reflog = config::cache::util::reflog_or_default(config.reflog, worktree_dir.is_some());
        refs.namespace.clone_from(&config.refs_namespace);
        let prefix = replacement_objects_refs_prefix(&config.resolved, lenient_config, filter_config_section)?;
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
                &mut replacements.into_iter(),
                gix_odb::store::init::Options {
                    slots: object_store_slots,
                    object_hash: config.object_hash,
                    use_multi_pack_index: config.use_multi_pack_index,
                    current_dir: current_dir.to_owned().into(),
                },
            )?),
            common_dir,
            refs,
            work_tree: worktree_dir,
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

// TODO: tests
fn replacement_objects_refs_prefix(
    config: &gix_config::File<'static>,
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
            .unwrap_or_else(|| Cow::Borrowed("refs/replace/".into()))
    }
    .into_owned();
    Ok(Some(ref_base))
}

fn check_safe_directories(
    git_dir: &std::path::Path,
    git_install_dir: Option<&std::path::Path>,
    current_dir: &std::path::Path,
    home: Option<&std::path::Path>,
    config: &config::Cache,
) -> Result<(), Error> {
    let mut is_safe = false;
    let git_dir = match gix_path::realpath_opts(git_dir, current_dir, gix_path::realpath::MAX_SYMLINKS) {
        Ok(p) => p,
        Err(_) => git_dir.to_owned(),
    };
    for safe_dir in config
        .resolved
        .strings_filter(Safe::DIRECTORY, &mut Safe::directory_filter)
        .unwrap_or_default()
    {
        if safe_dir.as_ref() == "*" {
            is_safe = true;
            continue;
        }
        if safe_dir.is_empty() {
            is_safe = false;
            continue;
        }
        if !is_safe {
            let safe_dir = match gix_config::Path::from(std::borrow::Cow::Borrowed(safe_dir.as_ref()))
                .interpolate(interpolate_context(git_install_dir, home))
            {
                Ok(path) => path,
                Err(_) => gix_path::from_bstr(safe_dir),
            };
            if safe_dir == git_dir {
                is_safe = true;
                continue;
            }
        }
    }
    if is_safe {
        Ok(())
    } else {
        Err(Error::UnsafeGitDir { path: git_dir })
    }
}

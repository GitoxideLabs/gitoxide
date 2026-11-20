use gix_config::file::Metadata;
use gix_error::{ErrorExt, ResultExt, bail, message, not_found, validation};
use gix_object::bstr::ByteSlice;
use gix_parallel::OwnShared;
use gix_path::RelativePath;
use gix_sec::Trust;
use std::path::Path;
use std::{borrow::Cow, collections::BTreeMap, ffi::OsStr, path::PathBuf};

use super::{Options, RelatedRepositoryKind, RepositoryTrust, related_repository_skipped};
use crate::{
    Error, Repository, Result, ThreadSafeRepository,
    bstr::BString,
    config,
    config::{
        cache::{interpolate_context, util::ApplyLeniencyDefaultValue},
        tree::{Core, Key, Safe, gitoxide},
    },
    open::Permissions,
};

fn not_a_repository(source: Error, path: PathBuf) -> Error {
    source.and_raise(message!("\"{}\" does not appear to be a git repository", path.display()).not_found())
}

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
    fn from_env() -> std::result::Result<Self, gix_sec::permission::Error<std::path::PathBuf>> {
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
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        Self::open_opts(path, Options::default())
    }

    /// Open a git repository at the given `path`, possibly expanding it to `path/.git` if `path` is a work tree dir, and use
    /// `options` for fine-grained control.
    /// Empty `core.worktree` values include their bytes as `input` [metadata](gix_error::Error::metadata()).
    ///
    /// By default, initial trust comes from ownership of the candidate path (`path` or `path/.git`) before resolving
    /// a possible gitfile. Ownership checks of the resolved Git directory, common directory, and existing worktree
    /// can lower this level. Use [`Options::with_trust()`] to set the initial level or
    /// [`Options::with_enforced_trust()`] to set the final level.
    /// Use [`crate::discover()`] if options such as permissions should also be selected according to ownership.
    ///
    /// Opening an absolute Git directory keeps repository paths absolute. The selected index path is always absolute,
    /// even when the Git directory is relative; see [`crate::Repository::index_path()`] for override resolution.
    ///
    /// ### Differences to `git2::Repository::open_ext()`
    ///
    /// Whereas `open_ext()` is the jack-of-all-trades that can do anything depending on its options, `gix` will always differentiate
    /// between discovering git repositories by searching, and opening a well-known repository by work tree or `.git` repository.
    ///
    /// Note that opening a repository for implementing custom hooks is also handle specifically in
    /// [`open_with_environment_overrides()`][Self::open_with_environment_overrides()].
    pub fn open_opts(path: impl Into<PathBuf>, options: Options) -> Result<Self> {
        Self::open_opts_inner(path.into(), options, None)
    }

    pub(crate) fn open_opts_inner(
        path: PathBuf,
        mut options: Options,
        related: Option<(&Repository, RelatedRepositoryKind)>,
    ) -> Result<Self> {
        let _span = gix_trace::coarse!("ThreadSafeRepository::open()");
        let (path, kind) = {
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
                        Err(err) => return Err(not_a_repository(err, path)),
                    },
                },
                None => match gix_discover::is_git(&path) {
                    Ok(kind) => (path, kind),
                    Err(err) => {
                        return Err(not_a_repository(err, path));
                    }
                },
            }
        };

        // To be altered later based on `core.precomposeUnicode`.
        let cwd = gix_fs::current_dir(false).or_error()?;
        let candidate_trust = options
            .git_dir_trust
            .is_none()
            .then(|| gix_sec::Trust::from_path_ownership(&path))
            .transpose()
            .or_error()?;
        let (git_dir, worktree_dir) = gix_discover::repository::Path::from_dot_git_dir(path.clone(), kind, &cwd)
            .expect("we have sanitized path with is_git()")
            .into_repository_and_work_tree_directories();
        if let Some(candidate_trust) = candidate_trust {
            options = options.with_trust(gix_discover::repository::trust(
                &git_dir,
                worktree_dir.as_deref(),
                &cwd,
                (&path, Some(candidate_trust)),
            )?);
        }
        options.current_dir = Some(cwd);
        if let Some((parent, kind)) = related {
            options = options
                .configure_related_open(parent, kind, &git_dir, worktree_dir.as_deref())?
                .continue_value()
                .ok_or_else(|| related_repository_skipped(&git_dir))?;
        }
        ThreadSafeRepository::open_from_paths(git_dir, worktree_dir, options, None)
    }

    /// Try to open a git repository in `fallback_directory` (can be worktree or `.git` directory) only if there is no override
    /// of the `gitdir` using git environment variables.
    ///
    /// Call `open_options_by_trust` with the ownership-derived trust level and corresponding opening defaults,
    /// as in [`Self::discover_opts()`]. Initial trust comes from the candidate path (`GIT_DIR` if set, otherwise
    /// `fallback_directory`) before resolving a possible gitfile. Ownership checks of the resolved Git directory,
    /// common directory, and existing worktree can only lower that level, so a trusted target cannot upgrade an
    /// untrusted gitfile. An explicit trust setting returned by the callback is preserved.
    ///
    /// Note that this will read various `GIT_*` environment variables to check for overrides, and is probably most useful when implementing
    /// custom hooks.
    /// Empty `core.worktree` values include their bytes as `input` [metadata](gix_error::Error::metadata()).
    // TODO: tests, with hooks, GIT_QUARANTINE for ref-log and transaction control (needs gix-sec support to remove write access in gix-ref)
    // TODO: The following vars should end up as overrides of the respective configuration values (see git-config).
    //       GIT_PROXY_SSL_CERT, GIT_PROXY_SSL_KEY, GIT_PROXY_SSL_CERT_PASSWORD_PROTECTED.
    //       GIT_PROXY_SSL_CAINFO, GIT_SSL_CIPHER_LIST, GIT_HTTP_MAX_REQUESTS, GIT_CURL_FTP_NO_EPSV,
    #[doc(alias = "open_from_env", alias = "git2")]
    pub fn open_with_environment_overrides(
        fallback_directory: impl Into<PathBuf>,
        open_options_by_trust: impl FnOnce(Options, Trust) -> Options,
    ) -> Result<Self> {
        Self::open_with_environment_and_options(fallback_directory, None, |trust| {
            Options::from_trust_callback(trust, open_options_by_trust)
        })
    }

    pub(crate) fn open_with_environment_and_options(
        fallback_directory: impl Into<PathBuf>,
        assumed_trust: Option<Trust>,
        open_options: impl FnOnce(Trust) -> Options,
    ) -> Result<Self> {
        let _span = gix_trace::coarse!("ThreadSafeRepository::open_with_environment_overrides()");
        let overrides = EnvironmentOverrides::from_env().or_error()?;
        let (path, path_kind): (PathBuf, _) = match overrides.git_dir {
            Some(git_dir) => gix_discover::is_git(&git_dir)
                .map_err(|err| not_a_repository(err, git_dir.clone()))
                .map(|kind| (git_dir, kind))?,
            None => {
                let fallback_directory = fallback_directory.into();
                gix_discover::is_git(&fallback_directory)
                    .map_err(|err| not_a_repository(err, fallback_directory.clone()))
                    .map(|kind| (fallback_directory, kind))?
            }
        };

        // To be altered later based on `core.precomposeUnicode`.
        let cwd = gix_fs::current_dir(false).or_error()?;
        let candidate_trust = assumed_trust
            .is_none()
            .then(|| Trust::from_path_ownership(&path))
            .transpose()
            .or_error()?;
        let (git_dir, worktree_dir) = gix_discover::repository::Path::from_dot_git_dir(path.clone(), path_kind, &cwd)
            .expect("we have sanitized path with is_git()")
            .into_repository_and_work_tree_directories();
        let worktree_dir = worktree_dir.or(overrides.worktree_dir);

        let git_dir_trust = match assumed_trust {
            Some(trust) => trust,
            None => gix_discover::repository::trust(&git_dir, worktree_dir.as_deref(), &cwd, (&path, candidate_trust))?,
        };
        let mut options = open_options(git_dir_trust);
        options.current_dir = Some(cwd);
        ThreadSafeRepository::open_from_paths(git_dir, worktree_dir, options, None)
    }

    /// Initialize a repository from already resolved repository paths, loading configuration and creating its stores.
    ///
    /// This is the shared initialization step for opening, discovery, initialization, and worktree proxies.
    /// Unlike [`Self::open_opts()`], it does not probe for `path/.git` or resolve a worktree's `.git` file, so a proxy
    /// can open its private Git directory even when the checkout is inaccessible.
    ///
    /// `git_dir` is the private Git directory, containing this repository's `HEAD` and default index. `worktree_dir` is an
    /// optional, inferred checkout directory; configuration and environment overrides may replace or suppress it.
    /// `options` controls configuration and store initialization. Callers must already have set `options.git_dir_trust`
    /// and captured the opening CWD in `options.current_dir`; `options.open_path_as_is` is reset here.
    /// Submodule callers may retain the inferred checkout despite `core.bare=true` using the internal,
    /// one-shot `options.retain_inferred_worktree_when_bare` policy. Its ownership is checked normally.
    ///
    /// `known_common_dir` is the already resolved directory containing shared configuration, objects, and references,
    /// not the raw contents of a `commondir` file or a path relative to `git_dir`. Worktree proxies supply their parent
    /// repository's common directory, anchored to the parent's captured CWD, to reuse that known relationship without
    /// rereading `git_dir/commondir`. With `None`, read that file and resolve its contents relative to `git_dir`; if the
    /// file is absent, `git_dir` also serves as the common directory.
    ///
    /// An absolute `git_dir` keeps the resulting repository paths absolute. The selected index path is always made
    /// absolute using `options.current_dir` where needed, even if `git_dir` is relative.
    pub(crate) fn open_from_paths(
        mut git_dir: PathBuf,
        mut worktree_dir: Option<PathBuf>,
        mut options: Options,
        known_common_dir: Option<PathBuf>,
    ) -> Result<Self> {
        let _span = gix_trace::detail!("open_from_paths()");
        options.open_path_as_is = false;
        let retain_inferred_worktree_when_bare = std::mem::take(&mut options.retain_inferred_worktree_when_bare);
        let Options {
            ref mut git_dir_trust,
            configure_related_repository: _,
            object_store_slots,
            filter_config_section,
            lossy_config,
            lenient_config,
            bail_if_untrusted,
            open_path_as_is: _,
            retain_inferred_worktree_when_bare: _,
            permissions:
                Permissions {
                    ref env,
                    config,
                    attributes,
                },
            ref api_config_overrides,
            ref cli_config_overrides,
            ref git_installation_config_path,
            ref system_config_path,
            use_repository_local_environment,
            ref mut current_dir,
        } = options;
        let git_dir_trust = git_dir_trust.as_mut().expect("trust must be determined by now");
        if bail_if_untrusted && matches!(git_dir_trust, RepositoryTrust::Enforced(Trust::Reduced)) {
            return Err(message!(
                "Full trust is required to open \"{}\", but reduced trust was enforced",
                git_dir.display()
            )
            .permission_denied_error());
        }

        let mut common_dir = match known_common_dir {
            Some(common_dir) => Some(common_dir),
            None => gix_discover::path::from_plain_file(git_dir.join("commondir").as_ref())
                .transpose()
                .or_error()?
                .map(|cd| git_dir.join(cd)),
        };
        if git_dir.is_absolute() {
            common_dir = common_dir
                .map(|path| {
                    absolute_path(
                        &path,
                        current_dir.as_deref().expect("BUG: current_dir must be set by caller"),
                    )
                })
                .transpose()?;
        }
        let mut filter_config_section = filter_config_section.unwrap_or(config::section::is_trusted);
        let repo_config = config::cache::StageOne::new(
            common_dir.as_deref().unwrap_or(&git_dir),
            git_dir.as_ref(),
            git_dir_trust.level(),
            lossy_config,
            lenient_config,
        )
        .or_raise(|| message("Repository configuration could not be loaded"))?;

        if repo_config.precompose_unicode {
            git_dir = gix_utils::str::precompose_path(git_dir.into()).into_owned();
            if let Some(common_dir) = common_dir.as_mut()
                && let Cow::Owned(precomposed) = gix_utils::str::precompose_path((&*common_dir).into())
            {
                *common_dir = precomposed;
            }
            if let Some(worktree_dir) = worktree_dir.as_mut()
                && let Cow::Owned(precomposed) = gix_utils::str::precompose_path((&*worktree_dir).into())
            {
                *worktree_dir = precomposed;
            }
        }
        let common_dir_ref = common_dir.as_deref().unwrap_or(&git_dir);

        let current_dir = {
            let current_dir_ref = current_dir.as_mut().expect("BUG: current_dir must be set by caller");
            if repo_config.precompose_unicode
                && let Cow::Owned(precomposed) = gix_utils::str::precompose_path((&*current_dir_ref).into())
            {
                *current_dir_ref = precomposed;
            }
            current_dir_ref.as_path()
        };

        let (mut refs, is_linked_worktree) = {
            let reflog = repo_config.reflog.unwrap_or(gix_ref::store::WriteReflog::Disable);
            let object_hash = repo_config.object_hash;
            let ref_store_init_opts = gix_ref::store::init::Options {
                write_reflog: reflog,
                precompose_unicode: repo_config.precompose_unicode,
                prohibit_windows_device_names: cfg!(windows),
            };
            // `commondir` (or the equivalent path supplied by a worktree proxy) identifies a private
            // Git directory backed by a shared store, regardless of how its parent directory is spelled.
            match &common_dir {
                Some(common_dir) => (
                    crate::RefStore::for_linked_worktree_opts(
                        git_dir.to_owned(),
                        common_dir.into(),
                        object_hash,
                        ref_store_init_opts,
                    ),
                    true,
                ),
                None => (
                    crate::RefStore::at_opts(git_dir.to_owned(), object_hash, ref_store_init_opts),
                    false,
                ),
            }
        };
        let head = refs.find("HEAD").ok();
        let git_install_dir = crate::path::install_dir().ok();
        let home = gix_path::env::home_dir().and_then(|home| env.home.check_opt(home));

        let mut resolved = repo_config
            .resolve(
                common_dir_ref,
                head.as_ref().and_then(|head| head.target.try_name()),
                git_install_dir.as_deref(),
                home.as_deref(),
                git_installation_config_path.as_deref(),
                system_config_path.as_deref(),
                *env,
                config,
                lenient_config,
                api_config_overrides,
                cli_config_overrides,
                use_repository_local_environment,
            )
            .or_raise(|| message("Repository configuration could not be loaded"))?;
        let safe_dirs = match git_dir_trust {
            RepositoryTrust::Enforced(_) => Vec::new(),
            RepositoryTrust::Precomputed(_) => resolved
                .file
                .strings_filter(Safe::DIRECTORY, Safe::directory_filter)
                .unwrap_or_default(),
        };
        let check_safe_directory = |path: &Path| {
            check_safe_directories(
                path,
                git_install_dir.as_deref(),
                current_dir,
                home.as_deref(),
                &safe_dirs,
            )
        };
        let is_safe = |path: &Path| !safe_dirs.is_empty() && check_safe_directory(path).is_ok();
        if let RepositoryTrust::Precomputed(trust @ Trust::Reduced) = git_dir_trust
            && (bail_if_untrusted || !safe_dirs.is_empty())
        {
            // Repository configuration can change the checkout after discovery.
            match check_safe_directory(worktree_dir.as_deref().unwrap_or(&git_dir)) {
                Ok(()) => *trust = Trust::Full,
                Err(err) if bail_if_untrusted => return Err(err),
                Err(_) => {}
            }
            update_repository_section_trust(&mut resolved.file, *trust, is_safe);
        }

        let inferred_worktree_dir = worktree_dir;
        let mut worktree_dir = loop {
            let mut worktree_dir = inferred_worktree_dir.clone();
            let mut is_bare = resolved.is_bare;
            // Git's precedence is: GIT_WORK_TREE, core.bare, core.worktree, inferred worktree.
            let configured_worktree = resolved
                .file
                .raw_value_with_section_filter(Core::WORKTREE, |section| {
                    is_eligible_worktree_config_section(section, &git_dir, current_dir, &mut filter_config_section)
                })
                .ok()
                .map(|(value, section)| (value, section.meta().source));
            let worktree_from_environment = configured_worktree
                .as_ref()
                .is_some_and(|(_, source)| *source == gix_config::Source::EnvOverride);
            let may_use_configured_worktree = is_bare == Some(false) || worktree_from_environment;

            if let Some((worktree, source)) = configured_worktree.filter(|_| may_use_configured_worktree) {
                if worktree.is_empty() {
                    return Err(not_found("path is missing").and_raise(
                        validation("The path at the 'core.worktree' configuration could not be interpolated")
                            .with_input(worktree),
                    ));
                }
                // Git treats core.worktree as a literal path, without tilde or prefix interpolation.
                let worktree = gix_path::from_bstr(worktree.as_bstr())?.into_owned();
                let worktree = match source {
                    gix_config::Source::Env
                    | gix_config::Source::Cli
                    | gix_config::Source::Api
                    | gix_config::Source::EnvOverride => worktree,
                    _ => worktree_dir_from_repository_config(&git_dir, worktree, current_dir),
                };
                let keep_worktree_relative = worktree_dir.as_deref().is_some_and(Path::is_relative)
                    && git_dir.is_relative()
                    && worktree.is_relative()
                    && !matches!(
                        source,
                        gix_config::Source::Env
                            | gix_config::Source::Cli
                            | gix_config::Source::Api
                            | gix_config::Source::EnvOverride
                    );
                worktree_dir = if worktree_from_environment {
                    Some(gix_path::normalize_saturating(worktree.into(), current_dir)?.into_owned())
                } else {
                    gix_path::normalize(worktree.into(), current_dir).map(Cow::into_owned)
                };
                // Normalization can consume leading `..` through CWD; preserve a relative inferred checkout's namespace.
                if keep_worktree_relative && let Some(worktree) = worktree_dir.as_mut() {
                    *worktree = gix_path::relativize_with_prefix(worktree, current_dir).into_owned();
                }
                #[allow(unused_variables, reason = "Used when tracing is enabled at compile time.")]
                if let Some(worktree_path) = worktree_dir.as_deref().filter(|wtd| !wtd.is_dir()) {
                    gix_trace::warn!(
                        "The configured worktree path \"{}\" is not a directory or doesn't exist - `core.worktree` may be misleading",
                        worktree_path.display()
                    );
                }
                if worktree_from_environment {
                    is_bare = Some(false);
                }
            } else if !lenient_config
                && is_bare == Some(false)
                && resolved
                    .file
                    .boolean_filter(Core::WORKTREE, |section| {
                        is_eligible_worktree_config_section(section, &git_dir, current_dir, &mut filter_config_section)
                    })
                    .or_raise(|| config::key::error(&Core::WORKTREE, "Invalid configuration value"))?
                    .is_some()
            {
                bail!(config::key::error(&Core::WORKTREE, "Invalid configuration value"));
            }

            // Without an explicit path, a non-bare `.git` directory implies its parent as worktree.
            if worktree_dir.is_none()
                && is_bare == Some(false)
                && refs.git_dir().file_name() == Some(OsStr::new(gix_discover::DOT_GIT_DIR))
            {
                worktree_dir = Some(git_dir.parent().expect("parent is always available").to_owned());
            }
            if is_bare == Some(true) && !is_linked_worktree {
                // Linked worktrees may inherit core.bare=true from their common repository.
                // Other checkouts are suppressed unless the caller is opening a submodule checkout.
                if retain_inferred_worktree_when_bare {
                    // Preserve the submodule checkout before checking its ownership, not after initialization.
                    if let Some(worktree) = worktree_dir.as_ref() {
                        _ = std::fs::read_dir(worktree).or_error()?;
                    }
                } else {
                    worktree_dir = None;
                }
            }

            // A configured checkout can differ from the one used to determine initial trust.
            if let RepositoryTrust::Precomputed(trust @ Trust::Full) = git_dir_trust
                && let Some(worktree) = worktree_dir.as_deref()
                && !gix_sec::identity::is_path_owned_by_current_user(worktree).unwrap_or(false)
                && let Err(err) = check_safe_directory(worktree)
            {
                if bail_if_untrusted {
                    return Err(err);
                }
                *trust = Trust::Reduced;
                update_repository_section_trust(&mut resolved.file, Trust::Reduced, is_safe);
                // Consult core.worktree again with the final section trust.
                continue;
            }
            resolved.is_bare = is_bare;
            break worktree_dir;
        };

        let mut config =
            config::Cache::from_resolved(resolved, filter_config_section, *env, attributes, lenient_config)
                .or_raise(|| message("Repository configuration could not be loaded"))?;
        let use_on_windows = cfg!(windows);
        refs.prohibit_windows_device_names = gitoxide::Core::PROTECT_WINDOWS
            .enrich_error(
                config
                    .resolved
                    .boolean_filter(gitoxide::Core::PROTECT_WINDOWS, &mut filter_config_section),
            )
            .with_lenient_default_value(lenient_config, Some(use_on_windows))?
            .unwrap_or(use_on_windows);

        let index_path = match config
            .resolved
            .raw_value_with_section_filter(gitoxide::Core::INDEX_FILE, &mut filter_config_section)
            .ok()
        {
            Some((value, section)) => {
                gitoxide::Core::INDEX_FILE.validate(value.as_bstr())?;
                let path = gix_path::from_bstr(value)?;
                match section.meta().source {
                    gix_config::Source::EnvOverride => path.into_owned(),
                    _ => git_dir.join(path),
                }
            }
            None => git_dir.join("index"),
        };
        let index_path = absolute_path(&index_path, current_dir)?;
        if git_dir.is_absolute() {
            worktree_dir = worktree_dir.map(|path| absolute_path(&path, current_dir)).transpose()?;
        }

        refs.write_reflog = config::cache::util::reflog_or_default(config.reflog, worktree_dir.is_some());
        refs.shared_repository_permissions = config.shared_repository_permissions;
        refs.namespace.clone_from(&config.refs_namespace);
        let prefix = replacement_objects_refs_prefix(&config.resolved, lenient_config, filter_config_section)?;

        if git_dir_trust.level() == Trust::Reduced && config.alloc_limit_bytes.is_none() {
            let alloc_limit_if_reduced_trust =
                match gitoxide::Objects::ALLOC_LIMIT_IF_REDUCED_TRUST.try_into_usize(config.resolved.integer_filter(
                    gitoxide::Objects::ALLOC_LIMIT_IF_REDUCED_TRUST,
                    &mut filter_config_section,
                )) {
                    Ok(Some(value)) => value,
                    Ok(None) => gitoxide::Objects::ALLOC_LIMIT_IF_REDUCED_TRUST_DEFAULT,
                    Err(_) if config.lenient_config => gitoxide::Objects::ALLOC_LIMIT_IF_REDUCED_TRUST_DEFAULT,
                    Err(err) => return Err(err),
                };
            if alloc_limit_if_reduced_trust != 0 {
                config.alloc_limit_bytes = Some(alloc_limit_if_reduced_trust);
                gix_trace::info!(
                    concat!(
                        "Applied a default allocation limit of {alloc_limit_bytes} ",
                        "bytes while opening reduced-trust repository \"{git_dir}\". ",
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
                        .filter_map(std::result::Result::ok)
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
            objects: OwnShared::new(
                gix_odb::Store::at_opts(
                    common_dir_ref.join("objects"),
                    config.object_hash,
                    &mut replacements.into_iter(),
                    gix_odb::store::init::Options {
                        slots: object_store_slots,
                        use_multi_pack_index: config.use_multi_pack_index,
                        alloc_limit_bytes: config.alloc_limit_bytes,
                        loose_compression: config.loose_compression,
                        shared_repository_permissions: config.shared_repository_permissions,
                        current_dir: current_dir.to_owned().into(),
                    },
                )
                .or_error()?,
            ),
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

/// Apply the current repository `trust` to the resolved config's file metadata and repository-local sections.
///
/// Under reduced repository trust, a section still receives full trust if `is_safe` approves its source
/// file through protected configuration. Sections from non-repository sources keep their existing trust.
fn update_repository_section_trust(config: &mut gix_config::File, trust: Trust, is_safe: impl Fn(&Path) -> bool) {
    if config.meta().trust != trust {
        config.set_meta(config.meta().clone().with(trust));
    }
    let section_ids: Vec<_> = config.section_ids().collect();
    let mut safe_by_path = BTreeMap::new();
    for id in section_ids {
        let Some(mut section) = config.section_mut_by_id(id) else {
            continue;
        };
        let meta = section.meta();
        if meta.source.kind() != gix_config::source::Kind::Repository {
            continue;
        }
        let section_trust = if trust == Trust::Full
            || meta
                .path
                .as_deref()
                .is_some_and(|path| *safe_by_path.entry(path.to_owned()).or_insert_with(|| is_safe(path)))
        {
            Trust::Full
        } else {
            Trust::Reduced
        };
        if meta.trust != section_trust {
            section.set_trust(section_trust);
        }
    }
}

/// Anchor `path` lexically to the captured `current_dir`, without requiring it to exist.
fn absolute_path(path: &Path, current_dir: &Path) -> Result<PathBuf> {
    let absolute = current_dir.join(path);
    if !absolute.is_absolute() {
        bail!(validation(format!(
            "Cannot make path '{}' absolute using captured directory '{}'",
            path.display(),
            current_dir.display()
        )));
    }
    Ok(absolute)
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
    // Compare normalized paths without changing symlink-sensitive filesystem path spelling.
    let Some(git_dir) = gix_path::normalize(current_dir.join(git_dir).into(), current_dir) else {
        return false;
    };
    section
        .path
        .as_deref()
        .and_then(|path| gix_path::normalize(current_dir.join(path).into(), current_dir))
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

fn replacement_objects_refs_prefix(
    config: &gix_config::File,
    lenient: bool,
    mut filter_config_section: fn(&gix_config::file::Metadata) -> bool,
) -> Result<Option<BString>> {
    let is_enabled = config::shared::is_replace_refs_enabled(config, lenient, filter_config_section)?.unwrap_or(true);

    if !is_enabled {
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
) -> Result<()> {
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
                    Err(_) => gix_path::from_bstr(safe_dir)?.into_owned(),
                };
            if !safe_dir.is_absolute() {
                gix_trace::warn!(
                    "safe.directory \"{safe_dir}\" not absolute",
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
        Err(message!(
            "The git directory at \"{}\" is considered unsafe as it's not owned by the current user.",
            path_to_test.display()
        )
        .permission_denied_error())
    }
}

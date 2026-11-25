use std::{
    borrow::Cow,
    ops::ControlFlow,
    path::{Path, PathBuf},
};

use gix_error::{ResultExt, message};
use gix_parallel::OwnShared;

use super::{Options, RelatedRepository, RelatedRepositoryKind, RepositoryTrust};
use crate::{Repository, Result, bstr::BString, open::Permissions};

impl Default for Options {
    fn default() -> Self {
        Options {
            object_store_slots: Default::default(),
            permissions: Default::default(),
            git_dir_trust: None,
            configure_related_repository: None,
            config_overrides_by_trust: None,
            filter_config_section: None,
            lossy_config: false,
            lenient_config: true,
            bail_if_untrusted: false,
            open_path_as_is: false,
            api_config_overrides: Vec::new(),
            cli_config_overrides: Vec::new(),
            git_installation_config_path: None,
            system_config_path: None,
            use_repository_local_environment: true,
            retain_inferred_worktree_when_bare: false,
            current_dir: None,
        }
    }
}

/// Instantiation
impl Options {
    /// Options configured to prevent accessing anything else than the repository configuration file, prohibiting
    /// accessing the environment or spreading beyond the git repository location.
    pub fn isolated() -> Self {
        Options::default().permissions(Permissions::isolated())
    }

    /// Clone all options, retaining the trust override for reopening the same repository.
    pub(crate) fn clone_with_trust(&self) -> Self {
        Options {
            object_store_slots: self.object_store_slots,
            permissions: self.permissions,
            git_dir_trust: self.git_dir_trust,
            configure_related_repository: self.configure_related_repository.clone(),
            config_overrides_by_trust: self.config_overrides_by_trust.clone(),
            filter_config_section: self.filter_config_section,
            lossy_config: self.lossy_config,
            lenient_config: self.lenient_config,
            bail_if_untrusted: self.bail_if_untrusted,
            api_config_overrides: self.api_config_overrides.clone(),
            cli_config_overrides: self.cli_config_overrides.clone(),
            git_installation_config_path: self.git_installation_config_path.clone(),
            system_config_path: self.system_config_path.clone(),
            use_repository_local_environment: self.use_repository_local_environment,
            open_path_as_is: self.open_path_as_is,
            retain_inferred_worktree_when_bare: self.retain_inferred_worktree_when_bare,
            current_dir: self.current_dir.clone(),
        }
    }

    /// Copy options for opening another repository, clearing trust, the captured CWD, and one-shot opening policies.
    /// Use [`Repository::reload()`][crate::Repository::reload()] or
    /// [`reload_with()`][crate::Repository::reload_with()] to reopen the same repository.
    /// Permissions are preserved, but repository-local environment overrides are disabled.
    pub fn clone_without_trust(&self) -> Self {
        Options {
            git_dir_trust: None,
            current_dir: None,
            open_path_as_is: false,
            retain_inferred_worktree_when_bare: false,
            ..self.clone_with_trust()
        }
        .without_repository_environment_overrides()
    }
}

/// Generic modification
impl Options {
    /// An adapter to allow calling any builder method on this instance despite only having a mutable reference.
    pub fn modify(&mut self, f: impl FnOnce(Self) -> Self) {
        *self = f(std::mem::take(self));
    }
}

/// Builder methods
impl Options {
    /// Configure opens of submodules and related worktrees initiated by this repository.
    ///
    /// The callback runs after path resolution and before loading the target's configuration and takes
    /// ownership of the options the target would otherwise open with.
    /// The callback is inherited with the opening options and passed to sub-repositories.
    ///
    /// Return [`ControlFlow::Continue`] with the options to use or [`ControlFlow::Break`] with `()`
    /// to skip the current open. `Submodule::open()` reports a skip as `Ok(None)`. Operations that require
    /// opening the repository report a skip as an error. Callback errors propagate to the caller.
    pub fn configure_related_repository(
        mut self,
        configure: impl Fn(RelatedRepository<'_>, Self) -> Result<ControlFlow<(), Self>> + Send + Sync + 'static,
    ) -> Self {
        self.configure_related_repository = Some(OwnShared::new(configure));
        self
    }

    /// Apply the given configuration `values` like `init.defaultBranch=special` or `core.bool-implicit-true` in memory to as early
    /// as the configuration is initialized to allow affecting the repository instantiation phase, both on disk or when opening.
    /// The configuration is marked with [source API][gix_config::Source::Api].
    pub fn config_overrides(mut self, values: impl IntoIterator<Item = impl Into<BString>>) -> Self {
        self.api_config_overrides = values.into_iter().map(Into::into).collect();
        self
    }

    /// Replace the configuration policy based on the repository's final trust level.
    ///
    /// The callback receives the configuration resolved during opening, including ordinary overrides.
    /// Return assignments like `index.threads=1`, or an empty vector to leave that configuration as is.
    /// Assignments are appended as [API configuration][gix_config::Source::Api],
    /// taking precedence over config files, CLI arguments, and environment overrides in ordinary lookups.
    /// The usual section filters and key-specific rules still apply.
    /// Any callback or assignment error fails the open.
    ///
    /// By default, [`Full`][gix_sec::Trust::Full] adds no overrides. [`Reduced`][gix_sec::Trust::Reduced]
    /// caps pack processing, index decoding, and checkout at four workers (or fewer if selected automatically),
    /// and caps each configured object or pack cache and the default pack cache at 16 MiB.
    /// Lower explicit limits are preserved. The policy also selects `diff.algorithm=histogram` and restricts
    /// selected transport and repository settings.
    /// This policy runs after opening has selected the Git directory, object format, checkout, and trust level.
    /// It is inherited by related repositories and run afresh for each open or reload.
    pub fn config_overrides_by_trust(
        mut self,
        configure: impl Fn(gix_sec::Trust, &gix_config::File) -> Result<Vec<BString>> + Send + Sync + 'static,
    ) -> Self {
        self.config_overrides_by_trust = Some(OwnShared::new(configure));
        self
    }

    /// Set configuration values of the form `core.abbrev=5` or `remote.origin.url = foo` or `core.bool-implicit-true` for application
    /// as CLI overrides to the repository configuration, marked with [source CLI][gix_config::Source::Cli].
    /// These are equivalent to CLI overrides passed with `-c` in `git`, for example.
    pub fn cli_overrides(mut self, values: impl IntoIterator<Item = impl Into<BString>>) -> Self {
        self.cli_config_overrides = values.into_iter().map(Into::into).collect();
        self
    }

    /// Use `path` as Git's installation configuration file instead of discovering it.
    ///
    /// Common locations are `$(prefix)/etc/gitconfig` on Unix,
    /// `/Applications/Xcode.app/Contents/Developer/usr/share/git-core/gitconfig` for Apple Git,
    /// and `C:/Program Files/Git/etc/gitconfig` for Git for Windows.
    ///
    /// The file is loaded only when [`permissions.config.git_binary`][Permissions::config] is enabled.
    pub fn git_installation_config_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.git_installation_config_path = Some(path.into().into_boxed_path());
        self
    }

    /// Use `path` as the system configuration file instead of discovering it.
    ///
    /// Common locations are `/etc/gitconfig` on Unix and `C:/ProgramData/Git/config` on Windows.
    ///
    /// The file is loaded only when [`permissions.config.system`][Permissions::config] is enabled.
    pub fn system_config_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.system_config_path = Some(path.into().into_boxed_path());
        self
    }

    /// Set the amount of slots to use for the object database. It's a value that doesn't need changes on the client, typically,
    /// but should be controlled on the server.
    pub fn object_store_slots(mut self, slots: gix_odb::store::init::Slots) -> Self {
        self.object_store_slots = slots;
        self
    }

    // TODO: tests
    /// Set the given permissions, which are typically derived by a `Trust` level.
    pub fn permissions(mut self, permissions: Permissions) -> Self {
        self.permissions = permissions;
        self
    }

    /// If `true`, default `false`, we will not modify the incoming path for this open call to assure it is a `.git` directory.
    ///
    /// If `true`, we will try to open the input directory as is, even though it doesn't appear to be a `git` repository
    /// due to the lack of `.git` suffix or because its basename is not `.git` as in `worktree/.git`.
    /// This option is consumed while resolving the path and isn't retained in the opened repository's options.
    pub fn open_path_as_is(mut self, enable: bool) -> Self {
        self.open_path_as_is = enable;
        self
    }

    /// Set the initial trust level when [opening a repository][Self::open], including after [discovery][Self::discover].
    /// Pass `None` to use the trust determined from ownership or supplied by discovery.
    ///
    /// When opening without an upward search, supplying a level skips the ownership checks of the repository path
    /// before resolving any gitfile, the resolved Git directory, its common directory (if separate), and the
    /// initially identified worktree (if it exists). It does not change the [upward search's trust policy][crate::discover_opts()].
    ///
    /// Opening still considers `safe.directory` and the ownership of the final worktree. The final worktree
    /// can differ because `core.worktree` is read after the initial path is identified and can point to a
    /// different checkout.
    ///
    /// Only supply [`Full`][gix_sec::Trust::Full] if you trust the repository, since its configuration can
    /// supply values such as external commands.
    ///
    /// See [`with_enforced_trust()`][Self::with_enforced_trust()] to set the final trust level.
    pub fn with_trust(mut self, trust: impl Into<Option<gix_sec::Trust>>) -> Self {
        self.git_dir_trust = trust.into().map(RepositoryTrust::Precomputed);
        self
    }

    /// Enforce `trust` when [opening a repository][Self::open], including after [discovery][Self::discover].
    /// Pass `None` to use the trust determined from ownership or supplied by discovery.
    ///
    /// An enforced level skips opening's repository and worktree ownership checks and `safe.directory` approval.
    /// It does not change the [upward search's trust policy][crate::discover_opts()].
    /// [Reloading][crate::Repository::reload()] retains it. Submodules and other worktrees determine their own trust.
    /// [`bail_if_untrusted(true)`][Self::bail_if_untrusted] rejects an enforced [`Reduced`][gix_sec::Trust::Reduced] level.
    ///
    /// See [`with_trust()`][Self::with_trust()] to set the initial level while keeping the later trust checks.
    ///
    /// # Security
    ///
    /// Enforce [`Full`][gix_sec::Trust::Full] only if you already trust the repository, its configuration,
    /// and its configured checkout. With the default configuration filter, sensitive reads can then use
    /// repository-local settings that would otherwise be ignored for a repository with reduced trust.
    ///
    /// An attacker who controls the repository can influence later operations:
    ///
    /// - **Run commands with the caller's privileges.**
    ///   - Network operations can invoke configured SSH commands, credential helpers, or askpass programs.
    ///   - Worktree and content operations can invoke clean, smudge, and process filters; diff operations
    ///     can invoke external diff commands and text converters; merges can invoke external merge drivers.
    ///   - Operations that request an editor can launch the configured editor. Signing and verification
    ///     can invoke configured signature programs, and SSH signing can run a command to select the key.
    /// - **Send repository data to an unintended remote.**
    ///   - A later push can use a destination from the repository's configuration.
    /// - **Redirect worktree reads and writes.**
    ///   - `core.worktree` can point to a different checkout whose ownership is not checked.
    /// - **Exhaust process memory.**
    ///   - Processing doctored repository files can consume large amounts of memory and cause an
    ///     out-of-memory failure. `gix` does not impose an overall memory limit, regardless of trust level.
    ///
    /// If you cannot vouch for the repository and checkout, leave trust unspecified so ownership and
    /// `safe.directory` checks still apply. Use process-level resource limits when handling untrusted input.
    pub fn with_enforced_trust(mut self, trust: impl Into<Option<gix_sec::Trust>>) -> Self {
        self.git_dir_trust = trust.into().map(RepositoryTrust::Enforced);
        self
    }

    /// Reject repositories with a final trust level of [`Reduced`][gix_sec::Trust::Reduced]. Defaults to `false`.
    ///
    /// Use this to mimic `git`s way of handling untrusted repositories. Note that `gitoxide` solves
    /// this by not using configuration from untrusted sources and by generally being secured against
    /// doctored input files which at worst could cause out-of-memory at the time of writing.
    pub fn bail_if_untrusted(mut self, toggle: bool) -> Self {
        self.bail_if_untrusted = toggle;
        self
    }

    /// Set the filter which determines if a configuration section can be used to read values from,
    /// hence it returns true if it is eligible.
    ///
    /// The default filter selects sections whose trust level is [`full`][gix_sec::Trust::Full] or
    /// whose source is not [`repository-local`][gix_config::source::Kind::Repository].
    ///
    /// # Security
    ///
    /// A custom filter replaces the default and can admit untrusted configuration without changing the repository's
    /// trust level. See the security warning in [`with_enforced_trust()`][Self::with_enforced_trust()].
    pub fn filter_config_section(mut self, filter: fn(&gix_config::file::Metadata) -> bool) -> Self {
        self.filter_config_section = Some(filter);
        self
    }

    /// If set, default is false, configuration will be read without retaining non-essential information like comments
    /// or whitespace to optimize lookup performance.
    ///
    /// This will prevent displaying or editing configuration losslessly.
    pub fn lossy_config(mut self, toggle: bool) -> Self {
        self.lossy_config = toggle;
        self
    }

    /// If set, default is false, invalid configuration values will cause an error even if these can safely be defaulted.
    ///
    /// This is recommended for all applications that prefer correctness over usability.
    /// `git` itself defaults to strict configuration mode, flagging incorrect configuration immediately.
    ///
    /// Failure to read configuration files due to IO errors will also be a hard error if this mode is enabled, otherwise
    /// these errors will merely be logged.
    pub fn strict_config(mut self, toggle: bool) -> Self {
        self.lenient_config = !toggle;
        self
    }
}

/// Lifecycle
impl Options {
    /// Open a repository at `path` with these options, without searching parent directories.
    ///
    /// As in [`crate::open_opts()`], `path` can be a worktree or a Git directory.
    pub fn open(self, path: impl Into<PathBuf>) -> Result<Repository> {
        crate::open_opts(path, self)
    }
}

impl Options {
    pub(crate) fn configure_related_open(
        mut self,
        parent: &Repository,
        kind: RelatedRepositoryKind,
        git_dir: &Path,
        inferred_worktree: Option<&Path>,
    ) -> Result<ControlFlow<(), Self>> {
        fn anchored<'a>(path: &'a Path, current_dir: &Path) -> Cow<'a, Path> {
            if path.is_absolute() {
                Cow::Borrowed(path)
            } else {
                Cow::Owned(current_dir.join(path))
            }
        }
        let Some(configure) = self.configure_related_repository.clone() else {
            return Ok(ControlFlow::Continue(self));
        };
        let trust = self.git_dir_trust.expect("initial trust has been determined");
        let current_dir = self.current_dir.take().expect("the opening CWD has been captured");
        let retain_inferred_worktree_when_bare = self.retain_inferred_worktree_when_bare;
        let config_overrides_by_trust = self.config_overrides_by_trust.clone();
        let git_dir = anchored(git_dir, &current_dir);
        let inferred_worktree = inferred_worktree.map(|path| anchored(path, &current_dir));
        let ControlFlow::Continue(mut options) = configure(
            RelatedRepository {
                kind,
                git_dir: &git_dir,
                inferred_worktree: inferred_worktree.as_deref(),
                initial_trust: trust.level(),
                parent,
            },
            self,
        )?
        else {
            return Ok(ControlFlow::Break(()));
        };
        // The callback can replace the entire options value.
        options.configure_related_repository.get_or_insert(configure);
        options.config_overrides_by_trust = options.config_overrides_by_trust.or(config_overrides_by_trust);
        options.git_dir_trust.get_or_insert(trust);
        options.current_dir = Some(current_dir);
        options.use_repository_local_environment = false;
        options.retain_inferred_worktree_when_bare = retain_inferred_worktree_when_bare;
        Ok(ControlFlow::Continue(options))
    }

    /// Prevent repository-local environment overrides from being inherited when opening another repository.
    ///
    /// To be used when opening a nested repository, like worktrees or submodules.
    pub(crate) fn without_repository_environment_overrides(mut self) -> Self {
        self.use_repository_local_environment = false;
        self
    }

    /// Related worktrees can share computed repository trust. A caller's override applies only to the original one.
    pub(crate) fn clone_for_linked_worktree(&self) -> Self {
        let mut options = self.clone_with_trust().without_repository_environment_overrides();
        if matches!(options.git_dir_trust, Some(RepositoryTrust::Enforced(_))) {
            options.git_dir_trust = None;
        }
        options
    }

    pub(crate) fn from_trust_callback(
        trust: gix_sec::Trust,
        open_options_by_trust: impl FnOnce(Self, gix_sec::Trust) -> Self,
    ) -> Self {
        let mut options = open_options_by_trust(Self::default_for_level(trust), trust);
        options.git_dir_trust.get_or_insert(RepositoryTrust::Precomputed(trust));
        options
    }

    pub(crate) fn current_dir_or_empty(&self) -> &std::path::Path {
        self.current_dir.as_deref().unwrap_or(std::path::Path::new(""))
    }
}

impl Options {
    /// Return default opening options for `level`, setting it as the initial repository trust.
    /// Only supply [`Full`][gix_sec::Trust::Full] if you already trust the repository; see [`Self::with_trust()`].
    pub fn default_for_level(level: gix_sec::Trust) -> Self {
        match level {
            gix_sec::Trust::Full => Options {
                object_store_slots: Default::default(),
                permissions: Permissions::default_for_level(level),
                git_dir_trust: Some(RepositoryTrust::Precomputed(level)),
                configure_related_repository: None,
                config_overrides_by_trust: None,
                retain_inferred_worktree_when_bare: false,
                filter_config_section: None,
                lossy_config: false,
                bail_if_untrusted: false,
                lenient_config: true,
                open_path_as_is: false,
                api_config_overrides: Vec::new(),
                cli_config_overrides: Vec::new(),
                git_installation_config_path: None,
                system_config_path: None,
                use_repository_local_environment: true,
                current_dir: None,
            },
            gix_sec::Trust::Reduced => Options {
                object_store_slots: gix_odb::store::init::Slots::Given(32), // limit resource usage
                permissions: Permissions::default_for_level(level),
                git_dir_trust: Some(RepositoryTrust::Precomputed(level)),
                configure_related_repository: None,
                config_overrides_by_trust: None,
                retain_inferred_worktree_when_bare: false,
                filter_config_section: None,
                bail_if_untrusted: false,
                lenient_config: true,
                open_path_as_is: false,
                lossy_config: false,
                api_config_overrides: Vec::new(),
                cli_config_overrides: Vec::new(),
                git_installation_config_path: None,
                system_config_path: None,
                use_repository_local_environment: true,
                current_dir: None,
            },
        }
    }
}

pub(super) fn apply_reduced_trust_config(
    config: &mut gix_config::File,
    filter_config_section: fn(&gix_config::file::Metadata) -> bool,
    lenient: bool,
) -> Result<()> {
    use crate::config::{
        self,
        cache::util::{ApplyLeniency, parse_object_caches},
        tree::{Checkout, Core, Index, Key, Pack, gitoxide},
    };

    const MAX_WORKERS: usize = 4;
    const MAX_CACHE_BYTES: usize = 16 * 1024 * 1024;

    let automatic_workers = gix_parallel::num_threads(None).min(MAX_WORKERS);
    let mut overrides = Vec::<BString>::new();
    let worker_counts: [(&dyn Key, Option<usize>); 3] = [
        (
            &Pack::THREADS,
            Pack::THREADS
                .try_into_usize(config.integer_filter(Pack::THREADS, filter_config_section))
                .with_leniency(lenient)?,
        ),
        (
            &Index::THREADS,
            // Index decoding reads this key even from reduced-trust sections.
            config
                .string(Index::THREADS)
                .map(|value| Index::THREADS.try_into_index_threads(value))
                .transpose()
                .with_leniency(lenient)?,
        ),
        (
            &Checkout::WORKERS,
            config
                .integer_filter(&Checkout::WORKERS as &dyn Key, filter_config_section)
                .or_raise(|| config::key::error(&Checkout::WORKERS, "Could not decode checkout workers"))
                .with_leniency(lenient)?
                // Checkout also treats negative counts as automatic. Oversized positive values must be capped
                // before checkout converts them to usize, which can be narrower than i64.
                .filter(|count| *count > 0)
                .map(|count| usize::try_from(count).unwrap_or(usize::MAX)),
        ),
    ];
    for (key, count) in worker_counts {
        let capped = match count {
            None | Some(0) => automatic_workers,
            Some(count) => count.min(MAX_WORKERS),
        };
        if count != Some(capped) {
            overrides.push(format!("{}={capped}", key.logical_name()).into());
        }
    }

    let (default_pack_bytes, delta_bytes, object_bytes) = parse_object_caches(config, lenient, filter_config_section)?;
    for (key, bytes) in [
        // An unset default pack cache has a budget above the cap. Leave an absent delta budget unset
        // so it still selects the default pack cache, and preserve zero as an explicit opt-out.
        (
            &gitoxide::Core::DEFAULT_PACK_CACHE_MEMORY_LIMIT,
            default_pack_bytes.unwrap_or(usize::MAX),
        ),
        (&Core::DELTA_BASE_CACHE_LIMIT, delta_bytes.unwrap_or_default()),
        (&gitoxide::Objects::CACHE_LIMIT, object_bytes),
    ] {
        if bytes > MAX_CACHE_BYTES {
            overrides.push(format!("{}={MAX_CACHE_BYTES}", key.logical_name()).into());
        }
    }

    let fixed = gix_config::File::try_from(REDUCED_TRUST_CONFIG)
        .or_raise(|| message("Could not parse default repository trust configuration overrides"))?;
    config.append(fixed)?;
    if !overrides.is_empty() {
        config::overrides::append(config, &overrides, gix_config::Source::Api, |_| {
            Some("Capped by the default reduced-trust resource policy".into())
        })?;
    }
    Ok(())
}

const REDUCED_TRUST_CONFIG: &str = r#"[core]
	useReplaceRefs = false # Prevent repository refs from substituting objects during lookup.
[index]
	skipHash = false # Verify the index checksum when one is present.
[diff]
	algorithm = histogram # Bound the search on repeated lines and fall back to heuristic Myers.
[gitoxide "allow"]
	protocolFromUser = false # Require explicit permission for protocols with a user-only policy.
[protocol "ext"]
	allow = never # Block the transport that can run external commands.
[http]
	sslVerify = true # Authenticate HTTPS servers by verifying their certificates.
	followRedirects = false # Prevent the server from redirecting requests to a different URL.
[gitoxide "http"]
	sslNoVerify = false # Prevent GIT_SSL_NO_VERIFY from bypassing TLS certificate checks.
[gitoxide "credentials"]
	terminalPrompt = false # Keep repository-triggered requests from prompting for credentials.
[fetch]
	recurseSubmodules = false # Avoid fetching submodule repositories automatically.
	fsckObjects = true # Request validation of objects received by fetch.
[submodule]
	recurse = false # Keep ordinary commands from recursing into submodule repositories.
[fsmonitor]
	allowRemote = false # Avoid starting filesystem monitoring on network-mounted worktrees.
[receive]
	fsckObjects = true # Request validation of objects received by push.
[transfer]
	fsckObjects = true # Request object validation when the fetch or receive setting is unset.
"#;

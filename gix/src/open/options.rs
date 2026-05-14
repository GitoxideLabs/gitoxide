use std::path::PathBuf;

use super::{Error, Options};
use crate::{ThreadSafeRepository, bstr::BString, config, open::Permissions};

impl Default for Options {
    fn default() -> Self {
        Options {
            object_store_slots: Default::default(),
            permissions: Default::default(),
            git_dir_trust: None,
            filter_config_section: None,
            lossy_config: false,
            lenient_config: true,
            bail_if_untrusted: false,
            open_path_as_is: false,
            api_config_overrides: Vec::new(),
            cli_config_overrides: Vec::new(),
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
    /// Apply the given configuration `values` like `init.defaultBranch=special` or `core.bool-implicit-true` in memory to as early
    /// as the configuration is initialized to allow affecting the repository instantiation phase, both on disk or when opening.
    /// The configuration is marked with [source API][gix_config::Source::Api].
    pub fn config_overrides(mut self, values: impl IntoIterator<Item = impl Into<BString>>) -> Self {
        self.api_config_overrides = values.into_iter().map(Into::into).collect();
        self
    }

    /// Set configuration values of the form `core.abbrev=5` or `remote.origin.url = foo` or `core.bool-implicit-true` for application
    /// as CLI overrides to the repository configuration, marked with [source CLI][gix_config::Source::Cli].
    /// These are equivalent to CLI overrides passed with `-c` in `git`, for example.
    pub fn cli_overrides(mut self, values: impl IntoIterator<Item = impl Into<BString>>) -> Self {
        self.cli_config_overrides = values.into_iter().map(Into::into).collect();
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

    /// If `true`, default `false`, we will not modify the incoming path to open to assure it is a `.git` directory.
    ///
    /// If `false`, we will try to open the input directory as is, even though it doesn't appear to be a `git` repository
    /// due to the lack of `.git` suffix or because its basename is not `.git` as in `worktree/.git`.
    pub fn open_path_as_is(mut self, enable: bool) -> Self {
        self.open_path_as_is = enable;
        self
    }

    /// Set the trust level of the `.git` directory we are about to open.
    ///
    /// This can be set manually to force trust even though otherwise it might
    /// not be fully trusted, leading to limitations in how configuration files
    /// are interpreted.
    ///
    /// If not called explicitly, it will be determined by looking at its
    /// ownership via [`gix_sec::Trust::from_path_ownership()`].
    ///
    /// # Security Warning
    ///
    /// Use with extreme care and only if it's absolutely known that the repository
    /// is always controlled by the desired user. Using this capability _only_ saves
    /// a permission check and only so if the [`open()`][Self::open()] method is used,
    /// as opposed to discovery.
    pub fn with(mut self, trust: gix_sec::Trust) -> Self {
        self.git_dir_trust = trust.into();
        self
    }

    /// If true, default false, and if the repository's trust level is not `Full`
    /// (see [`with()`][Self::with()] for more), then the open operation will fail.
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

    /// Open a repository at `path` with the options set so far.
    #[allow(clippy::result_large_err)]
    pub fn open(self, path: impl Into<PathBuf>) -> Result<ThreadSafeRepository, Error> {
        ThreadSafeRepository::open_opts(path, self)
    }
}

impl Options {
    pub(crate) fn current_dir_or_empty(&self) -> &std::path::Path {
        self.current_dir.as_deref().unwrap_or(std::path::Path::new(""))
    }
}

impl gix_sec::trust::DefaultForLevel for Options {
    fn default_for_level(level: gix_sec::Trust) -> Self {
        match level {
            gix_sec::Trust::Full => Options {
                object_store_slots: Default::default(),
                permissions: Permissions::default_for_level(level),
                git_dir_trust: gix_sec::Trust::Full.into(),
                filter_config_section: Some(config::section::is_trusted),
                lossy_config: false,
                bail_if_untrusted: false,
                lenient_config: true,
                open_path_as_is: false,
                api_config_overrides: Vec::new(),
                cli_config_overrides: Vec::new(),
                current_dir: None,
            },
            gix_sec::Trust::Reduced => Options {
                object_store_slots: gix_odb::store::init::Slots::Given(32), // limit resource usage
                permissions: Permissions::default_for_level(level),
                git_dir_trust: gix_sec::Trust::Reduced.into(),
                filter_config_section: Some(config::section::is_trusted),
                bail_if_untrusted: false,
                lenient_config: true,
                open_path_as_is: false,
                lossy_config: false,
                api_config_overrides: Vec::new(),
                cli_config_overrides: Vec::new(),
                current_dir: None,
            },
        }
    }
}

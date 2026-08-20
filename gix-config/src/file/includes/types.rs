use crate::path::interpolate;

/// The error returned when following includes.
pub type Error = gix_error::Exn;

/// Options to handle includes, like `include.path` or `includeIf.<condition>.path`,
#[derive(Clone, Copy)]
pub struct Options<'a> {
    /// The maximum allowed length of the file include chain built by following nested resolve_includes where base level is depth = 0.
    pub max_depth: u8,
    /// When max depth is exceeded while following nested includes,
    /// return an error if true or silently stop following resolve_includes.
    ///
    /// Setting this value to false allows to read configuration with cycles,
    /// which otherwise always results in an error.
    pub err_on_max_depth_exceeded: bool,
    /// If true, default false, failing to interpolate paths will result in an error.
    ///
    /// Interpolation also happens if paths in conditional includes can't be interpolated.
    pub err_on_interpolation_failure: bool,
    /// If true, default true, configuration not originating from a path will cause errors when trying to resolve
    /// relative include paths (which would require the including configuration's path).
    pub err_on_missing_config_path: bool,
    /// Used during path interpolation, both for include paths before trying to read the file, and for
    /// paths used in conditional `gitdir` includes.
    pub interpolate: interpolate::Context<'a>,

    /// Additional context for conditional includes to work.
    pub conditional: conditional::Context<'a>,
}

impl<'a> Options<'a> {
    /// Provide options to never follow include directives at all.
    pub fn no_follow() -> Self {
        Options {
            max_depth: 0,
            err_on_max_depth_exceeded: false,
            err_on_interpolation_failure: false,
            err_on_missing_config_path: false,
            interpolate: Default::default(),
            conditional: Default::default(),
        }
    }
    /// Provide options to follow includes like git does, provided the required `conditional` and `interpolate` contexts
    /// to support `gitdir` and `onbranch` based `includeIf` directives as well as standard `include.path` resolution.
    /// Note that the follow-mode is `git`-style, following at most 10 indirections while
    /// producing an error if the depth is exceeded.
    pub fn follow(interpolate: interpolate::Context<'a>, conditional: conditional::Context<'a>) -> Self {
        Options {
            max_depth: 10,
            err_on_max_depth_exceeded: true,
            err_on_interpolation_failure: false,
            err_on_missing_config_path: true,
            interpolate,
            conditional,
        }
    }

    /// For use with `follow` type options, cause failure if an include path couldn't be interpolated or the depth limit is exceeded.
    pub fn strict(mut self) -> Self {
        self.err_on_interpolation_failure = true;
        self.err_on_max_depth_exceeded = true;
        self.err_on_missing_config_path = true;
        self
    }

    /// Like [`follow`][Options::follow()], but without information to resolve `includeIf` directories as well as default
    /// configuration to allow resolving `~username/` path. `home_dir` is required to resolve `~/` paths if set.
    /// Note that `%(prefix)` paths cannot be interpolated with this configuration, use [`follow()`][Options::follow()]
    /// instead for complete control.
    pub fn follow_without_conditional(home_dir: Option<&'a std::path::Path>) -> Self {
        Options {
            max_depth: 10,
            err_on_max_depth_exceeded: true,
            err_on_interpolation_failure: false,
            err_on_missing_config_path: true,
            interpolate: interpolate::Context {
                git_install_dir: None,
                home_dir,
                home_for_user: Some(interpolate::home_for_user),
            },
            conditional: Default::default(),
        }
    }

    /// Set the context used for interpolation when interpolating paths to include as well as the paths
    /// in `gitdir` conditional includes.
    pub fn interpolate_with(mut self, context: interpolate::Context<'a>) -> Self {
        self.interpolate = context;
        self
    }
}

impl Default for Options<'_> {
    fn default() -> Self {
        Self::no_follow()
    }
}

///
pub mod conditional {
    /// Options to handle conditional includes like `includeIf.<condition>.path`.
    #[derive(Clone, Copy, Default)]
    pub struct Context<'a> {
        /// The location of the .git directory. If `None`, `gitdir` conditions cause an error.
        ///
        /// Used for conditional includes, e.g. `includeIf.gitdir:…` or `includeIf:gitdir/i…`.
        pub git_dir: Option<&'a std::path::Path>,
        /// The name of the branch that is currently checked out. If `None`, `onbranch` conditions cause an error.
        ///
        /// Used for conditional includes, e.g. `includeIf.onbranch:main.…`
        pub branch_name: Option<&'a gix_ref::FullNameRef>,
    }
}

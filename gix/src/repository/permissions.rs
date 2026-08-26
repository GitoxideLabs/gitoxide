//! Various permissions to define what can be done when operating a [`Repository`][crate::Repository].
use crate::open::Permissions;
use gix_sec::Trust;

/// Configure from which sources git configuration may be loaded.
///
/// Note that configuration from inside of the repository is always loaded as it's definitely required for correctness.
#[derive(Copy, Clone, Ord, PartialOrd, PartialEq, Eq, Debug, Hash)]
pub struct Config {
    /// The git binary may come with configuration as part of its configuration, and if this is true (default false)
    /// we will load the configuration of the git binary, if present and not a duplicate of the ones below.
    ///
    /// It's disabled by default as it may involve executing the git binary once per execution of the application.
    pub git_binary: bool,
    /// Whether to use the system configuration.
    /// This is defined as `$(prefix)/etc/gitconfig` on unix.
    pub system: bool,
    /// Whether to use the git application configuration.
    ///
    /// A platform defined location for where a user's git application configuration should be located.
    /// If `$XDG_CONFIG_HOME` is not set or empty, `$HOME/.config/git/config` will be used
    /// on unix.
    pub git: bool,
    /// Whether to use the user configuration.
    /// This is usually `~/.gitconfig` on unix.
    pub user: bool,
    /// Whether to use the configuration from environment variables.
    pub env: bool,
    /// Whether to follow include files are encountered in loaded configuration,
    /// via `include` and `includeIf` sections.
    pub includes: bool,
}

impl Config {
    /// Allow everything which usually relates to a fully trusted environment
    pub fn all() -> Self {
        Config {
            git_binary: false,
            system: true,
            git: true,
            user: true,
            env: true,
            includes: true,
        }
    }

    /// Load only configuration local to the git repository.
    pub fn isolated() -> Self {
        Config {
            git_binary: false,
            system: false,
            git: false,
            user: false,
            env: false,
            includes: false,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::all()
    }
}

/// Configure from which `gitattribute` files may be loaded.
///
/// Note that `.gitattribute` files from within the repository are always loaded.
#[derive(Copy, Clone, Ord, PartialOrd, PartialEq, Eq, Debug, Hash)]
pub struct Attributes {
    /// The git binary may come with attribute configuration in its installation directory, and if this is true (default false)
    /// we will load the configuration of the git binary.
    ///
    /// It's disabled by default as it involves executing the git binary once per execution of the application.
    pub git_binary: bool,
    /// Whether to use the system configuration.
    /// This is typically defined as `$(prefix)/etc/gitconfig`.
    pub system: bool,
    /// Whether to use the git application configuration.
    ///
    /// A platform defined location for where a user's git application configuration should be located.
    /// If `$XDG_CONFIG_HOME` is not set or empty, `$HOME/.config/git/attributes` will be used
    /// on unix.
    pub git: bool,
}

impl Attributes {
    /// Allow everything which usually relates to a fully trusted environment
    pub fn all() -> Self {
        Attributes {
            git_binary: false,
            system: true,
            git: true,
        }
    }

    /// Allow loading attributes that are local to the git repository.
    pub fn isolated() -> Self {
        Attributes {
            git_binary: false,
            system: false,
            git: false,
        }
    }
}

impl Default for Attributes {
    fn default() -> Self {
        Self::all()
    }
}

/// Permissions related to the usage of environment variables
#[derive(Debug, Clone, Copy)]
pub struct Environment {
    /// Control whether resources pointed to by `XDG_CONFIG_HOME` can be used when looking up common configuration values.
    ///
    /// Note that [`gix_sec::Permission::Forbid`] will cause the operation to abort if a resource is set via the XDG config environment.
    pub xdg_config_home: gix_sec::Permission,
    /// Control the way resources pointed to by the home directory (similar to `xdg_config_home`) may be used.
    pub home: gix_sec::Permission,
    /// Control if environment variables to configure the HTTP transport, like `http_proxy` may be used.
    ///
    /// Note that http-transport related environment variables prefixed with `GIT_` may also be included here
    /// if they match this category like `GIT_HTTP_USER_AGENT`.
    pub http_transport: gix_sec::Permission,
    /// Control if the `EMAIL` environment variables may be read.
    ///
    /// Note that identity related environment variables prefixed with `GIT_` may also be included here
    /// if they match this category.
    pub identity: gix_sec::Permission,
    /// Control if environment variables related to the object database are handled. This includes features and performance
    /// options alike.
    pub objects: gix_sec::Permission,
    /// Control if `GIT_*` prefixed environment variables can be used, **but only** if they are not contained in any
    /// other category. This is a catch-all for Git-prefixed variables.
    pub git_prefix: gix_sec::Permission,
    /// Control if environment variables not contained in any other category can be used, like `TERM`, `VISUAL`, and
    /// `EDITOR`.
    pub other: gix_sec::Permission,
    /// Control if resources pointed to by `SSH_*` prefixed environment variables can be used (like `SSH_ASKPASS`)
    pub ssh_prefix: gix_sec::Permission,
}

impl Environment {
    /// Allow access to the entire environment.
    pub fn all() -> Self {
        let allow = gix_sec::Permission::Allow;
        Environment {
            xdg_config_home: allow,
            home: allow,
            git_prefix: allow,
            other: allow,
            ssh_prefix: allow,
            http_transport: allow,
            identity: allow,
            objects: allow,
        }
    }

    /// Don't allow loading any environment variables.
    pub fn isolated() -> Self {
        let deny = gix_sec::Permission::Deny;
        Environment {
            xdg_config_home: deny,
            home: deny,
            ssh_prefix: deny,
            git_prefix: deny,
            other: deny,
            http_transport: deny,
            identity: deny,
            objects: deny,
        }
    }
}

impl Permissions {
    /// Secure permissions are similar to `all()`
    pub fn secure() -> Self {
        Permissions {
            env: Environment::all(),
            config: Config::all(),
            attributes: Attributes::all(),
        }
    }

    /// Everything is allowed with this set of permissions, thus we read all configuration and do what git typically
    /// does with owned repositories.
    pub fn all() -> Self {
        Permissions {
            env: Environment::all(),
            config: Config::all(),
            attributes: Attributes::all(),
        }
    }

    /// Don't read any but the local git configuration and deny reading any environment variables.
    pub fn isolated() -> Self {
        Permissions {
            config: Config::isolated(),
            attributes: Attributes::isolated(),
            env: Environment::isolated(),
        }
    }
}

impl gix_sec::trust::DefaultForLevel for Permissions {
    fn default_for_level(level: Trust) -> Self {
        match level {
            Trust::Full => Permissions::all(),
            Trust::Reduced => Permissions::secure(),
        }
    }
}

impl Default for Permissions {
    fn default() -> Self {
        Permissions::secure()
    }
}

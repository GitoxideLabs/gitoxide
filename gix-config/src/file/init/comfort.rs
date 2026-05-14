use std::borrow::Cow;

use crate::{
    File, Source,
    file::{Metadata, init},
    path, source,
};

/// Easy-instantiation of typical non-repository git configuration files with all configuration defaulting to typical values.
///
/// ### Limitations
///
/// Note that `includeIf` conditions in global files will cause failure as the required information
/// to resolve them isn't present without a repository.
///
/// Also note that relevant information to interpolate paths will be obtained from the environment or other
/// source on unix.
impl File<'static> {
    /// Open all global configuration files which involves the following sources:
    ///
    /// * [git-installation](source::Kind::GitInstallation)
    /// * [system](source::Kind::System)
    /// * [globals](source::Kind::Global)
    ///
    /// which excludes repository local configuration, as well as override-configuration from environment variables.
    ///
    /// Note that the file might [be empty][File::is_void()] in case no configuration file was found.
    pub fn from_globals() -> Result<File<'static>, init::from_paths::Error> {
        let metas = [
            source::Kind::GitInstallation,
            source::Kind::System,
            source::Kind::Global,
        ]
        .iter()
        .flat_map(|kind| kind.sources())
        .filter_map(|source| {
            let path = source
                .storage_location(&mut gix_path::env::var)
                .and_then(|p| p.is_file().then_some(p))
                .map(Cow::into_owned);

            Metadata {
                path,
                source: *source,
                level: 0,
                trust: gix_sec::Trust::Full,
            }
            .into()
        });

        let home = gix_path::env::home_dir();
        let options = init::Options {
            includes: init::includes::Options::follow_without_conditional(home.as_deref()),
            ..Default::default()
        };
        File::from_paths_metadata(metas, options).map(Option::unwrap_or_default)
    }

    /// Generates a config from `GIT_CONFIG_*` environment variables and return a possibly empty `File`.
    /// A typical use of this is to [`append`][File::append()] this configuration to another one with lower
    /// precedence to obtain overrides.
    ///
    /// See [`git-config`'s documentation] for more information on the environment variables in question.
    ///
    /// [`git-config`'s documentation]: https://git-scm.com/docs/git-config#Documentation/git-config.txt-GITCONFIGCOUNT
    pub fn from_environment_overrides() -> Result<File<'static>, init::from_env::Error> {
        let home = gix_path::env::home_dir();
        let options = init::Options {
            includes: init::includes::Options::follow_without_conditional(home.as_deref()),
            ..Default::default()
        };

        File::from_env(options).map(Option::unwrap_or_default)
    }
}

/// An easy way to provide complete configuration for a repository.
impl File<'static> {
    /// This configuration type includes the following sources, in order of precedence:
    ///
    /// - globals
    /// - repository-local by loading `dir`/config
    /// - worktree by loading `dir`/config.worktree
    /// - environment
    ///
    /// Note that `dir` is the `.git` dir to load the configuration from, not the configuration file.
    ///
    /// Includes will be resolved within limits as some information like the git installation directory is missing to interpolate
    /// paths with as well as git repository information like the branch name.
    pub fn from_git_dir(dir: std::path::PathBuf) -> Result<File<'static>, from_git_dir::Error> {
        let (mut local, git_dir) = {
            let source = Source::Local;
            let mut path = dir;
            path.push(
                source
                    .storage_location(&mut gix_path::env::var)
                    .expect("location available for local"),
            );
            let local = Self::from_path_no_includes(path.clone(), source)?;
            path.pop();
            (local, path)
        };

        let worktree = match local.boolean("extensions.worktreeConfig") {
            Some(Ok(worktree_config)) => worktree_config.then(|| {
                let source = Source::Worktree;
                let path = git_dir.join(
                    source
                        .storage_location(&mut gix_path::env::var)
                        .expect("location available for worktree"),
                );
                Self::from_path_no_includes(path, source)
            }),
            _ => None,
        }
        .transpose()?;

        let home = gix_path::env::home_dir();
        let options = init::Options {
            includes: init::includes::Options::follow(
                path::interpolate::Context {
                    home_dir: home.as_deref(),
                    ..Default::default()
                },
                init::includes::conditional::Context {
                    git_dir: Some(git_dir.as_ref()),
                    branch_name: None,
                },
            ),
            ..Default::default()
        };

        let mut globals = Self::from_globals()?;
        globals.resolve_includes(options)?;
        local.resolve_includes(options)?;

        globals.append(local);
        if let Some(mut worktree) = worktree {
            worktree.resolve_includes(options)?;
            globals.append(worktree);
        }
        globals.append(Self::from_environment_overrides()?);

        Ok(globals)
    }
}

///
pub mod from_git_dir {
    use crate::file::init;

    /// The error returned by [`File::from_git_dir()`][crate::File::from_git_dir()].
    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error(transparent)]
        FromPaths(#[from] init::from_paths::Error),
        #[error(transparent)]
        FromEnv(#[from] init::from_env::Error),
        #[error(transparent)]
        Init(#[from] init::Error),
        #[error(transparent)]
        Includes(#[from] init::includes::Error),
    }
}

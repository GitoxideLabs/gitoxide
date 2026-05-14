use gix_diff::tree::recorder::Location;
pub use gix_diff::*;

///
pub mod options {
    ///
    pub mod init {
        /// The error returned when instantiating [diff options](crate::diff::Options).
        #[derive(Debug, thiserror::Error)]
        #[allow(missing_docs)]
        pub enum Error {
            #[cfg(feature = "blob-diff")]
            #[error(transparent)]
            RewritesConfiguration(#[from] crate::diff::new_rewrites::Error),
        }
    }
}

/// General diff-related options for configuring rename-tracking and blob diffs.
#[derive(Debug, Copy, Clone)]
pub struct Options {
    location: Option<Location>,
    #[cfg(feature = "blob-diff")]
    rewrites: Option<gix_diff::Rewrites>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            location: Some(Location::Path),
            #[cfg(feature = "blob-diff")]
            rewrites: None,
        }
    }
}

#[cfg(feature = "blob-diff")]
impl From<Options> for gix_diff::tree_with_rewrites::Options {
    fn from(opts: Options) -> Self {
        gix_diff::tree_with_rewrites::Options {
            location: opts.location,
            #[cfg(feature = "blob-diff")]
            rewrites: opts.rewrites,
        }
    }
}

/// Lifecycle
impl Options {
    #[cfg(feature = "blob-diff")]
    pub(crate) fn from_configuration(config: &crate::config::Cache) -> Result<Self, options::init::Error> {
        Ok(Options {
            location: Some(Location::Path),
            rewrites: {
                let (rewrites, is_configured) = config.diff_renames()?;
                if is_configured {
                    rewrites
                } else {
                    Some(Default::default())
                }
            },
        })
    }
}

/// Setters
impl Options {
    /// Do not keep track of filepaths at all, which will leave all `location` fields empty.
    pub fn no_locations(&mut self) -> &mut Self {
        self.location = None;
        self
    }

    /// Keep track of file-names, which makes `location` fields usable with the filename of the changed item.
    pub fn track_filename(&mut self) -> &mut Self {
        self.location = Some(Location::FileName);
        self
    }

    /// Keep track of the entire path of a change, relative to the repository. (default).
    ///
    /// This makes the `location` field fully usable.
    pub fn track_path(&mut self) -> &mut Self {
        self.location = Some(Location::Path);
        self
    }

    /// Provide `None` to disable rewrite tracking entirely, or pass `Some(<configuration>)` to control to
    /// what extent rename and copy tracking is performed.
    ///
    /// Note that by default, the git configuration determines rewrite tracking and git defaults are used
    /// if nothing is configured, which turns rename tracking with 50% similarity on, while not tracking copies at all.
    #[cfg(feature = "blob-diff")]
    pub fn track_rewrites(&mut self, renames: Option<gix_diff::Rewrites>) -> &mut Self {
        self.rewrites = renames;
        self
    }
}

/// Builder
impl Options {
    /// Provide `None` to disable rewrite tracking entirely, or pass `Some(<configuration>)` to control to
    /// what extent rename and copy tracking is performed.
    ///
    /// Note that by default, the git configuration determines rewrite tracking and git defaults are used
    /// if nothing is configured, which turns rename tracking with 50% similarity on, while not tracking copies at all.
    #[cfg(feature = "blob-diff")]
    pub fn with_rewrites(mut self, renames: Option<gix_diff::Rewrites>) -> Self {
        self.rewrites = renames;
        self
    }
}

///
pub mod rename {
    /// Determine how to do rename tracking.
    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub enum Tracking {
        /// Do not track renames at all, the fastest option.
        Disabled,
        /// Track renames.
        Renames,
        /// Track renames and copies.
        ///
        /// This is the most expensive option.
        RenamesAndCopies,
    }
}

///
#[cfg(feature = "blob-diff")]
pub(crate) mod utils {
    use gix_diff::{Rewrites, rewrites::Copies};

    use crate::{
        Repository,
        config::{cache::util::ApplyLeniency, tree::Diff},
        diff::rename::Tracking,
    };

    ///
    pub mod new_rewrites {
        /// The error returned by [`new_rewrites()`](super::new_rewrites()).
        #[derive(Debug, thiserror::Error)]
        #[allow(missing_docs)]
        pub enum Error {
            #[error(transparent)]
            ConfigDiffRenames(#[from] crate::config::key::GenericError),
            #[error(transparent)]
            ConfigDiffRenameLimit(#[from] crate::config::unsigned_integer::Error),
        }
    }

    ///
    pub mod resource_cache {
        /// The error returned by [`resource_cache()`](super::resource_cache()).
        #[derive(Debug, thiserror::Error)]
        #[allow(missing_docs)]
        pub enum Error {
            #[error(transparent)]
            DiffAlgorithm(#[from] crate::config::diff::algorithm::Error),
            #[error(transparent)]
            WorktreeFilterOptions(#[from] crate::filter::pipeline::options::Error),
            #[error(transparent)]
            DiffDrivers(#[from] crate::config::diff::drivers::Error),
            #[error(transparent)]
            DiffPipelineOptions(#[from] crate::config::diff::pipeline_options::Error),
            #[error(transparent)]
            CommandContext(#[from] crate::config::command_context::Error),
        }
    }

    /// Create an instance by reading all relevant information from the `config`uration, while being `lenient` or not.
    /// Returns `Ok((None, false))` if nothing is configured, or `Ok((None, true))` if it's configured and disabled.
    ///
    /// Note that missing values will be defaulted similar to what git does.
    #[allow(clippy::result_large_err)]
    pub fn new_rewrites(
        config: &gix_config::File<'static>,
        lenient: bool,
    ) -> Result<(Option<Rewrites>, bool), new_rewrites::Error> {
        new_rewrites_inner(config, lenient, &Diff::RENAMES, &Diff::RENAME_LIMIT)
    }

    pub(crate) fn new_rewrites_inner(
        config: &gix_config::File<'static>,
        lenient: bool,
        renames: &'static crate::config::tree::diff::Renames,
        rename_limit: &'static crate::config::tree::keys::UnsignedInteger,
    ) -> Result<(Option<Rewrites>, bool), new_rewrites::Error> {
        let copies = match config
            .boolean(renames)
            .map(|value| renames.try_into_renames(value))
            .transpose()
            .with_leniency(lenient)?
        {
            Some(renames) => match renames {
                Tracking::Disabled => return Ok((None, true)),
                Tracking::Renames => None,
                Tracking::RenamesAndCopies => Some(Copies::default()),
            },
            None => return Ok((None, false)),
        };

        let default = Rewrites::default();
        Ok((
            Rewrites {
                copies,
                limit: config
                    .integer(rename_limit)
                    .map(|value| rename_limit.try_into_usize(value))
                    .transpose()
                    .with_leniency(lenient)?
                    .unwrap_or(default.limit),
                ..default
            }
            .into(),
            true,
        ))
    }

    /// Return a low-level utility to efficiently prepare a blob-level diff operation between two resources,
    /// and cache these diffable versions so that matrix-like MxN diffs are efficient.
    ///
    /// `repo` is used to obtain the needed configuration values.
    /// `mode` determines how the diffable files will look like, and also how fast, in average, these conversions are.
    /// `attr_stack` is for accessing `.gitattributes` for knowing how to apply filters. Know that it's typically adjusted based on the
    /// `roots` - if there are no worktree roots, `.gitattributes` are also not usually read from worktrees.
    /// `roots` provide information about where to get diffable data from, so source and destination can either be sourced from
    /// a worktree, or from the object database, or both.
    pub fn resource_cache(
        repo: &Repository,
        mode: gix_diff::blob::pipeline::Mode,
        attr_stack: gix_worktree::Stack,
        roots: gix_diff::blob::pipeline::WorktreeRoots,
    ) -> Result<gix_diff::blob::Platform, resource_cache::Error> {
        let diff_algo = repo.config.diff_algorithm()?;
        let diff_cache = gix_diff::blob::Platform::new(
            gix_diff::blob::platform::Options {
                algorithm: Some(diff_algo),
                skip_internal_diff_if_external_is_configured: false,
            },
            gix_diff::blob::Pipeline::new(
                roots,
                gix_filter::Pipeline::new(repo.command_context()?, crate::filter::Pipeline::options(repo)?),
                repo.config.diff_drivers()?,
                repo.config.diff_pipeline_options()?,
            ),
            mode,
            attr_stack,
        );
        Ok(diff_cache)
    }
}
#[cfg(feature = "blob-diff")]
pub use utils::{new_rewrites, resource_cache};

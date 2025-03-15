//! lower-level access to filters which are applied to create working tree checkouts or to 'clean' working tree contents for storage in git.
use std::borrow::Cow;

pub use gix_filter as plumbing;
use gix_object::Find;

use crate::prelude::ObjectIdExt;
use crate::{
    bstr::BStr,
    config::{
        cache::util::{ApplyLeniency, ApplyLeniencyDefaultValue},
        tree::Core,
    },
    Repository,
};

///
pub mod pipeline {
    ///
    pub mod options {
        use crate::{bstr::BString, config};

        /// The error returned by [Pipeline::options()](crate::filter::Pipeline::options()).
        #[derive(Debug, thiserror::Error)]
        #[allow(missing_docs)]
        pub enum Error {
            #[error(transparent)]
            CheckRoundTripEncodings(#[from] config::encoding::Error),
            #[error(transparent)]
            SafeCrlf(#[from] config::key::GenericErrorWithValue),
            #[error("Could not interpret 'filter.{name}.required' configuration")]
            Driver {
                name: BString,
                source: gix_config::value::Error,
            },
            #[error(transparent)]
            CommandContext(#[from] config::command_context::Error),
        }
    }

    ///
    pub mod convert_to_git {
        /// The error returned by [Pipeline::convert_to_git()](crate::filter::Pipeline::convert_to_git()).
        #[derive(Debug, thiserror::Error)]
        #[allow(missing_docs)]
        pub enum Error {
            #[error("Failed to prime attributes to the path at which the data resides")]
            WorktreeCacheAtPath(#[from] std::io::Error),
            #[error(transparent)]
            Convert(#[from] gix_filter::pipeline::convert::to_git::Error),
        }
    }

    ///
    pub mod convert_to_worktree {
        /// The error returned by [Pipeline::convert_to_worktree()](crate::filter::Pipeline::convert_to_worktree()).
        #[derive(Debug, thiserror::Error)]
        #[allow(missing_docs)]
        pub enum Error {
            #[error("Failed to prime attributes to the path at which the data resides")]
            WorktreeCacheAtPath(#[from] std::io::Error),
            #[error(transparent)]
            Convert(#[from] gix_filter::pipeline::convert::to_worktree::Error),
        }
    }

    ///
    pub mod worktree_file_to_object {
        use std::path::PathBuf;

        /// The error returned by [Pipeline::worktree_file_to_object()](crate::filter::Pipeline::worktree_file_to_object()).
        #[derive(Debug, thiserror::Error)]
        #[allow(missing_docs)]
        pub enum Error {
            #[error("Cannot add worktree files in bare repositories")]
            MissingWorktree,
            #[error("Failed to perform IO for object creation for '{}'", path.display())]
            IO { source: std::io::Error, path: PathBuf },
            #[error(transparent)]
            WriteBlob(#[from] crate::object::write::Error),
            #[error(transparent)]
            ConvertToGit(#[from] crate::filter::pipeline::convert_to_git::Error),
        }
    }
}

/// A git pipeline for transforming data *to-git* and *to-worktree*, based
/// [on git configuration and attributes](https://git-scm.com/docs/gitattributes).
#[derive(Clone)]
pub struct Pipeline<'repo> {
    inner: gix_filter::Pipeline,
    cache: gix_worktree::Stack,
    /// The repository this pipeline is associated with.
    pub repo: &'repo Repository,
}

/// Lifecycle
impl<'repo> Pipeline<'repo> {
    /// Extract options from `repo` that are needed to properly drive a standard git filter pipeline.
    pub fn options(repo: &'repo Repository) -> Result<gix_filter::pipeline::Options, pipeline::options::Error> {
        let config = &repo.config.resolved;
        let encodings =
            Core::CHECK_ROUND_TRIP_ENCODING.try_into_encodings(config.string("core.checkRoundtripEncoding"))?;
        let safe_crlf = config
            .string("core.safecrlf")
            .map(|value| Core::SAFE_CRLF.try_into_safecrlf(value))
            .transpose()
            .map(Option::unwrap_or_default)
            .with_lenient_default_value(
                repo.config.lenient_config,
                // in lenient mode, we prefer the safe option, instead of just (trying) to output warnings.
                gix_filter::pipeline::CrlfRoundTripCheck::Fail,
            )?;
        let auto_crlf = config
            .string("core.autocrlf")
            .map(|value| Core::AUTO_CRLF.try_into_autocrlf(value))
            .transpose()
            .with_leniency(repo.config.lenient_config)?
            .unwrap_or_default();
        let eol = config
            .string("core.eol")
            .map(|value| Core::EOL.try_into_eol(value))
            .transpose()?;
        let drivers = extract_drivers(repo)?;
        Ok(gix_filter::pipeline::Options {
            drivers,
            eol_config: gix_filter::eol::Configuration { auto_crlf, eol },
            encodings_with_roundtrip_check: encodings,
            crlf_roundtrip_check: safe_crlf,
            object_hash: repo.object_hash(),
        })
    }

    /// Create a new instance by extracting all necessary information and configuration from a `repo` along with `cache` for accessing
    /// attributes. The `index` is used for some filters which may access it under very specific circumstances.
    pub fn new(repo: &'repo Repository, cache: gix_worktree::Stack) -> Result<Self, pipeline::options::Error> {
        let pipeline = gix_filter::Pipeline::new(repo.command_context()?, Self::options(repo)?);
        Ok(Pipeline {
            inner: pipeline,
            cache,
            repo,
        })
    }

    /// Detach the repository and obtain the individual functional parts.
    pub fn into_parts(self) -> (gix_filter::Pipeline, gix_worktree::Stack) {
        (self.inner, self.cache)
    }
}

/// Conversions
impl Pipeline<'_> {
    /// Convert a `src` stream (to be found at `rela_path`, a repo-relative path) to a representation suitable for storage in `git`
    /// by using all attributes at `rela_path` and configuration of the repository to know exactly which filters apply.
    /// `index` is used in particularly rare cases where the CRLF filter in auto-mode tries to determine whether to apply itself,
    /// and it should match the state used when [instantiating this instance](Self::new()).
    /// Note that the return-type implements [`std::io::Read`].
    pub fn convert_to_git<R>(
        &mut self,
        src: R,
        rela_path: &std::path::Path,
        index: &gix_index::State,
    ) -> Result<gix_filter::pipeline::convert::ToGitOutcome<'_, R>, pipeline::convert_to_git::Error>
    where
        R: std::io::Read,
    {
        let entry = self.cache.at_path(rela_path, None, &self.repo.objects)?;
        Ok(self.inner.convert_to_git(
            src,
            rela_path,
            &mut |_, attrs| {
                entry.matching_attributes(attrs);
            },
            &mut |buf| -> Result<_, gix_object::find::Error> {
                let entry = match index.entry_by_path(gix_path::into_bstr(rela_path).as_ref()) {
                    None => return Ok(None),
                    Some(entry) => entry,
                };
                let obj = self.repo.objects.try_find(&entry.id, buf)?;
                Ok(obj.filter(|obj| obj.kind == gix_object::Kind::Blob).map(|_| ()))
            },
        )?)
    }

    /// Convert a `src` buffer located at `rela_path` (in the index) from what's in `git` to the worktree representation.
    /// This method will obtain all attributes and configuration necessary to know exactly which filters to apply.
    /// Note that the return-type implements [`std::io::Read`].
    ///
    /// Use `can_delay` to tell driver processes that they may delay the return of data. Doing this will require the caller to specifically
    /// handle delayed files by keeping state and using [`Self::into_parts()`] to get access to the driver state to follow the delayed-files
    /// protocol. For simplicity, most will want to disallow delayed processing.
    pub fn convert_to_worktree<'input>(
        &mut self,
        src: &'input [u8],
        rela_path: &BStr,
        can_delay: gix_filter::driver::apply::Delay,
    ) -> Result<gix_filter::pipeline::convert::ToWorktreeOutcome<'input, '_>, pipeline::convert_to_worktree::Error>
    {
        let entry = self.cache.at_entry(rela_path, None, &self.repo.objects)?;
        Ok(self.inner.convert_to_worktree(
            src,
            rela_path,
            &mut |_, attrs| {
                entry.matching_attributes(attrs);
            },
            can_delay,
        )?)
    }

    /// Add the worktree file at `rela_path` to the object database and return its `(id, entry, symlink_metadata)` for use in a tree or in the index, for instance.
    /// If `rela_path` is a directory *and* is a repository with its `HEAD` pointing to a commit, it will also be provided with the appropriate kind.
    /// Note that this can easily lead to embedded repositories as no submodule-crosscheck is performed. Otherwise, unreadable repositories or directories
    /// are ignored with `None` as return value.
    ///
    /// `index` is used in particularly rare cases where the CRLF filter in auto-mode tries to determine whether to apply itself,
    /// and it should match the state used when [instantiating this instance](Self::new()).
    ///
    /// Return `Ok(None)` the file didn't exist in the worktree, or if it was of an untrackable type.
    pub fn worktree_file_to_object(
        &mut self,
        rela_path: &BStr,
        index: &gix_index::State,
    ) -> Result<
        Option<(gix_hash::ObjectId, gix_object::tree::EntryKind, std::fs::Metadata)>,
        pipeline::worktree_file_to_object::Error,
    > {
        use pipeline::worktree_file_to_object::Error;

        let rela_path_as_path = gix_path::from_bstr(rela_path);
        let repo = self.repo;
        let worktree_dir = repo.workdir().ok_or(Error::MissingWorktree)?;
        let path = worktree_dir.join(&rela_path_as_path);
        let md = match std::fs::symlink_metadata(&path) {
            Ok(md) => md,
            Err(err) => {
                if gix_fs::io_err::is_not_found(err.kind(), err.raw_os_error()) {
                    return Ok(None);
                } else {
                    return Err(Error::IO { source: err, path });
                }
            }
        };
        let (id, kind) = if md.is_symlink() {
            let target = std::fs::read_link(&path).map_err(|source| Error::IO { source, path })?;
            let id = repo.write_blob(gix_path::into_bstr(target).as_ref())?;
            (id, gix_object::tree::EntryKind::Link)
        } else if md.is_file() {
            use gix_filter::pipeline::convert::ToGitOutcome;

            let file = std::fs::File::open(&path).map_err(|source| Error::IO { source, path })?;
            let file_for_git = self.convert_to_git(file, rela_path_as_path.as_ref(), index)?;
            let id = match file_for_git {
                ToGitOutcome::Unchanged(mut file) => repo.write_blob_stream(&mut file)?,
                ToGitOutcome::Buffer(buf) => repo.write_blob(buf)?,
                ToGitOutcome::Process(mut read) => repo.write_blob_stream(&mut read)?,
            };

            let kind = if gix_fs::is_executable(&md) {
                gix_object::tree::EntryKind::BlobExecutable
            } else {
                gix_object::tree::EntryKind::Blob
            };
            (id, kind)
        } else if md.is_dir() {
            let Some(submodule_repo) = crate::open_opts(&path, repo.open_options().clone()).ok() else {
                return Ok(None);
            };
            let Some(id) = submodule_repo.head_id().ok() else {
                return Ok(None);
            };
            (id.detach().attach(repo), gix_object::tree::EntryKind::Commit)
        } else {
            // This is probably a type-change to something we can't track.
            return Ok(None);
        };

        Ok(Some((id.detach(), kind, md)))
    }

    /// Retrieve the static context that is made available to the process filters.
    ///
    /// The context set here is relevant for the [`convert_to_git()`][Self::convert_to_git()] and
    /// [`convert_to_worktree()`][Self::convert_to_worktree()] methods.
    pub fn driver_context_mut(&mut self) -> &mut gix_filter::pipeline::Context {
        self.inner.driver_context_mut()
    }
}

/// Obtain a list of all configured driver, but ignore those in sections that we don't trust enough.
fn extract_drivers(repo: &Repository) -> Result<Vec<gix_filter::Driver>, pipeline::options::Error> {
    repo.config
        .resolved
        .sections_by_name("filter")
        .into_iter()
        .flatten()
        .filter(|s| repo.filter_config_section()(s.meta()))
        .filter_map(|s| {
            s.header().subsection_name().map(|name| {
                Ok(gix_filter::Driver {
                    name: name.to_owned(),
                    clean: s.value("clean").map(Cow::into_owned),
                    smudge: s.value("smudge").map(Cow::into_owned),
                    process: s.value("process").map(Cow::into_owned),
                    required: s
                        .value("required")
                        .map(|value| gix_config::Boolean::try_from(value.as_ref()))
                        .transpose()
                        .map_err(|err| pipeline::options::Error::Driver {
                            name: name.to_owned(),
                            source: err,
                        })?
                        .unwrap_or_default()
                        .into(),
                })
            })
        })
        .collect::<Result<Vec<_>, pipeline::options::Error>>()
}

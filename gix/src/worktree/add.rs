use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use gix_error::{ClassificationMarker, ErrorExt, OptionExt, ResultExt, bail, ensure, message, validation};
use gix_utils::progress::{NestedProgress, Progress};

use crate::{Result, repository::FormatVersion};
use gix_ref::transaction::{LogChange, PreviousValue, RefEdit, RefLog};

use crate::config::{
    cache::util::ApplyLeniency,
    tree::{Core, Extensions, Worktree},
};

/// The kind of `HEAD` to install in a newly added worktree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Head {
    /// Check out an existing local branch in the default reference namespace.
    Attached(gix_ref::FullName),
    /// Check out a commit with a detached `HEAD`.
    Detached(gix_hash::ObjectId),
}

/// A rejection reported by [`Repository::add_worktree()`][crate::Repository::add_worktree()].
/// These errors are retained in the returned error chain for callers to inspect.
/// Invalid branch input is classified as [`gix_error::Class::Validation`], occupied branches and destinations
/// as [`gix_error::Class::Conflict`], and an observed cancellation as [`gix_error::Class::Cancelled`].
#[derive(Debug)]
#[expect(missing_docs)]
pub enum Error {
    NotLocalBranch {
        name: gix_ref::FullName,
    },
    CheckedOut {
        name: gix_ref::FullName,
        worktree_dirs: Vec<std::path::PathBuf>,
    },
    DestinationRegistered {
        destination: std::path::PathBuf,
    },
    Interrupted,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotLocalBranch { name } => write!(f, "{name:?} is not a local branch"),
            Error::CheckedOut { name, worktree_dirs } => {
                write!(f, "The local branch {name:?} is checked out in {worktree_dirs:?}")
            }
            Error::DestinationRegistered { destination } => {
                write!(
                    f,
                    "The worktree destination {} is already registered",
                    destination.display()
                )
            }
            Error::Interrupted => f.write_str("Adding the worktree was interrupted"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Error::NotLocalBranch { .. } => const { &ClassificationMarker::VALIDATION },
            Error::CheckedOut { .. } | Error::DestinationRegistered { .. } => const { &ClassificationMarker::CONFLICT },
            Error::Interrupted => const { &ClassificationMarker::CANCELLED },
        })
    }
}

impl crate::Repository {
    /// Add and check out a linked worktree at `destination` with the given `head`.
    ///
    /// Attached heads must name an existing local branch which isn't checked out or reserved by an ongoing
    /// bisect or rebase in any worktree.
    /// Reference namespaces are ignored for branch lookup, occupancy checks, and the returned repository, like
    /// `git worktree add`. The source repository's namespace is preserved.
    /// The destination must either not exist or be an empty, unregistered directory. Any files created by this
    /// method are removed if adding the worktree fails or is interrupted.
    ///
    /// `progress` reports the number of files checked out and bytes written through the named `checkout` and
    /// `writing` child progress items, respectively. Set `should_interrupt` to `true` to request cancellation.
    /// It is checked before worktree preparation, during checkout, and again before writing the index and
    /// finalizing the worktree. An observed interruption returns an error and triggers the cleanup described above.
    ///
    /// With `extensions.worktreeConfig`, the source's `config.worktree` is copied before checkout, excluding
    /// `core.worktree` and `core.bare` key set to `true`.
    /// The new `HEAD` reflog records its initial commit when `core.logAllRefUpdates` permits it.
    ///
    /// `worktree.useRelativePaths` selects relative links instead of the default absolute links. When enabled,
    /// the shared config is upgraded to repository format version 1 with `extensions.relativeWorktrees=true`.
    /// This compatibility marker remains set even if checkout fails, and requires Git 2.48 or newer.
    /// The parent repository's configuration snapshot is unchanged; call [`reload()`][Self::reload()] to refresh it.
    /// Source repository paths are anchored to its captured [`current_dir()`][Self::current_dir()], while a relative
    /// `destination` uses the process's current directory. The returned repository has absolute paths.
    pub fn add_worktree<P>(
        &self,
        destination: impl AsRef<Path>,
        head: Head,
        progress: P,
        should_interrupt: &AtomicBool,
    ) -> Result<(crate::Repository, gix_worktree_state::checkout::Outcome)>
    where
        P: NestedProgress,
        P::SubProgress: NestedProgress + 'static,
    {
        self.prepare_add_worktree(destination, head, should_interrupt)?
            .checkout(progress, should_interrupt)
    }

    /// Register a linked worktree at `destination` with the given `head`, without checking out files or creating its index.
    ///
    /// The same validation, configuration and HEAD initialization as [`Self::add_worktree()`] apply.
    /// Dropping the returned preparation rolls back the registration. Call [`PrepareCheckout::persist()`]
    /// to keep it without checkout, or [`PrepareCheckout::checkout()`] to check out and finish registration.
    /// Set `should_interrupt` to `true` to request cancellation, checked before preparation and before returning the handle.
    pub fn prepare_add_worktree(
        &self,
        destination: impl AsRef<Path>,
        head: Head,
        should_interrupt: &AtomicBool,
    ) -> Result<PrepareCheckout> {
        let repo = self.with_absolute_paths()?;
        let destination = destination.as_ref();
        let relative_paths = Worktree::USE_RELATIVE_PATHS
            .enrich_error(repo.config.resolved.boolean(Worktree::USE_RELATIVE_PATHS))
            .with_leniency(repo.config.lenient_config)?
            .unwrap_or_default();
        let (head_target, commit_id, root_tree_id) = match head {
            Head::Attached(name) => {
                if name.category() != Some(gix_ref::Category::LocalBranch) {
                    bail!(Error::NotLocalBranch { name });
                }
                let checked_out = repo.checked_out_branches_without_namespace()?;
                if let Some(worktree_dirs) = checked_out.get(&name) {
                    bail!(Error::CheckedOut {
                        name,
                        worktree_dirs: worktree_dirs.clone(),
                    });
                }
                let mut source = repo.as_ref().clone();
                source.clear_namespace();
                let mut reference = source
                    .find_reference(name.as_ref())
                    .or_raise(|| message("The local branch could not be found"))?;
                let commit = reference.peel_to_commit()?;
                let root_tree_id = commit.tree_id()?.detach();
                (gix_ref::Target::Symbolic(name), commit.id, root_tree_id)
            }
            Head::Detached(commit_id) => {
                let root_tree_id = repo
                    .find_commit(commit_id)
                    .or_raise(|| message("The detached target is not an existing commit"))?
                    .tree_id()?
                    .detach();
                (gix_ref::Target::Object(commit_id), commit_id, root_tree_id)
            }
        };
        ensure!(!should_interrupt.load(Ordering::Relaxed), Error::Interrupted);

        let main_repo = repo
            .main_repo()
            .or_raise(|| message("Could not open a worktree repository"))?;
        let mut registered_destinations = main_repo.workdir().map(Path::to_owned).into_iter().collect::<Vec<_>>();
        // Read registered paths directly instead of using `worktrees_including_main()`, which can apply
        // `core.worktree` overrides and suppresses errors reading the registration's `gitdir` file.
        for worktree in repo.worktrees()? {
            registered_destinations.push(
                worktree
                    .base()
                    .or_raise(|| message("Failed to read or iterate worktree directories"))?,
            );
        }
        let prepared = gix_worktree::add::prepare(
            repo.common_dir(),
            destination,
            gix_worktree::add::Options { relative_paths },
        )
        .or_raise(|| message("Could not prepare the linked worktree"))?;
        // `prepare()` already applied `gix_path::realpath()`, but that preserves component spelling.
        // Canonicalize the existing destination to also match Windows casing and short-name aliases.
        // The `realpath()` comparison below still handles registered paths whose directories are missing.
        let canonical_destination =
            std::fs::canonicalize(prepared.work_dir()).or_raise(|| message("Could not prepare the linked worktree"))?;
        for registered_destination in registered_destinations {
            let registered_destination = gix_path::realpath(registered_destination)
                .or_raise(|| message("Failed to resolve a registered worktree directory"))?;
            let same_destination = if registered_destination == prepared.work_dir() {
                true
            } else {
                match std::fs::canonicalize(&registered_destination) {
                    Ok(path) => path == canonical_destination,
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => false,
                    Err(err) => {
                        bail!(err.and_raise(message("Failed to resolve a registered worktree directory")));
                    }
                }
            };
            if same_destination {
                bail!(Error::DestinationRegistered {
                    destination: destination.to_owned(),
                });
            }
        }
        if relative_paths {
            let mut config = repo
                .config_file_mut(repo.common_dir().join("config"))
                .or_raise(|| message("Could not enable relative worktrees in the shared configuration"))?;
            let version = Core::REPOSITORY_FORMAT_VERSION
                .try_into_repository_format_version(config.integer(Core::REPOSITORY_FORMAT_VERSION))
                .or_raise(|| message("Could not upgrade the repository format for relative worktrees"))?
                .unwrap_or_default();
            if version == FormatVersion::V0 {
                version.validate_upgrade_to_v1(&config)?;
                config
                    .set_raw_value(Core::REPOSITORY_FORMAT_VERSION, "1")
                    .or_raise(|| message("Could not set the repository format for relative worktrees"))?;
            }
            let enabled = Extensions::RELATIVE_WORKTREES
                .enrich_error(config.boolean(Extensions::RELATIVE_WORKTREES))?
                .unwrap_or_default();
            if version == FormatVersion::V0 || !enabled {
                config
                    .set_raw_value(Extensions::RELATIVE_WORKTREES, "true")
                    .or_raise(|| message("Could not set the repository format for relative worktrees"))?;
                config
                    .commit()
                    .or_raise(|| message("Could not enable relative worktrees in the shared configuration"))?;
            }
        }
        // Opening the repository requires HEAD. Preserve symbolic targets so conditional configuration
        // can see the branch; detached HEADs start at null so their first update records initialization.
        let mut head_contents = Vec::new();
        let reflog_mode = match &head_target {
            gix_ref::Target::Object(_) => {
                repo.object_hash()
                    .null()
                    .write_hex_to(&mut head_contents)
                    .or_raise(|| message("Could not write the linked worktree HEAD"))?;
                RefLog::AndReference
            }
            gix_ref::Target::Symbolic(name) => {
                head_contents.extend_from_slice(b"ref: ");
                head_contents.extend_from_slice(name.as_bstr());
                RefLog::Only
            }
        };
        head_contents.push(b'\n');
        std::fs::write(prepared.git_dir().join("HEAD"), head_contents)
            .or_raise(|| message("Could not write the linked worktree HEAD"))?;

        if Extensions::WORKTREE_CONFIG
            .enrich_error(repo.config.resolved.boolean(Extensions::WORKTREE_CONFIG))
            .with_leniency(repo.config.lenient_config)?
            .unwrap_or_default()
        {
            copy_worktree_config(
                &repo.git_dir().join("config.worktree"),
                &prepared.git_dir().join("config.worktree"),
            )?;
        }

        let options = repo
            .options
            .clone()
            .without_repository_environment_overrides()
            .open_path_as_is(true);
        let mut repo = crate::ThreadSafeRepository::open_opts(prepared.git_dir(), options)
            .or_raise(|| message("Could not open a worktree repository"))?
            .to_thread_local();
        repo.clear_namespace();
        // Like clone, initialize a symbolic HEAD's log without dereferencing or updating its branch.
        repo.edit_reference(RefEdit::update_with_log(
            "HEAD".try_into().expect("valid reference name"),
            commit_id,
            PreviousValue::Any,
            LogChange {
                mode: reflog_mode,
                ..Default::default()
            },
        ))
        .or_raise(|| message("Could not initialize the linked worktree HEAD and its reflog"))?;
        Ok(PrepareCheckout {
            repo,
            prepared,
            root_tree_id,
        })
    }
}

/// A registered linked worktree which owns rollback until persisted or successfully checked out.
///
/// Registration installs `.git`, HEAD and configuration, but no index or tracked files.
/// Drop performs best-effort rollback of checkout and private administration. Shared configuration
/// upgrades and attached branches are retained. Use [`Self::rollback()`] to observe cleanup errors.
#[must_use]
#[derive(Debug)]
pub struct PrepareCheckout {
    repo: crate::Repository,
    prepared: gix_worktree::add::PreparedWorktree,
    root_tree_id: gix_hash::ObjectId,
}

/// Access
impl PrepareCheckout {
    /// Access the repository with absolute paths before checking out or persisting its registration.
    pub fn repo(&self) -> &crate::Repository {
        &self.repo
    }
}

/// Lifecycle
impl PrepareCheckout {
    /// Complete registration without checkout and return the repository.
    ///
    /// The returned repository remains usable for object/reference access even if its checkout
    /// directory is later missing. Use [`crate::Worktree::checkout()`] for initial checkout.
    pub fn persist(self) -> Result<crate::Repository> {
        self.prepared
            .persist()
            .or_raise(|| message("Could not finish adding the linked worktree"))?;
        Ok(self.repo)
    }

    /// Explicitly undo registration, reporting checkout or administrative cleanup failures.
    pub fn rollback(self) -> Result<()> {
        self.prepared
            .rollback()
            .or_raise(|| message("Could not roll back the linked worktree"))
    }

    /// Consume this preparation, check out its selected tree, and complete registration.
    ///
    /// `progress` reports files checked out and bytes written through the `checkout` and `writing`
    /// child progress items, respectively. Set `should_interrupt` to `true` to request cancellation;
    /// it is checked before and during checkout.
    ///
    /// Failure or cancellation rolls back its checkout and private administration. A destination
    /// which existed before preparation is left empty; parent directories are retained.
    pub fn checkout<P>(
        self,
        progress: P,
        should_interrupt: &AtomicBool,
    ) -> Result<(crate::Repository, gix_worktree_state::checkout::Outcome)>
    where
        P: NestedProgress,
        P::SubProgress: NestedProgress + 'static,
    {
        match checkout_tree(&self.repo, &self.root_tree_id, progress, should_interrupt) {
            Ok(outcome) => Ok((self.persist()?, outcome)),
            Err(err) => {
                if let Err(cleanup) = self.rollback() {
                    return Err(err.and_raise(cleanup));
                }
                Err(err)
            }
        }
    }
}

pub(crate) fn checkout_tree<P>(
    repo: &crate::Repository,
    root_tree_id: &gix_hash::oid,
    mut progress: P,
    should_interrupt: &AtomicBool,
) -> Result<gix_worktree_state::checkout::Outcome>
where
    P: NestedProgress,
    P::SubProgress: NestedProgress + 'static,
{
    ensure!(!should_interrupt.load(Ordering::Relaxed), Error::Interrupted);
    let repo = repo.with_absolute_paths()?;
    let work_dir = repo
        .workdir()
        .ok_or_raise(|| validation("A bare repository has no worktree to check out"))?;
    let mut index = repo
        .index_from_tree(root_tree_id)
        .or_raise(|| message("Could not create an index from the target tree"))?;
    let mut checkout_options = repo.checkout_options(gix_worktree::stack::state::attributes::Source::IdMapping)?;
    checkout_options.destination_is_initially_empty = true;

    let mut files = progress.add_child("checkout");
    let mut bytes = progress.add_child("writing");
    files.init(Some(index.entries().len()), crate::progress::count("files"));
    bytes.init(None, crate::progress::bytes());
    let started = std::time::Instant::now();
    let outcome = gix_worktree_state::checkout(
        &mut index,
        work_dir,
        repo.objects
            .clone()
            .into_arc()
            .or_raise(|| message("Failed to reopen the object database for checkout"))?,
        &files,
        &bytes,
        should_interrupt,
        checkout_options,
    )?;
    files.show_throughput(started);
    bytes.show_throughput(started);
    ensure!(!should_interrupt.load(Ordering::Relaxed), Error::Interrupted);
    index
        .write(Default::default())
        .or_raise(|| message("Could not write the linked worktree index"))?;
    Ok(outcome)
}
fn copy_worktree_config(source: &Path, destination: &Path) -> Result<()> {
    let mut config = match gix_config::File::from_path_no_includes(source.to_owned(), gix_config::Source::Worktree) {
        Ok(config) => config,
        Err(err) if err.is_not_found() => return Ok(()),
        Err(err) => {
            bail!(err.and_raise(message("Could not read the source worktree configuration")));
        }
    };
    if Core::BARE.enrich_error(config.boolean(Core::BARE))?.unwrap_or_default()
        && let Ok(mut values) = config.raw_values_mut(Core::BARE)
    {
        values.delete_all();
    }
    if let Ok(mut values) = config.raw_values_mut(Core::WORKTREE) {
        values.delete_all();
    }
    let mut destination =
        std::fs::File::create(destination).or_raise(|| message("Could not write the new worktree configuration"))?;
    config
        .write_to(&mut destination)
        .or_raise(|| message("Could not write the new worktree configuration"))?;
    Ok(())
}

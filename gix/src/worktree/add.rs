use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use gix_features::progress::{NestedProgress, Progress};
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

/// The error returned by [`Repository::add_worktree()`][crate::Repository::add_worktree()].
#[derive(Debug, thiserror::Error)]
#[expect(missing_docs)]
pub enum Error {
    #[error(transparent)]
    ConfigBoolean(#[from] crate::config::boolean::Error),
    #[error("Could not enable relative worktrees in the shared configuration")]
    Configure(#[from] crate::config::file_mut::Error),
    #[error("Could not set the repository format for relative worktrees")]
    SetConfig(#[from] gix_config::file::set_raw_value::Error),
    #[error("Could not upgrade the repository format for relative worktrees")]
    RepositoryFormat(#[from] crate::config::Error),
    #[error("Cannot upgrade repository format with unsupported extension {name:?}")]
    UnsupportedExtension { name: String },
    #[error("Could not read the source worktree configuration")]
    ReadWorktreeConfig(#[source] gix_config::file::init::from_paths::Error),
    #[error("Could not write the new worktree configuration")]
    WriteWorktreeConfig(#[source] std::io::Error),
    #[error("{name:?} is not a local branch")]
    NotLocalBranch { name: gix_ref::FullName },
    #[error("The local branch {name:?} is checked out in {worktree_dirs:?}")]
    CheckedOut {
        name: gix_ref::FullName,
        worktree_dirs: Vec<std::path::PathBuf>,
    },
    #[error("The worktree destination {destination:?} is already registered")]
    DestinationRegistered { destination: std::path::PathBuf },
    #[error("Failed to read or iterate worktree directories")]
    WorktreeListing(#[source] std::io::Error),
    #[error("Could not open a worktree repository")]
    OpenWorktreeRepo(#[source] crate::open::Error),
    #[error("Failed to follow a symbolic reference while inspecting worktrees")]
    FollowSymref(#[source] gix_ref::file::find::existing::Error),
    #[error("The local branch could not be found")]
    FindBranch(#[from] crate::reference::find::existing::Error),
    #[error("The local branch could not be peeled to a commit")]
    PeelBranch(#[from] crate::reference::peel::to_kind::Error),
    #[error("The detached target is not an existing commit")]
    FindDetachedCommit(#[from] crate::object::find::existing::with_conversion::Error),
    #[error("Could not decode the commit")]
    DecodeCommit(#[from] gix_object::decode::Error),
    #[error("Could not prepare the linked worktree")]
    Prepare(#[source] std::io::Error),
    #[error("Could not write the linked worktree HEAD")]
    WriteHead(#[source] std::io::Error),
    #[error("Could not initialize the linked worktree HEAD and its reflog")]
    InitializeHead(#[source] crate::reference::edit::Error),
    #[error("Could not create an index from the target tree")]
    IndexFromTree(#[from] crate::repository::index_from_tree::Error),
    #[error(transparent)]
    CheckoutOptions(#[from] crate::config::checkout_options::Error),
    #[error("Failed to reopen the object database for checkout")]
    OpenArcOdb(#[source] std::io::Error),
    #[error(transparent)]
    Checkout(#[from] gix_worktree_state::checkout::Error),
    #[error("Adding the worktree was interrupted")]
    Interrupted,
    #[error("Could not write the linked worktree index")]
    WriteIndex(#[from] gix_index::file::write::Error),
    #[error("Could not finish adding the linked worktree")]
    Persist(#[source] std::io::Error),
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
    /// With `extensions.worktreeConfig`, the source's `config.worktree` is copied before checkout, excluding
    /// `core.worktree` and a true `core.bare` setting.
    /// The new `HEAD` reflog records its initial commit when `core.logAllRefUpdates` permits it.
    ///
    /// `core.sharedRepository` applies to repository metadata, including `HEAD`, its reflog, and the index.
    /// Like Git when preparing `<destination>/.git`, this also applies shared permissions to newly created
    /// destination and parent directories, as well as the common `worktrees` directory. Existing directories
    /// retain their permissions. The private Git directory and checked-out files and subdirectories use normal
    /// filesystem permissions, including the umask. As a deviation from Git, the linking files `.git`, `gitdir`,
    /// `commondir`, and `locked` also receive shared permissions.
    ///
    /// `worktree.useRelativePaths` selects relative links instead of the default absolute links. When enabled,
    /// the shared config is upgraded to repository format version 1 with `extensions.relativeWorktrees=true`.
    /// This compatibility marker remains set even if checkout fails, and requires Git 2.48 or newer.
    /// The parent repository's configuration snapshot is unchanged; call [`reload()`][Self::reload()] to refresh it.
    pub fn add_worktree<P>(
        &self,
        destination: impl AsRef<Path>,
        head: Head,
        mut progress: P,
        should_interrupt: &AtomicBool,
    ) -> Result<(crate::Repository, gix_worktree_state::checkout::Outcome), Error>
    where
        P: NestedProgress,
        P::SubProgress: NestedProgress + 'static,
    {
        let destination = destination.as_ref();
        let relative_paths = Worktree::USE_RELATIVE_PATHS
            .enrich_error(self.config.resolved.boolean(Worktree::USE_RELATIVE_PATHS))
            .with_leniency(self.config.lenient_config)?
            .unwrap_or_default();
        let (head_target, commit_id, root_tree_id) = match head {
            Head::Attached(name) => {
                if name.category() != Some(gix_ref::Category::LocalBranch) {
                    return Err(Error::NotLocalBranch { name });
                }
                let checked_out = self.checked_out_branches(None).map_err(|err| match err {
                    crate::repository::worktree::CheckedOutBranchesError::WorktreeListing(err) => {
                        Error::WorktreeListing(err)
                    }
                    crate::repository::worktree::CheckedOutBranchesError::OpenWorktreeRepo(err) => {
                        Error::OpenWorktreeRepo(err)
                    }
                    crate::repository::worktree::CheckedOutBranchesError::FollowSymref(err) => Error::FollowSymref(err),
                })?;
                if let Some(worktree_dirs) = checked_out.get(&name) {
                    return Err(Error::CheckedOut {
                        name,
                        worktree_dirs: worktree_dirs.clone(),
                    });
                }
                let mut source = self.clone();
                source.clear_namespace();
                let mut reference = source.find_reference(name.as_ref())?;
                let commit = reference.peel_to_commit()?;
                let root_tree_id = commit.tree_id()?.detach();
                (gix_ref::Target::Symbolic(name), commit.id, root_tree_id)
            }
            Head::Detached(commit_id) => {
                let root_tree_id = self.find_commit(commit_id)?.tree_id()?.detach();
                (gix_ref::Target::Object(commit_id), commit_id, root_tree_id)
            }
        };
        if should_interrupt.load(Ordering::Relaxed) {
            return Err(Error::Interrupted);
        }

        let main_repo = self.main_repo().map_err(Error::OpenWorktreeRepo)?;
        let mut registered_destinations = main_repo.workdir().map(Path::to_owned).into_iter().collect::<Vec<_>>();
        for worktree in self.worktrees().map_err(Error::WorktreeListing)? {
            registered_destinations.push(worktree.base().map_err(Error::WorktreeListing)?);
        }
        let prepared = gix_worktree::add::prepare(
            self.common_dir(),
            destination,
            gix_worktree::add::Options {
                relative_paths,
                shared_repository_permissions: self.config.shared_repository_permissions,
            },
        )
        .map_err(Error::Prepare)?;
        let canonical_destination = std::fs::canonicalize(prepared.work_dir()).map_err(Error::Prepare)?;
        for registered_destination in registered_destinations {
            let registered_destination = gix_path::realpath(registered_destination)
                .map_err(|err| Error::WorktreeListing(std::io::Error::other(err)))?;
            let same_destination = if registered_destination == prepared.work_dir() {
                true
            } else {
                match std::fs::canonicalize(&registered_destination) {
                    Ok(path) => path == canonical_destination,
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => false,
                    Err(err) => return Err(Error::WorktreeListing(err)),
                }
            };
            if same_destination {
                return Err(Error::DestinationRegistered {
                    destination: destination.to_owned(),
                });
            }
        }
        if relative_paths {
            let mut config = self.config_file_mut(self.common_dir().join("config"))?;
            let version = Core::REPOSITORY_FORMAT_VERSION
                .try_into_usize(config.integer(Core::REPOSITORY_FORMAT_VERSION))
                .map_err(crate::config::Error::from)?
                .unwrap_or_default();
            if version > 1 {
                return Err(crate::config::Error::UnsupportedRepositoryFormatVersion { version }.into());
            }
            if version == 0 {
                // These are Git's grandfathered v0 extensions. Others become significant on upgrade.
                for section in config.sections_by_name("extensions").into_iter().flatten() {
                    for name in section.value_names() {
                        if section.header().subsection_name().is_some()
                            || !["noop", "preciousobjects", "partialclone", "worktreeconfig"]
                                .iter()
                                .any(|known| name.eq_ignore_ascii_case(known))
                        {
                            return Err(Error::UnsupportedExtension { name });
                        }
                    }
                }
                config.set_raw_value(Core::REPOSITORY_FORMAT_VERSION, "1")?;
            }
            let enabled = Extensions::RELATIVE_WORKTREES
                .enrich_error(config.boolean(Extensions::RELATIVE_WORKTREES))?
                .unwrap_or_default();
            if version == 0 || !enabled {
                config.set_raw_value(Extensions::RELATIVE_WORKTREES, "true")?;
                config.commit()?;
            }
        }
        // Opening the repository requires HEAD. Preserve symbolic targets so conditional configuration
        // can see the branch; detached HEADs start at null so their first update records initialization.
        let mut head_contents = Vec::with_capacity(6 + self.object_hash().len_in_hex());
        let reflog_mode = match &head_target {
            gix_ref::Target::Object(_) => {
                self.object_hash()
                    .null()
                    .write_hex_to(&mut head_contents)
                    .map_err(Error::WriteHead)?;
                RefLog::AndReference
            }
            gix_ref::Target::Symbolic(name) => {
                head_contents.extend_from_slice(b"ref: ");
                head_contents.extend_from_slice(name.as_bstr());
                RefLog::Only
            }
        };
        head_contents.push(b'\n');
        std::fs::write(prepared.git_dir().join("HEAD"), head_contents).map_err(Error::WriteHead)?;

        if Extensions::WORKTREE_CONFIG
            .enrich_error(self.config.resolved.boolean(Extensions::WORKTREE_CONFIG))
            .with_leniency(self.config.lenient_config)?
            .unwrap_or_default()
        {
            copy_worktree_config(
                &self.git_dir().join("config.worktree"),
                &prepared.git_dir().join("config.worktree"),
            )?;
        }

        let options = self
            .options
            .clone()
            .without_repository_environment_overrides()
            .open_path_as_is(true);
        let mut repo = crate::ThreadSafeRepository::open_opts(prepared.git_dir(), options)
            .map_err(Error::OpenWorktreeRepo)?
            .to_thread_local();
        repo.clear_namespace();
        gix_fs::set_shared_repository_permissions(
            &repo.git_dir().join("HEAD"),
            repo.config.shared_repository_permissions,
        )
        .map_err(Error::WriteHead)?;
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
        .map_err(Error::InitializeHead)?;
        let mut index = repo.index_from_tree(&root_tree_id)?;
        let mut checkout_options = repo.checkout_options(gix_worktree::stack::state::attributes::Source::IdMapping)?;
        checkout_options.destination_is_initially_empty = true;

        let mut files = progress.add_child("checkout");
        let mut bytes = progress.add_child("writing");
        files.init(Some(index.entries().len()), crate::progress::count("files"));
        bytes.init(None, crate::progress::bytes());
        let started = std::time::Instant::now();
        let outcome = gix_worktree_state::checkout(
            &mut index,
            prepared.work_dir(),
            repo.objects.clone().into_arc().map_err(Error::OpenArcOdb)?,
            &files,
            &bytes,
            should_interrupt,
            checkout_options,
        )?;
        files.show_throughput(started);
        bytes.show_throughput(started);
        if should_interrupt.load(Ordering::Relaxed) {
            return Err(Error::Interrupted);
        }
        index.write(Default::default(), repo.config.shared_repository_permissions)?;
        prepared.persist().map_err(Error::Persist)?;
        Ok((repo, outcome))
    }
}

fn copy_worktree_config(source: &Path, destination: &Path) -> Result<(), Error> {
    let mut config = match gix_config::File::from_path_no_includes(source.to_owned(), gix_config::Source::Worktree) {
        Ok(config) => config,
        Err(gix_config::file::init::from_paths::Error::Io { source, .. })
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            return Ok(());
        }
        Err(err) => return Err(Error::ReadWorktreeConfig(err)),
    };
    if Core::BARE.enrich_error(config.boolean(Core::BARE))?.unwrap_or_default()
        && let Ok(mut values) = config.raw_values_mut(Core::BARE)
    {
        values.delete_all();
    }
    if let Ok(mut values) = config.raw_values_mut(Core::WORKTREE) {
        values.delete_all();
    }
    config
        .write_to(&mut std::fs::File::create(destination).map_err(Error::WriteWorktreeConfig)?)
        .map_err(Error::WriteWorktreeConfig)
}

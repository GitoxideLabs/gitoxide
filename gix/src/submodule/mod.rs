#![allow(clippy::result_large_err)]
//! Submodule plumbing and abstractions
//!
use std::{
    borrow::Cow,
    cell::{Ref, RefCell, RefMut},
    path::PathBuf,
};

pub use gix_submodule::*;

use crate::{bstr::BStr, is_dir_to_mode, worktree::IndexPersistedOrInMemory, Repository, Submodule};

pub(crate) type ModulesFileStorage = gix_features::threading::OwnShared<gix_fs::SharedFileSnapshotMut<File>>;
/// A lazily loaded and auto-updated worktree index.
pub type ModulesSnapshot = gix_fs::SharedFileSnapshot<File>;

/// The name of the file containing (sub) module information.
pub(crate) const MODULES_FILE: &str = ".gitmodules";

mod errors;
pub use errors::*;

/// A platform maintaining state needed to interact with submodules, created by [`Repository::submodules()].
pub(crate) struct SharedState<'repo> {
    pub repo: &'repo Repository,
    pub(crate) modules: ModulesSnapshot,
    is_active: RefCell<Option<IsActiveState>>,
    index: RefCell<Option<IndexPersistedOrInMemory>>,
}

impl<'repo> SharedState<'repo> {
    pub(crate) fn new(repo: &'repo Repository, modules: ModulesSnapshot) -> Self {
        SharedState {
            repo,
            modules,
            is_active: RefCell::new(None),
            index: RefCell::new(None),
        }
    }

    fn index(&self) -> Result<Ref<'_, IndexPersistedOrInMemory>, crate::repository::index_or_load_from_head::Error> {
        {
            let mut state = self.index.borrow_mut();
            if state.is_none() {
                *state = self.repo.index_or_load_from_head()?.into();
            }
        }
        Ok(Ref::map(self.index.borrow(), |opt| {
            opt.as_ref().expect("just initialized")
        }))
    }

    fn active_state_mut(
        &self,
    ) -> Result<(RefMut<'_, IsActivePlatform>, RefMut<'_, gix_worktree::Stack>), is_active::Error> {
        let mut state = self.is_active.borrow_mut();
        if state.is_none() {
            let platform = self
                .modules
                .is_active_platform(&self.repo.config.resolved, self.repo.config.pathspec_defaults()?)?;
            let index = self.index()?;
            let attributes = self
                .repo
                .attributes_only(
                    &index,
                    gix_worktree::stack::state::attributes::Source::WorktreeThenIdMapping
                        .adjust_for_bare(self.repo.is_bare()),
                )?
                .detach();
            *state = Some(IsActiveState { platform, attributes });
        }
        Ok(RefMut::map_split(state, |opt| {
            let state = opt.as_mut().expect("populated above");
            (&mut state.platform, &mut state.attributes)
        }))
    }
}

struct IsActiveState {
    platform: IsActivePlatform,
    attributes: gix_worktree::Stack,
}

///Access
impl Submodule<'_> {
    /// Return the submodule's name.
    pub fn name(&self) -> &BStr {
        self.name.as_ref()
    }
    /// Return the path at which the submodule can be found, relative to the repository.
    ///
    /// For details, see [gix_submodule::File::path()].
    pub fn path(&self) -> Result<Cow<'_, BStr>, config::path::Error> {
        self.state.modules.path(self.name())
    }

    /// Return the url from which to clone or update the submodule.
    ///
    /// This method takes into consideration submodule configuration overrides.
    pub fn url(&self) -> Result<gix_url::Url, config::url::Error> {
        self.state.modules.url(self.name())
    }

    /// Return the `update` field from this submodule's configuration, if present, or `None`.
    ///
    /// This method takes into consideration submodule configuration overrides.
    pub fn update(&self) -> Result<Option<config::Update>, config::update::Error> {
        self.state.modules.update(self.name())
    }

    /// Return the `branch` field from this submodule's configuration, if present, or `None`.
    ///
    /// This method takes into consideration submodule configuration overrides.
    pub fn branch(&self) -> Result<Option<config::Branch>, config::branch::Error> {
        self.state.modules.branch(self.name())
    }

    /// Return the `fetchRecurseSubmodules` field from this submodule's configuration, or retrieve the value from `fetch.recurseSubmodules` if unset.
    pub fn fetch_recurse(&self) -> Result<Option<config::FetchRecurse>, fetch_recurse::Error> {
        Ok(match self.state.modules.fetch_recurse(self.name())? {
            Some(val) => Some(val),
            None => self
                .state
                .repo
                .config
                .resolved
                .boolean("fetch.recurseSubmodules")
                .map(|res| crate::config::tree::Fetch::RECURSE_SUBMODULES.try_into_recurse_submodules(res))
                .transpose()?,
        })
    }

    /// Return the `ignore` field from this submodule's configuration, if present, or `None`.
    ///
    /// This method takes into consideration submodule configuration overrides.
    pub fn ignore(&self) -> Result<Option<config::Ignore>, config::Error> {
        self.state.modules.ignore(self.name())
    }

    /// Return the `shallow` field from this submodule's configuration, if present, or `None`.
    ///
    /// If `true`, the submodule will be checked out with `depth = 1`. If unset, `false` is assumed.
    pub fn shallow(&self) -> Result<Option<bool>, gix_config::value::Error> {
        self.state.modules.shallow(self.name())
    }

    /// Returns true if this submodule is considered active and can thus participate in an operation.
    ///
    /// Please see the [plumbing crate documentation](gix_submodule::IsActivePlatform::is_active()) for details.
    pub fn is_active(&self) -> Result<bool, is_active::Error> {
        let (mut platform, mut attributes) = self.state.active_state_mut()?;
        let is_active = platform.is_active(&self.state.repo.config.resolved, self.name.as_ref(), {
            &mut |relative_path, case, is_dir, out| {
                attributes
                    .set_case(case)
                    .at_entry(relative_path, Some(is_dir_to_mode(is_dir)), &self.state.repo.objects)
                    .is_ok_and(|platform| platform.matching_attributes(out))
            }
        })?;
        Ok(is_active)
    }

    /// Return the object id of the submodule as stored in the index of the superproject,
    /// or `None` if it was deleted from the index.
    ///
    /// If `None`, but `Some()` when calling [`Self::head_id()`], then the submodule was just deleted but the change
    /// wasn't yet committed. Note that `None` is also returned if the entry at the submodule path isn't a submodule.
    /// If `Some()`, but `None` when calling [`Self::head_id()`], then the submodule was just added without having committed the change.
    pub fn index_id(&self) -> Result<Option<gix_hash::ObjectId>, index_id::Error> {
        let path = self.path()?;
        Ok(self
            .state
            .index()?
            .entry_by_path(&path)
            .and_then(|entry| (entry.mode == gix_index::entry::Mode::COMMIT).then_some(entry.id)))
    }

    /// Return the object id of the submodule as stored in `HEAD^{tree}` of the superproject, or `None` if it wasn't yet committed.
    ///
    /// If `Some()`, but `None` when calling [`Self::index_id()`], then the submodule was just deleted but the change
    /// wasn't yet committed. Note that `None` is also returned if the entry at the submodule path isn't a submodule.
    /// If `None`, but `Some()` when calling [`Self::index_id()`], then the submodule was just added without having committed the change.
    pub fn head_id(&self) -> Result<Option<gix_hash::ObjectId>, head_id::Error> {
        let path = self.path()?;
        Ok(self
            .state
            .repo
            .head_commit()?
            .tree()?
            .peel_to_entry_by_path(gix_path::from_bstr(path.as_ref()))?
            .and_then(|entry| (entry.mode().is_commit()).then_some(entry.inner.oid)))
    }

    /// Return the path at which the repository of the submodule should be located.
    ///
    /// The directory might not exist yet.
    pub fn git_dir(&self) -> PathBuf {
        self.state
            .repo
            .common_dir()
            .join("modules")
            .join(gix_path::from_bstr(self.name()))
    }

    /// Return the path to the location at which the workdir would be checked out.
    ///
    /// Note that it may be a path relative to the repository if, for some reason, the parent directory
    /// doesn't have a working dir set.
    pub fn work_dir(&self) -> Result<PathBuf, config::path::Error> {
        let worktree_git = gix_path::from_bstr(self.path()?);
        Ok(match self.state.repo.workdir() {
            None => worktree_git.into_owned(),
            Some(prefix) => prefix.join(worktree_git),
        })
    }

    /// Return the path at which the repository of the submodule should be located, or the path inside of
    /// the superproject's worktree where it actually *is* located if the submodule in the 'old-form', thus is a directory
    /// inside of the superproject's work-tree.
    ///
    /// Note that 'old-form' paths returned aren't verified, i.e. the `.git` repository might be corrupt or otherwise
    /// invalid - it's left to the caller to try to open it.
    ///
    /// Also note that the returned path may not actually exist.
    pub fn git_dir_try_old_form(&self) -> Result<PathBuf, config::path::Error> {
        let worktree_git = self.work_dir()?.join(gix_discover::DOT_GIT_DIR);
        Ok(if worktree_git.is_dir() {
            worktree_git
        } else {
            self.git_dir()
        })
    }

    /// Query various parts of the submodule and assemble it into state information.
    #[doc(alias = "status", alias = "git2")]
    pub fn state(&self) -> Result<State, config::path::Error> {
        let maybe_old_path = self.git_dir_try_old_form()?;
        let git_dir = self.git_dir();
        let worktree_git = self.work_dir()?.join(gix_discover::DOT_GIT_DIR);
        let superproject_configuration = self
            .state
            .repo
            .config
            .resolved
            .sections_by_name("submodule")
            .into_iter()
            .flatten()
            .any(|section| section.header().subsection_name() == Some(self.name.as_ref()));
        Ok(State {
            repository_exists: maybe_old_path.is_dir(),
            is_old_form: maybe_old_path != git_dir,
            worktree_checkout: worktree_git.exists(),
            superproject_configuration,
        })
    }

    /// Open the submodule as repository, or `None` if the submodule wasn't initialized yet.
    ///
    /// More states can be derived here:
    ///
    /// * *initialized* - a repository exists, i.e. `Some(repo)` and the working tree is present.
    /// * *uninitialized* - a repository does not exist, i.e. `None`
    /// * *deinitialized* - a repository does exist, i.e. `Some(repo)`, but its working tree is empty.
    ///
    /// Also see the [state()](Self::state()) method for learning about the submodule.
    /// The repository can also be used to learn about the submodule `HEAD`, i.e. where its working tree is at,
    /// which may differ compared to the superproject's index or `HEAD` commit.
    pub fn open(&self) -> Result<Option<Repository>, open::Error> {
        match crate::open_opts(self.git_dir_try_old_form()?, self.state.repo.options.clone()) {
            Ok(repo) => Ok(Some(repo)),
            Err(crate::open::Error::NotARepository { .. }) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}

///
#[cfg(feature = "status")]
pub mod status {
    use super::{head_id, index_id, open, Status};
    use crate::Submodule;
    use gix_submodule::config;

    /// The error returned by [Submodule::status()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        State(#[from] config::path::Error),
        #[error(transparent)]
        HeadId(#[from] head_id::Error),
        #[error(transparent)]
        IndexId(#[from] index_id::Error),
        #[error(transparent)]
        OpenRepository(#[from] open::Error),
        #[error(transparent)]
        IgnoreConfiguration(#[from] config::Error),
        #[error(transparent)]
        StatusPlatform(#[from] crate::status::Error),
        #[error(transparent)]
        StatusIter(#[from] crate::status::into_iter::Error),
        #[error(transparent)]
        NextStatusItem(#[from] crate::status::iter::Error),
    }

    impl Submodule<'_> {
        /// Return the status of the submodule.
        ///
        /// Use `ignore` to control the portion of the submodule status to ignore. It can be obtained from
        /// submodule configuration using the [`ignore()`](Submodule::ignore()) method.
        /// If `check_dirty` is `true`, the computation will stop once the first in a ladder operations
        /// ordered from cheap to expensive shows that the submodule is dirty.
        /// Thus, submodules that are clean will still impose the complete set of computation, as given.
        #[doc(alias = "submodule_status", alias = "git2")]
        pub fn status(
            &self,
            ignore: config::Ignore,
            check_dirty: bool,
        ) -> Result<crate::submodule::status::types::Status, Error> {
            self.status_opts(ignore, check_dirty, &mut |s| s)
        }
        /// Return the status of the submodule, just like [`status`](Self::status), but allows to adjust options
        /// for more control over how the status is performed.
        ///
        /// If `check_dirty` is `true`, the computation will stop once the first in a ladder operations
        /// ordered from cheap to expensive shows that the submodule is dirty. When checking for detailed
        /// status information (i.e. untracked file, modifications, HEAD-index changes) only the first change
        /// will be kept to stop as early as possible.
        ///
        /// Use `&mut std::convert::identity` for `adjust_options` if no specific options are desired.
        /// A reason to change them might be to enable sorting to enjoy deterministic order of changes.
        ///
        /// The status allows to easily determine if a submodule [has changes](Status::is_dirty).
        #[doc(alias = "submodule_status", alias = "git2")]
        pub fn status_opts(
            &self,
            ignore: config::Ignore,
            check_dirty: bool,
            adjust_options: &mut dyn for<'a> FnMut(
                crate::status::Platform<'a, gix_features::progress::Discard>,
            )
                -> crate::status::Platform<'a, gix_features::progress::Discard>,
        ) -> Result<Status, Error> {
            let mut state = self.state()?;
            if ignore == config::Ignore::All {
                return Ok(Status {
                    state,
                    ..Default::default()
                });
            }

            let index_id = self.index_id()?;
            if !state.repository_exists {
                return Ok(Status {
                    state,
                    index_id,
                    ..Default::default()
                });
            }
            let sm_repo = match self.open()? {
                None => {
                    state.repository_exists = false;
                    return Ok(Status {
                        state,
                        index_id,
                        ..Default::default()
                    });
                }
                Some(repo) => repo,
            };

            let checked_out_head_id = sm_repo.head_id().ok().map(crate::Id::detach);
            let mut status = Status {
                state,
                index_id,
                checked_out_head_id,
                ..Default::default()
            };
            if ignore == config::Ignore::Dirty || check_dirty && status.is_dirty() == Some(true) {
                return Ok(status);
            }

            if !state.worktree_checkout {
                return Ok(status);
            }
            let statuses = adjust_options(sm_repo.status(gix_features::progress::Discard)?)
                .index_worktree_options_mut(|opts| {
                    if ignore == config::Ignore::Untracked {
                        opts.dirwalk_options = None;
                    }
                })
                .into_iter(None)?;
            let mut changes = Vec::new();
            for change in statuses {
                changes.push(change?);
                if check_dirty {
                    break;
                }
            }
            status.changes = Some(changes);
            Ok(status)
        }
    }

    impl Status {
        /// Return `Some(true)` if the submodule status could be determined sufficiently and
        /// if there are changes that would render this submodule dirty.
        ///
        /// Return `Some(false)` if the submodule status could be determined and it has no changes
        /// at all.
        ///
        /// Return `None` if the repository clone or the worktree are missing entirely, which would leave
        /// it to the caller to determine if that's considered dirty or not.
        pub fn is_dirty(&self) -> Option<bool> {
            if !self.state.worktree_checkout || !self.state.repository_exists {
                return None;
            }
            let is_dirty =
                self.checked_out_head_id != self.index_id || self.changes.as_ref().is_some_and(|c| !c.is_empty());
            Some(is_dirty)
        }
    }

    pub(super) mod types {
        use crate::submodule::State;

        /// A simplified status of the Submodule.
        ///
        /// As opposed to the similar-sounding [`State`], it is more exhaustive and potentially expensive to compute,
        /// particularly for submodules without changes.
        ///
        /// It's produced by [Submodule::status()](crate::Submodule::status()).
        #[derive(Default, Clone, PartialEq, Debug)]
        pub struct Status {
            /// The cheapest part of the status that is always performed, to learn if the repository is cloned
            /// and if there is a worktree checkout.
            pub state: State,
            /// The commit at which the submodule is supposed to be according to the super-project's index.
            /// `None` means the computation wasn't performed, or the submodule didn't exist in the super-project's index anymore.
            pub index_id: Option<gix_hash::ObjectId>,
            /// The commit-id of the `HEAD` at which the submodule is currently checked out.
            /// `None` if the computation wasn't performed as it was skipped early, or if no repository was available or
            /// if the HEAD could not be obtained or wasn't born.
            pub checked_out_head_id: Option<gix_hash::ObjectId>,
            /// The set of changes obtained from running something akin to `git status` in the submodule working tree.
            ///
            /// `None` if the computation wasn't performed as the computation was skipped early, or if no working tree was
            /// available or repository was available.
            pub changes: Option<Vec<crate::status::Item>>,
        }
    }
}
#[cfg(feature = "status")]
pub use status::types::Status;

/// A summary of the state of all parts forming a submodule, which allows to answer various questions about it.
///
/// Note that expensive questions about its presence in the `HEAD` or the `index` are left to the caller.
#[derive(Default, Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct State {
    /// if the submodule repository has been cloned.
    pub repository_exists: bool,
    /// if the submodule repository is located directly in the worktree of the superproject.
    pub is_old_form: bool,
    /// if the worktree is checked out.
    pub worktree_checkout: bool,
    /// If submodule configuration was found in the superproject's `.git/config` file.
    /// Note that the presence of a single section is enough, independently of the actual values.
    pub superproject_configuration: bool,
}

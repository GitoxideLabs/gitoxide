///
pub mod open_modules_file {
    /// The error returned by [Repository::open_modules_file()](crate::Repository::open_modules_file()).
    pub type Error = gix_error::Error;
}

///
pub mod modules {
    /// The error returned by [Repository::modules()](crate::Repository::modules()).
    pub type Error = gix_error::Error;
}

///
pub mod is_active {
    /// The error returned by [Submodule::is_active()](crate::Submodule::is_active()).
    pub type Error = gix_error::Error;
}

///
pub mod fetch_recurse {
    /// The error returned by [Submodule::fetch_recurse()](crate::Submodule::fetch_recurse()).
    pub type Error = gix_error::Error;
}

///
pub mod open {
    // TODO(review): kept concrete. Matched at `gix/tests/gix/submodule.rs:343-344` and `:800-801`:
    //                `Error::GitDir(git_dir_try_old_form::Error::InvalidGitDirFileTarget { .. })` /
    //                `::GitDir(..)`. Separately, `submodule::status::Error::OpenRepository`
    //                (`gix/src/submodule/mod.rs`) already has an erased slot via `StatusIter`, so
    //                this type is doubly blocked from erasure there.
    /// The error returned by [Submodule::open()](crate::Submodule::open()).
    #[derive(Debug, thiserror::Error)]
    #[expect(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        OpenRepository(#[from] crate::open::Error),
        #[error(transparent)]
        GitDir(#[from] crate::submodule::git_dir_try_old_form::Error),
        #[error(transparent)]
        PathConfiguration(#[from] gix_submodule::config::path::Error),
        #[error(transparent)]
        WorktreeDirInaccessible(#[from] std::io::Error),
    }
}

///
pub mod git_dir_try_old_form {
    // TODO(review): kept concrete. Callers destructure its `InvalidGitDirFileTarget { gitdir_file,
    //                target, source }` variant at `gix/tests/gix/submodule.rs:325`, `:334`, `:344`,
    //                `:358`, `:398` and `:412`; `:793` matches its `GitDir(..)` variant. Its two
    //                `#[from]` parents — `open::Error::GitDir` (above) and `state::Error::
    //                GitDirTryOldForm` (below) — have no other erased member.
    /// The error returned by [Submodule::git_dir_try_old_form()](crate::Submodule::git_dir_try_old_form()).
    #[derive(Debug, thiserror::Error)]
    #[expect(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        PathConfiguration(#[from] gix_submodule::config::path::Error),
        #[error("The gitdir file at '{}' contains an invalid gitdir target: {:?}", .gitdir_file.display(), .target)]
        InvalidGitDirFileTarget {
            gitdir_file: std::path::PathBuf,
            target: Option<std::path::PathBuf>,
            #[source]
            source: Option<gix_discover::path::from_gitdir_file::Error>,
        },
        #[error(transparent)]
        GitDir(#[from] gix_validate::submodule::name::Error),
    }
}

///
pub mod state {
    // TODO(review): kept concrete. Matched at `gix/tests/gix/submodule.rs:333`, `:411` and `:808`:
    //                `Error::GitDirTryOldForm(..)`. Separately, `submodule::status::Error::State`
    //                (`gix/src/submodule/mod.rs`) already has an erased slot via `StatusIter`, so
    //                this type is doubly blocked from erasure there.
    /// The error returned by [Submodule::state()](crate::Submodule::state()).
    #[derive(Debug, thiserror::Error)]
    #[expect(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        GitDirTryOldForm(#[from] crate::submodule::git_dir_try_old_form::Error),
        #[error(transparent)]
        GitDir(#[from] gix_validate::submodule::name::Error),
        #[error(transparent)]
        PathConfiguration(#[from] gix_submodule::config::path::Error),
    }
}

///
pub mod index_id {
    /// The error returned by [Submodule::index_id()](crate::Submodule::index_id()).
    // TODO(review): kept concrete because `submodule::status::Error` already embeds the erased
    //                `status::into_iter::Error` (via `StatusIter`); erasing this one would give it
    //                a second `From<gix::Error>` via `IndexId`.
    #[derive(Debug, thiserror::Error)]
    #[expect(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        PathConfiguration(#[from] gix_submodule::config::path::Error),
        // TODO(review): embeds an erased `gix_error::Error`; erasing a sibling variant would collide
        //                with it via a duplicate `From<gix::Error>` impl (E0119).
        #[error(transparent)]
        Index(#[from] crate::repository::index_or_load_from_head::Error),
    }
}

///
pub mod head_id {
    /// The error returned by [Submodule::head_id()](crate::Submodule::head_id()).
    // TODO(review): kept concrete because `submodule::status::Error` already embeds the erased
    //                `status::into_iter::Error` (via `StatusIter`); erasing this one would give it
    //                a second `From<gix::Error>` via `HeadId`.
    #[derive(Debug, thiserror::Error)]
    #[expect(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        HeadCommit(#[from] crate::reference::head_commit::Error),
        #[error("Could not get tree of head commit")]
        CommitTree(#[from] crate::object::commit::Error),
        #[error("Could not peel tree to submodule path")]
        PeelTree(#[from] crate::object::find::existing::Error),
        #[error(transparent)]
        PathConfiguration(#[from] gix_submodule::config::path::Error),
    }
}

///
pub mod open_modules_file {
    /// The error returned by [Repository::open_modules_file()](crate::Repository::open_modules_file()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        Configuration(#[from] gix_config::parse::Error),
        #[error("Could not read '.gitmodules' file")]
        Io(#[from] std::io::Error),
    }
}

///
pub mod modules {
    /// The error returned by [Repository::modules()](crate::Repository::modules()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        OpenModulesFile(#[from] crate::submodule::open_modules_file::Error),
        #[error(transparent)]
        OpenIndex(#[from] crate::worktree::open_index::Error),
        #[error("Could not find the .gitmodules file by id in the object database")]
        FindExistingBlob(#[from] crate::object::find::existing::Error),
        #[error(transparent)]
        FindHeadRef(#[from] crate::reference::find::existing::Error),
        #[error(transparent)]
        PeelHeadRef(#[from] crate::head::peel::Error),
        #[error(transparent)]
        PeelObjectToCommit(#[from] crate::object::peel::to_kind::Error),
        #[error(transparent)]
        TreeFromCommit(#[from] crate::object::commit::Error),
    }
}

///
pub mod is_active {
    /// The error returned by [Submodule::is_active()](crate::Submodule::is_active()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        InitIsActivePlatform(#[from] gix_submodule::is_active_platform::Error),
        #[error(transparent)]
        QueryIsActive(#[from] gix_config::value::Error),
        #[error(transparent)]
        InitAttributes(#[from] crate::config::attribute_stack::Error),
        #[error(transparent)]
        InitPathspecDefaults(#[from] gix_pathspec::defaults::from_environment::Error),
        #[error(transparent)]
        ObtainIndex(#[from] crate::repository::index_or_load_from_head::Error),
    }
}

///
pub mod fetch_recurse {
    /// The error returned by [Submodule::fetch_recurse()](crate::Submodule::fetch_recurse()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        ModuleBoolean(#[from] gix_submodule::config::Error),
        #[error(transparent)]
        ConfigurationFallback(#[from] crate::config::key::GenericErrorWithValue),
    }
}

///
pub mod open {
    /// The error returned by [Submodule::open()](crate::Submodule::open()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
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
    /// The error returned by [Submodule::git_dir_try_old_form()](crate::Submodule::git_dir_try_old_form()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        PathConfiguration(#[from] gix_submodule::config::path::Error),
        #[error(transparent)]
        GitDir(#[from] gix_validate::submodule::name::Error),
    }
}

///
pub mod state {
    /// The error returned by [Submodule::state()](crate::Submodule::state()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
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
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        PathConfiguration(#[from] gix_submodule::config::path::Error),
        #[error(transparent)]
        Index(#[from] crate::repository::index_or_load_from_head::Error),
    }
}

///
pub mod head_id {
    /// The error returned by [Submodule::head_id()](crate::Submodule::head_id()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
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

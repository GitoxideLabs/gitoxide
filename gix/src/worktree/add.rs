use core::sync::atomic::AtomicBool;
use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use gix_lock::acquire;
use gix_ref::transaction::{self, RefEdit};
use gix_worktree_state::checkout;

use crate::{Commit, RefStore, Repository};

#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Bad .git structure")]
    BadDotGitStructure(),

    #[error("Can't create a worktree dir")]
    CantCreateWorktreeDir(#[source] std::io::Error),

    #[error("Can't resolve a ref")]
    CantResolveRef(gix_ref::Target),

    #[error("Can't setup a worktree dir")]
    CantSetupWorktreeDir(#[source] std::io::Error),

    #[error("Can't create index")]
    CantCreateIndex(#[from] crate::repository::index_from_tree::Error),

    #[error("Can't write index")]
    CantWriteIndex(#[from] gix_index::file::write::Error),

    #[error("Can't checkout a worktree dir")]
    CantCheckoutWorktreeDir(#[from] checkout::Error),

    #[error("Can't prepare linked worktree HEAD update")]
    PrepareHeadUpdate(#[from] gix_ref::file::transaction::prepare::Error),

    #[error("Can't obtain committer for linked worktree HEAD reflog")]
    Committer(#[from] crate::config::time::Error),

    #[error("Can't commit linked worktree HEAD update")]
    CommitHeadUpdate(#[from] gix_ref::file::transaction::commit::Error),
}

///
pub fn add_worktree<'repo>(
    repo: &'repo mut Repository,
    wt_dir: &Path,
    target: gix_ref::Target,
    files: &dyn gix_features::progress::Count,
    bytes: &dyn gix_features::progress::Count,
    should_interrupt: &AtomicBool,
    options: checkout::Options,
) -> Result<checkout::Outcome, Error> {
    let base_git_dir = repo.git_dir();

    let wt_repo_git_dir = wt_dir.join(".git");
    let wt_name = wt_dir.file_name().unwrap_or(OsStr::new("wt"));

    let wt_data_dir = create_wt_data_dir(&base_git_dir, wt_name)?;
    std::fs::create_dir_all(wt_dir).map_err(Error::CantCreateWorktreeDir)?;
    std::fs::write(wt_data_dir.join("commondir"), "../..\n").map_err(Error::CantSetupWorktreeDir)?;
    std::fs::write(wt_data_dir.join("gitdir"), format!("{}\n", wt_repo_git_dir.display()))
        .map_err(Error::CantSetupWorktreeDir)?;

    let target_tid = {
        let commit = resolve_target_commit(repo, &target)?;
        commit
            .tree_id()
            .map_err(|_| Error::CantResolveRef(target.clone()))?
            .detach()
    };
    let mut target_idx = repo.index_from_tree(&target_tid)?;
    target_idx.set_path(wt_data_dir.join("index"));

    let res = checkout(
        &mut target_idx,
        wt_dir,
        repo.objects.clone(),
        files,
        bytes,
        should_interrupt,
        options,
    )
    ?;
    target_idx.write(Default::default())?;
    std::fs::write(&wt_repo_git_dir, format!("gitdir: {}\n", wt_data_dir.display()))
        .map_err(Error::CantSetupWorktreeDir)?;

    let store = RefStore::for_linked_worktree(wt_data_dir.clone(), base_git_dir.into(), Default::default());
    let edit = RefEdit {
        name: "HEAD".try_into().unwrap(),
        deref: false,
        change: transaction::Change::Update {
            expected: transaction::PreviousValue::MustNotExist,
            new: target,
            log: transaction::LogChange {
                mode: transaction::RefLog::AndReference,
                force_create_reflog: true,
                message: "Worktree creation by tt-agents".into(),
            },
        },
    };
    let tx = store
        .transaction()
        .prepare([edit], acquire::Fail::Immediately, acquire::Fail::Immediately)?;
    let committer = repo.committer_or_set_generic_fallback()?;
    tx.commit(committer)?;

    Ok(res)
}

fn resolve_target_commit<'repo>(repo: &'repo Repository, t: &gix_ref::Target) -> Result<Commit<'repo>, Error> {
    let commit = match t {
        gix_ref::Target::Object(oid) => repo
            .find_object(*oid)
            .map_err(|_| Error::CantResolveRef(t.clone()))?
            .peel_to_commit()
            .map_err(|_| Error::CantResolveRef(t.clone()))?,
        gix_ref::Target::Symbolic(name) => repo
            .find_reference(name)
            .map_err(|_| Error::CantResolveRef(t.clone()))?
            .peel_to_commit()
            .map_err(|_| Error::CantResolveRef(t.clone()))?,
    };
    Ok(commit)
}

fn create_wt_data_dir(git_dir: &Path, wt_name: &OsStr) -> Result<PathBuf, Error> {
    let wts_dir = git_dir.join("worktrees");

    let _ = std::fs::create_dir_all(&wts_dir);
    if !wts_dir.is_dir() {
        return Err(Error::BadDotGitStructure());
    }

    // NOTE: We limit number of attempts to 1000 just in case of pathological fs issues
    let mut last_error = None;
    for i in 0..1000 {
        let wt_dir_name = if i == 0 {
            wt_name.to_owned()
        } else {
            let mut name = OsString::new();
            name.push(wt_name);
            name.push(OsStr::new("-"));
            name.push(format!("{i}"));
            name
        };
        let wt_dir = wts_dir.join(wt_dir_name);
        match std::fs::create_dir_all(&wt_dir) {
            Ok(()) => return Ok(wt_dir),
            Err(err) => last_error = Some(err),
        }
    }

    let source = last_error.unwrap_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "could not find an available worktree directory name",
        )
    });
    Err(Error::CantCreateWorktreeDir(source))
}

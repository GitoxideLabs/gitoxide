use anyhow::bail;
use gix::{
    bstr::{BString, ByteSlice},
    prelude::ObjectIdExt,
};
use unicode_width::UnicodeWidthStr;

use crate::OutputFormat;

pub fn add<P>(
    repo: gix::Repository,
    path: std::path::PathBuf,
    reference: BString,
    detach: bool,
    progress: P,
) -> anyhow::Result<()>
where
    P: gix::NestedProgress,
    P::SubProgress: gix::NestedProgress + 'static,
{
    let head = if detach {
        let commit_id = repo
            .rev_parse_single(reference.as_bstr())?
            .object()?
            .peel_to_commit()?
            .id;
        gix::worktree::create::Head::Detached(commit_id)
    } else {
        let branch = repo.find_reference(reference.as_bstr())?.name().to_owned();
        gix::worktree::create::Head::Attached(branch)
    };
    repo.create_worktree(path, head, progress, &gix::interrupt::IS_INTERRUPTED)?;
    Ok(())
}

pub fn remove<P>(repo: gix::Repository, worktree: std::path::PathBuf, force: u8, progress: P) -> anyhow::Result<()>
where
    P: gix::NestedProgress,
    P::SubProgress: gix::NestedProgress + 'static,
{
    let force = match force {
        0 => gix::worktree::remove::Force::Never,
        1 => gix::worktree::remove::Force::DiscardChanges,
        _ => gix::worktree::remove::Force::OverrideLock,
    };
    repo.remove_worktree(worktree, force, progress)?;
    Ok(())
}

pub fn list(repo: gix::Repository, out: &mut dyn std::io::Write, format: OutputFormat) -> anyhow::Result<()> {
    if format != OutputFormat::Human {
        bail!("JSON output isn't implemented yet");
    }
    let main_repo = repo.main_repo()?;
    let mut worktrees = Vec::new();

    if let Some(worktree) = main_repo.worktree() {
        worktrees.push(create_worktree_info(&main_repo, gix::path::realpath(worktree.base())?)?);
    }

    for proxy in main_repo.worktrees()? {
        let base = gix::path::realpath(proxy.base()?)?;

        match proxy.into_repo() {
            Ok(worktree_repo) => {
                worktrees.push(create_worktree_info(&worktree_repo, base)?);
            }
            Err(_) => {
                worktrees.push(create_inaccessible_worktree_info(&repo, base));
            }
        }
    }

    let path_width = worktrees
        .iter()
        .map(|worktree| UnicodeWidthStr::width(worktree.base.as_str()))
        .max()
        .unwrap_or(0);

    for worktree in worktrees {
        worktree.write(out, path_width)?;
    }

    Ok(())
}

struct WorktreeInfo {
    base: String,
    head: String,
    branch: String,
}

impl WorktreeInfo {
    fn write(&self, out: &mut dyn std::io::Write, path_width: usize) -> std::io::Result<()> {
        writeln!(
            out,
            "{}{} {} [{}]",
            self.base,
            " ".repeat(path_width.saturating_sub(UnicodeWidthStr::width(self.base.as_str()))),
            self.head,
            self.branch,
        )
    }
}

fn create_worktree_info(repo: &gix::Repository, base: std::path::PathBuf) -> anyhow::Result<WorktreeInfo> {
    let head = repo
        .head_id()
        .map_or_else(
            |_| repo.object_hash().null().attach(repo).shorten_or_id(),
            |id| id.shorten_or_id(),
        )
        .to_string();

    let branch = repo.head_name()?.map_or_else(
        || "<detached>".to_string(),
        |name| name.shorten().to_owned().to_string(),
    );

    Ok(WorktreeInfo {
        base: base.display().to_string(),
        head,
        branch,
    })
}

fn create_inaccessible_worktree_info(repo: &gix::Repository, base: std::path::PathBuf) -> WorktreeInfo {
    WorktreeInfo {
        base: base.display().to_string(),
        head: repo.object_hash().null().attach(repo).shorten_or_id().to_string(),
        branch: "<unknown>".to_string(),
    }
}

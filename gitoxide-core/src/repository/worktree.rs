use std::{path::Path, sync::atomic::AtomicBool};

use gix::{
    NestedProgress, Result,
    bstr::{BStr, BString, ByteSlice},
    error::{OptionExt, ResultExt, bail, message},
    prelude::ObjectIdExt,
};
use unicode_width::UnicodeWidthStr;

use crate::OutputFormat;

pub struct AddOptions {
    pub new_branch: Option<BString>,
    pub commit_ish: Option<BString>,
    pub detach: bool,
    pub format: OutputFormat,
}

pub fn add<P>(
    mut repo: gix::Repository,
    destination: &Path,
    out: &mut dyn std::io::Write,
    progress: P,
    should_interrupt: &AtomicBool,
    options: AddOptions,
) -> Result<()>
where
    P: NestedProgress,
    P::SubProgress: NestedProgress + 'static,
{
    use gix::{refs::transaction::PreviousValue, worktree::add::Head};

    if options.format != OutputFormat::Human {
        bail!(gix::error::unsupported("JSON output isn't implemented yet"));
    }
    repo.clear_namespace();
    let branch = if let Some(name) = &options.new_branch {
        Some(local_branch_name(name.as_bstr())?)
    } else if options.detach {
        None
    } else if let Some(spec) = &options.commit_ish {
        local_branch_name(spec.as_bstr()).ok()
    } else {
        Some(local_branch_name(gix::path::os_str_into_bstr(
            destination
                .file_name()
                .ok_or_raise(|| gix::error::validation("The destination needs a directory name"))?,
        )?)?)
    };
    let head = match branch {
        Some(name) if options.new_branch.is_none() && repo.try_find_reference(name.as_ref())?.is_some() => {
            Head::Attached(name)
        }
        branch => {
            let start_point = options
                .commit_ish
                .as_ref()
                .map_or(b"HEAD".as_bstr(), |spec| spec.as_bstr());
            let commit_id = repo.rev_parse_single(start_point)?.object()?.peel_to_commit()?.id;
            match branch.filter(|_| options.new_branch.is_some() || options.commit_ish.is_none()) {
                Some(name) => {
                    let mut reflog_message: BString = "branch: Created from ".into();
                    reflog_message.extend_from_slice(start_point);
                    // Git also retains this branch if subsequent worktree setup fails.
                    repo.reference(name.clone(), commit_id, PreviousValue::MustNotExist, reflog_message)?;
                    Head::Attached(name)
                }
                None => Head::Detached(commit_id),
            }
        }
    };
    let (created, outcome) = repo.add_worktree(destination, head, progress, should_interrupt)?;
    if let Some(error) = outcome.errors.into_iter().next() {
        return Err(error.error).or_raise(|| message!("Worktree checkout failed at {:?}", error.path));
    }
    if let Some(collision) = outcome.collisions.into_iter().next() {
        return Err(std::io::Error::from(collision.error_kind))
            .or_raise(|| message!("Worktree checkout collided at {:?}", collision.path).conflict());
    }
    if !outcome.delayed_paths_unprocessed.is_empty() || !outcome.delayed_paths_unknown.is_empty() {
        bail!(
            "Checkout filters left unprocessed paths {:?} and returned unexpected paths {:?}",
            outcome.delayed_paths_unprocessed,
            outcome.delayed_paths_unknown
        );
    }
    let info = create_worktree_info(
        &created,
        gix::path::realpath(
            created
                .workdir()
                .ok_or_raise(|| message("The new worktree has no checkout directory"))?,
        )?,
    )?;
    info.write(out, UnicodeWidthStr::width(info.base.as_str())).or_error()?;
    Ok(())
}

fn local_branch_name(name: &BStr) -> Result<gix::refs::FullName> {
    if name.starts_with(b"-") {
        bail!(gix::error::validation("Branch names must not start with '-'"));
    }
    let mut full_name: BString = "refs/heads/".into();
    full_name.extend_from_slice(name);
    gix::validate::reference::branch_name(full_name.as_bstr()).or_error()?;
    gix::refs::FullName::try_from(full_name).or_error()
}

pub fn remove<P>(repo: gix::Repository, worktree: &Path, force: u8, progress: P, format: OutputFormat) -> Result<()>
where
    P: NestedProgress,
    P::SubProgress: NestedProgress + 'static,
{
    use gix::worktree::remove::Force;

    if format != OutputFormat::Human {
        bail!(gix::error::unsupported("JSON output isn't implemented yet"));
    }
    repo.remove_worktree(
        worktree,
        match force {
            0 => Force::Never,
            1 => Force::DiscardChanges,
            _ => Force::OverrideLock,
        },
        progress,
    )?;
    Ok(())
}

pub fn list(repo: gix::Repository, out: &mut dyn std::io::Write, format: OutputFormat) -> Result<()> {
    if format != OutputFormat::Human {
        bail!(gix::error::unsupported("JSON output isn't implemented yet"));
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
        worktree.write(out, path_width).or_error()?;
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

fn create_worktree_info(repo: &gix::Repository, base: std::path::PathBuf) -> Result<WorktreeInfo> {
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

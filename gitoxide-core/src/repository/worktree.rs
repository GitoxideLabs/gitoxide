use anyhow::bail;

use crate::OutputFormat;

const HEAD_LENGTH: usize = 9;
const ZERO_HEAD: &str = "000000000";
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
                worktrees.push(create_inaccessible_worktree_info(base));
            }
        }
    }

    let path_width = worktrees.iter().map(|worktree| worktree.base.len()).max().unwrap_or(0);

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
            "{:<path_width$} {} [{}]",
            self.base,
            self.head,
            self.branch,
            path_width = path_width,
        )
    }
}

fn create_worktree_info(repo: &gix::Repository, base: std::path::PathBuf) -> anyhow::Result<WorktreeInfo> {
    let head = repo.head_id().map_or_else(
        |_| ZERO_HEAD.to_string(),
        |id| id.to_hex_with_len(HEAD_LENGTH).to_string(),
    );

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

fn create_inaccessible_worktree_info(base: std::path::PathBuf) -> WorktreeInfo {
    WorktreeInfo {
        base: base.display().to_string(),
        head: ZERO_HEAD.to_string(),
        branch: "<unknown>".to_string(),
    }
}

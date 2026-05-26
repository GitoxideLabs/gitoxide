use anyhow::bail;

use crate::OutputFormat;

const HEAD_LENGTH: usize = 9;
const ZERO_HEAD: &str = "000000000";

/// list: List all worktrees associated with the main repository.
///
/// This function collects information about the main worktree and any linked
/// worktrees, then writes them to the provided output stream in a human-readable
/// format.
///
/// # Parameters
///
/// - `repo`: The Git repository from which the worktrees are retrieved.
/// - `out`: The output stream where the worktree list is written.
/// - `format`: The output format to use. Currently, only `OutputFormat::Human`
///   is supported.
///
/// # Returns
///
/// Returns `Ok(())` if the worktrees are successfully listed.
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

/// WorktreeInfo
///
/// Stores display worktree information
struct WorktreeInfo {
    base: String,
    head: String,
    branch: String,
}

impl WorktreeInfo {
    /// write: Writes the worktree information to the given output stream.
    ///
    /// The output contains the worktree path, the shortened HEAD commit hash,
    /// and the current branch name.
    ///
    /// # Parameters
    ///
    /// - `out`: The output stream where the worktree information is written.
    /// - `path_width`: The width used to align the worktree paths.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the information is successfully written.
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

/// create_worktree_info: Creates display information for an accessible worktree.
///
/// This function reads the worktree HEAD and branch name from the given
/// repository. If the HEAD commit cannot be read, a zero hash is used instead.
/// If the repository is in a detached HEAD state, the branch name is displayed
/// as `<detached>`.
///
/// # Parameters
///
/// - `repo`: The repository associated with the worktree.
/// - `base`: The resolved base path of the worktree.
///
/// # Returns
///
/// Returns a `WorktreeInfo` value containing:
/// - the worktree base path,
/// - the shortened HEAD commit hash,
/// - the current branch name or `<detached>`.
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

/// create_inaccessible_worktree_info: Creates display information for
/// an inaccessible worktree.
///
/// This function is used when a linked worktree exists but its repository
/// cannot be opened. In that case, the HEAD is displayed as a zero hash and
/// the branch is displayed as `<unknown>`.
///
/// # Parameters
///
/// - `base`: The resolved base path of the inaccessible worktree.
///
/// # Returns
///
/// Returns a `WorktreeInfo` value containing:
/// - the worktree base path,
/// - a zero hash as the HEAD value,
/// - `<unknown>` as the branch name.
fn create_inaccessible_worktree_info(base: std::path::PathBuf) -> WorktreeInfo {
    WorktreeInfo {
        base: base.display().to_string(),
        head: ZERO_HEAD.to_string(),
        branch: "<unknown>".to_string(),
    }
}

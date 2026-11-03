use crate::{Result, bstr::BStr};

pub(super) fn query(repo: &crate::Repository, token: &BStr) -> Result<Vec<u8>> {
    let git_dir = gix_path::realpath_opts(repo.git_dir(), repo.current_dir(), gix_path::realpath::MAX_SYMLINKS)?;
    // shortcut: remote Git directories can use a different socket location; fall back to scanning until supported.
    gix_fsmonitor::client::daemon::query(
        &git_dir.join("fsmonitor--daemon.ipc"),
        token,
        || repo.command_context(),
        &Default::default(),
    )
}

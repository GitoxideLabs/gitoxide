use crate::state;

impl crate::Repository {
    /// Returns the status of an in progress operation on a repository or [`None`]
    /// if no operation is currently in progress.
    ///
    /// Note to be confused with the repositories 'status'.
    #[gix_macros::unstable(reason = "The exhaustive `state::InProgress` enum needs an extension policy.")]
    pub fn state(&self) -> Option<state::InProgress> {
        let git_dir = self.path();

        // This is modeled on the logic from wt_status_get_state in git's wt-status.c and
        // ps1 from gix-prompt.sh.

        if git_dir.join("rebase-apply/applying").is_file() {
            Some(state::InProgress::ApplyMailbox)
        } else if git_dir.join("rebase-apply/rebasing").is_file() {
            Some(state::InProgress::Rebase)
        } else if git_dir.join("rebase-apply").is_dir() {
            Some(state::InProgress::ApplyMailboxRebase)
        } else if git_dir.join("rebase-merge/interactive").is_file() {
            Some(state::InProgress::RebaseInteractive)
        } else if git_dir.join("rebase-merge").is_dir() {
            Some(state::InProgress::Rebase)
        } else if git_dir.join("CHERRY_PICK_HEAD").is_file() {
            if git_dir.join("sequencer/todo").is_file() {
                Some(state::InProgress::CherryPickSequence)
            } else {
                Some(state::InProgress::CherryPick)
            }
        } else if git_dir.join("MERGE_HEAD").is_file() {
            Some(state::InProgress::Merge)
        } else if git_dir.join("BISECT_LOG").is_file() {
            Some(state::InProgress::Bisect)
        } else if git_dir.join("REVERT_HEAD").is_file() {
            if git_dir.join("sequencer/todo").is_file() {
                Some(state::InProgress::RevertSequence)
            } else {
                Some(state::InProgress::Revert)
            }
        } else {
            None
        }
    }
}

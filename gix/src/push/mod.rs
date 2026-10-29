/// All possible values of `push.default`.
#[derive(Default, Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash, Debug)]
pub enum Default {
    /// Do not push anything unless a refspec is provided explicitly.
    ///
    /// This is for safety.
    Nothing,
    /// Push the current branch to update a remote branch with the same name.
    Current,
    /// Push the current branch to the branch it would fetch from and merge with,
    /// i.e. what is configured in `branch.<name>.merge`, retrievable with
    /// the `@{upstream}` refspec.
    Upstream,
    /// Push the current branch with the same name to the remote.
    /// This is the same as [`Current`](Default::Current), but fails if
    /// `branch.<name>.merge` is set to a branch that is named differently.
    #[default]
    Simple,
    /// Push *all* branches to their similarly named counterpart on the remote.
    Matching,
}

#[cfg(feature = "blocking-network-client")]
mod update;
#[cfg(feature = "blocking-network-client")]
pub use update::{Lease, Options, Outcome, Rejection, Status, Update, UpdateOutcome};
#[cfg(feature = "blocking-network-client")]
pub(crate) mod plan;

#[cfg(feature = "blocking-network-client")]
pub(crate) mod local;

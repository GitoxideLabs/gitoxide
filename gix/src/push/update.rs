use crate::{ObjectId, bstr::BString};

/// A condition on the remote value observed before pushing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lease {
    /// Accept the advertised value, subject to normal fast-forward and force checks.
    #[default]
    Any,
    /// Require this value; `None` requires that the reference does not exist.
    /// A matching lease allows replacing the value, even without a fast-forward.
    MustMatch(Option<ObjectId>),
    /// Derive the expected value from the first matching fetch refspec's local tracking reference.
    /// A missing mapping or tracking reference requires that the destination does not exist.
    Tracking {
        /// Also require the advertised commit to be reachable from the destination-named local
        /// reference's recent reflog, like `git push --force-with-lease --force-if-includes`.
        /// The source revision or object ID does not select this reflog. This check applies only
        /// when a tracking tip exists, and also applies to deletions if requested.
        force_if_includes: bool,
    },
}

/// An explicit update to a reference on a remote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Update {
    /// A fully qualified destination such as `refs/heads/main`.
    pub destination: gix_ref::FullName,
    /// The desired value, or `None` to delete the reference.
    pub new_id: Option<ObjectId>,
    /// The expected remote value. Even without a lease, the wire update compares
    /// against the advertisement, preventing races during transfer.
    pub lease: Lease,
    /// Permit non-fast-forward updates and tag replacement. This never bypasses a
    /// lease or allows storing non-commit objects in `refs/heads/`.
    pub force: bool,
}

/// Settings for an explicit push.
#[derive(Clone, Copy, Debug, Default)]
pub struct Options<'a> {
    /// Require all requested changes to succeed together. Local rejection prevents
    /// sending any change, and the server must support atomic transactions.
    pub atomic: bool,
    /// Validate against the advertisement without sending updates or objects.
    pub dry_run: bool,
    /// Options forwarded to the remote receive hooks, including empty strings.
    pub push_options: &'a [BString],
    /// Ask the server to suppress non-error progress output.
    pub quiet: bool,
}

/// Why one requested update was rejected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rejection {
    /// The advertised value differs from the lease.
    StaleLease,
    /// The tracking tip matches, but the destination-named local reflog does not include it.
    RemoteUpdated,
    /// Replacing an existing tag requires force or a matching lease.
    AlreadyExists,
    /// The previous remote object is not available locally to check its ancestry.
    FetchFirst,
    /// The old and new objects cannot both be peeled to commits.
    NeedsForce,
    /// The new commit is not a descendant of the previous commit.
    NonFastForward,
    /// Branches must point directly to commits, even when forced.
    NonCommitBranch,
    /// Another local rejection prevented this update in an atomic push.
    Atomic,
    /// The remote rejected the update, with the reason preserved verbatim.
    Server(BString),
}

/// The disposition of an explicit push update.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// The remote already has the desired value. This is checked before the lease.
    UpToDate,
    /// The update was accepted by the remote.
    Updated {
        /// The update needed force or a matching lease to override normal rules.
        forced: bool,
    },
    /// The update passed local checks in a dry run.
    WouldUpdate {
        /// The update needs force or a matching lease to override normal rules.
        forced: bool,
    },
    /// No update was made, for the given reason.
    Rejected(Rejection),
}

/// The result for one requested update, preserving input order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateOutcome {
    /// The original request.
    pub update: Update,
    /// The value advertised by the server, or `None` if absent.
    pub old_id: Option<ObjectId>,
    /// What happened to this update.
    pub status: Status,
}

/// The advertisement and result of an explicit push.
#[derive(Clone, Debug)]
pub struct Outcome {
    /// The receive-pack advertisement, including the original remote refs.
    pub handshake: gix_protocol::Handshake,
    /// One result per requested update, in input order.
    pub updates: Vec<UpdateOutcome>,
    /// The complete server report, including unpack failures and proc-receive
    /// metadata. Absent for dry runs or when no commands were sent.
    pub response: Option<gix_protocol::push::Response>,
}

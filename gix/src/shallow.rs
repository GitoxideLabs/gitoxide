pub(crate) type CommitsStorage =
    gix_features::threading::OwnShared<gix_fs::SharedFileSnapshotMut<nonempty::NonEmpty<gix_hash::ObjectId>>>;
/// A lazily loaded and auto-updated list of commits which are at the shallow boundary (behind which there are no commits available),
/// sorted to allow bisecting.
pub type Commits = gix_fs::SharedFileSnapshot<nonempty::NonEmpty<gix_hash::ObjectId>>;

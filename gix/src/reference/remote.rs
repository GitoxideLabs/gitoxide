use gix_ref::{Category, FullName};

use crate::{
    Reference,
    bstr::{BStr, ByteSlice},
    remote,
    repository::{branch_remote_ref_name, branch_remote_tracking_ref_name},
};

/// Remotes
impl<'repo> Reference<'repo> {
    /// Find the name of our remote for `direction` as configured in `branch.<name>.remote|pushRemote` respectively.
    /// Return `None` if no remote is configured.
    ///
    /// See also [`Repository::branch_remote_name()`](crate::Repository::branch_remote_name()) for more details.
    pub fn remote_name(&self, direction: remote::Direction) -> Option<remote::Name<'_>> {
        let (category, shortname) = self.name().category_and_short_name()?;
        match category {
            Category::RemoteBranch => {
                let mut remotes = None;
                remote_name_from_tracking_branch(shortname, |candidate| {
                    remotes
                        .get_or_insert_with(|| self.repo.remote_names())
                        .contains(candidate)
                })
                .and_then(|name| name.to_str().ok())
                .map(|name| remote::Name::Symbol(name.into()))
            }
            Category::LocalBranch => self.repo.branch_remote_name(shortname, direction),
            _ => None,
        }
    }

    /// Find the remote along with all configuration associated with it suitable for handling this reference.
    ///
    /// See also [`Repository::branch_remote()`](crate::Repository::branch_remote()) for more details.
    pub fn remote(
        &self,
        direction: remote::Direction,
    ) -> Option<Result<crate::Remote<'repo>, remote::find::existing::Error>> {
        self.repo.branch_remote(self.name().shorten(), direction)
    }

    /// Return the name of this reference on the remote side.
    ///
    /// See [`Repository::branch_remote_ref_name()`](crate::Repository::branch_remote_ref_name()) for details.
    #[doc(alias = "upstream", alias = "git2")]
    pub fn remote_ref_name(
        &self,
        direction: remote::Direction,
    ) -> Option<Result<FullName, branch_remote_ref_name::Error>> {
        self.repo.branch_remote_ref_name(self.name(), direction)
    }

    /// Return the name of the reference that tracks this reference on the remote side.
    ///
    /// See [`Repository::branch_remote_tracking_ref_name()`](crate::Repository::branch_remote_tracking_ref_name()) for details.
    #[doc(alias = "upstream", alias = "git2")]
    pub fn remote_tracking_ref_name(
        &self,
        direction: remote::Direction,
    ) -> Option<Result<FullName, branch_remote_tracking_ref_name::Error>> {
        self.repo.branch_remote_tracking_ref_name(self.name(), direction)
    }
}

/// Infer the remote name from a remote-tracking branch name without its `refs/remotes/` prefix.
///
/// A name with exactly one slash, like `origin/main`, is unambiguous and yields `origin` even if that remote is not
/// configured. With multiple slashes, both remote and branch names may contain slashes, so configured remote names are
/// considered from longest to shortest: for `team/origin/topic`, `team/origin` wins over `team` when both exist.
/// A name without a slash, or a multi-slash name without a matching configured remote prefix, yields `None`.
/// `is_configured_remote` is only called for multi-slash names, for which configuration is needed to disambiguate.
pub(crate) fn remote_name_from_tracking_branch(
    shortname: &BStr,
    mut is_configured_remote: impl FnMut(&BStr) -> bool,
) -> Option<&BStr> {
    if shortname.find_iter("/").take(2).count() == 1 {
        let slash_pos = shortname.find_byte(b'/').expect("it was just found");
        return Some(shortname[..slash_pos].as_bstr());
    }
    shortname
        .rfind_iter("/")
        .map(|slash_pos| shortname[..slash_pos].as_bstr())
        .find(|candidate| is_configured_remote(candidate))
}

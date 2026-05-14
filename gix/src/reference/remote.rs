use std::borrow::Cow;

use gix_ref::{Category, FullNameRef};

use crate::{
    Reference,
    bstr::ByteSlice,
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
                if shortname.find_iter("/").take(2).count() == 1 {
                    let slash_pos = shortname.find_byte(b'/').expect("it was just found");
                    shortname[..slash_pos]
                        .as_bstr()
                        .to_str()
                        .ok()
                        .map(|n| remote::Name::Symbol(n.into()))
                } else {
                    let remotes = self.repo.remote_names();
                    for slash_pos in shortname.rfind_iter("/") {
                        let candidate = shortname[..slash_pos].as_bstr();
                        if remotes.contains(candidate) {
                            return candidate.to_str().ok().map(|n| remote::Name::Symbol(n.into()));
                        }
                    }
                    None
                }
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
    ) -> Option<Result<Cow<'_, FullNameRef>, branch_remote_ref_name::Error>> {
        self.repo.branch_remote_ref_name(self.name(), direction)
    }

    /// Return the name of the reference that tracks this reference on the remote side.
    ///
    /// See [`Repository::branch_remote_tracking_ref_name()`](crate::Repository::branch_remote_tracking_ref_name()) for details.
    #[doc(alias = "upstream", alias = "git2")]
    pub fn remote_tracking_ref_name(
        &self,
        direction: remote::Direction,
    ) -> Option<Result<Cow<'_, FullNameRef>, branch_remote_tracking_ref_name::Error>> {
        self.repo.branch_remote_tracking_ref_name(self.name(), direction)
    }
}

use gix_hash::ObjectId;

use crate::{
    Head,
    bstr::{BString, ByteSlice},
};

impl<'repo> Head<'repo> {
    /// Return a platform for obtaining iterators on the reference log associated with the `HEAD` reference.
    pub fn log_iter(&self) -> gix_ref::file::log::iter::Platform<'static, 'repo> {
        gix_ref::file::log::iter::Platform {
            store: &self.repo.refs,
            name: "HEAD".try_into().expect("HEAD is always valid"),
            buf: Vec::new(),
        }
    }

    /// Return a list of all branch names that were previously checked out with the first-ever checked out branch
    /// being the first entry of the list, and the most recent is the last, along with the commit they were pointing to
    /// at the time.
    pub fn prior_checked_out_branches(&self) -> std::io::Result<Option<Vec<(BString, ObjectId)>>> {
        Ok(self.log_iter().all()?.map(|log| {
            log.filter_map(Result::ok)
                .filter_map(|line| {
                    line.message
                        .strip_prefix(b"checkout: moving from ")
                        .and_then(|from_to| from_to.find(" to ").map(|pos| &from_to[..pos]))
                        .map(|from_branch| (from_branch.as_bstr().to_owned(), line.previous_oid()))
                })
                .collect()
        }))
    }
}

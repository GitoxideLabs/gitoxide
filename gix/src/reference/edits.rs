///
pub mod set_target_id {
    use gix_macros::momo;
    use gix_ref::{transaction::PreviousValue, Target};

    use crate::{bstr::BString, Reference};

    mod error {
        use gix_ref::FullName;

        /// The error returned by [`Reference::set_target_id()`][super::Reference::set_target_id()].
        #[derive(Debug, thiserror::Error)]
        #[allow(missing_docs)]
        pub enum Error {
            #[error("Cannot change symbolic reference {name:?} into a direct one by setting it to an id")]
            SymbolicReference { name: FullName },
            #[error(transparent)]
            ReferenceEdit(#[from] crate::reference::edit::Error),
        }
    }
    pub use error::Error;

    impl Reference<'_> {
        /// Set the id of this direct reference to `id` and use `reflog_message` for the reflog (if enabled in the repository).
        ///
        /// Note that the operation will fail on symbolic references, to change their type use the lower level reference database,
        /// or if the reference was deleted or changed in the mean time.
        /// Furthermore, refrain from using this method for more than a one-off change as it creates a transaction for each invocation.
        /// If multiple reference should be changed, use [`Repository::edit_references()`][crate::Repository::edit_references()]
        /// or the lower level reference database instead.
        #[allow(clippy::result_large_err)]
        #[momo]
        pub fn set_target_id(
            &mut self,
            id: impl Into<gix_hash::ObjectId>,
            reflog_message: impl Into<BString>,
        ) -> Result<(), Error> {
            match &self.inner.target {
                Target::Symbolic(name) => return Err(Error::SymbolicReference { name: name.clone() }),
                Target::Object(current_id) => {
                    let changed = self.repo.reference(
                        self.name(),
                        id,
                        PreviousValue::MustExistAndMatch(Target::Object(current_id.to_owned())),
                        reflog_message,
                    )?;
                    *self = changed;
                }
            }
            Ok(())
        }
    }
}

///
pub mod delete {
    use gix_ref::transaction::{Change, PreviousValue, RefEdit, RefLog};

    use crate::Reference;

    impl Reference<'_> {
        /// Delete this reference or fail if it was changed since last observed.
        /// Note that this instance remains available in memory but probably shouldn't be used anymore.
        pub fn delete(&self) -> Result<(), crate::reference::edit::Error> {
            self.repo
                .edit_reference(RefEdit {
                    change: Change::Delete {
                        expected: PreviousValue::MustExistAndMatch(self.inner.target.clone()),
                        log: RefLog::AndReference,
                    },
                    name: self.inner.name.clone(),
                    deref: false,
                })
                .map(|_| ())
        }
    }
}

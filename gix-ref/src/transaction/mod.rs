//! **Transactions** are the only way make changes to the ref store in order to increase the chance of consistency in a multi-threaded
//! environment.
//!
//! Transactions currently allow to…
//!
//! * create or update reference
//! * delete references
//!
//! The following guarantees are made:
//!
//! * transactions are prepared which is when other writers are prevented from changing them
//!   - errors during preparations will cause a perfect rollback
//! * prepared transactions are committed to finalize the change
//!   - errors when committing while leave the ref store in an inconsistent, but operational state.
use gix_object::bstr::BString;

use crate::{FullName, Target};

/// A change to the reflog.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct LogChange {
    /// How to treat the reference log.
    pub mode: RefLog,
    /// If set, create a reflog even though it would otherwise not be the case as prohibited by general rules.
    /// Note that ref-log writing might be prohibited in the entire repository which is when this flag has no effect either.
    pub force_create_reflog: bool,
    /// The message to put into the reference log. It must be a single line, hence newlines are forbidden.
    /// The string can be empty to indicate there should be no message at all.
    pub message: BString,
}

impl Default for LogChange {
    fn default() -> Self {
        LogChange {
            mode: RefLog::AndReference,
            force_create_reflog: false,
            message: Default::default(),
        }
    }
}

/// The desired value of an updated value
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub enum PreviousValue {
    /// No requirements are made towards the current value, and the new value is set unconditionally.
    Any,
    /// The reference must exist and may have any value.
    MustExist,
    /// Create the ref only, hence the reference must not exist.
    MustNotExist,
    /// The ref _must_ exist and have the given value.
    MustExistAndMatch(Target),
    /// The ref _may_ exist and have the given value, or may not exist at all.
    ExistingMustMatch(Target),
}

/// A description of an edit to perform.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub enum Change {
    /// If previous is not `None`, the ref must exist and its `oid` must agree with the `previous`, and
    /// we function like `update`.
    /// Otherwise it functions as `create-or-update`.
    Update {
        /// The desired change to the reference log.
        log: LogChange,
        /// The expected value already present in the reference.
        /// If a ref was existing previously this field will be overwritten with `MustExistAndMatch(actual_value)` for use after
        /// the transaction was committed successfully.
        expected: PreviousValue,
        /// The new state of the reference, either for updating an existing one or creating a new one.
        new: Target,
    },
    /// Delete a reference and optionally check if `previous` is its content.
    Delete {
        /// The expected value of the reference, with the `MustNotExist` variant being invalid.
        ///
        /// If a previous ref existed, this value will be filled in automatically as `MustExistAndMatch(actual_value)` and
        /// can be accessed if the transaction was committed successfully.
        expected: PreviousValue,
        /// How to treat the reference log during deletion.
        log: RefLog,
    },
}

impl Change {
    /// Return references to values that are the new value after the change is applied, if this is an update.
    pub fn new_value(&self) -> Option<crate::TargetRef<'_>> {
        match self {
            Change::Update { new, .. } => new.to_ref().into(),
            Change::Delete { .. } => None,
        }
    }

    /// Return references to values that are in common between all variants and denote the previous observed value.
    pub fn previous_value(&self) -> Option<crate::TargetRef<'_>> {
        match self {
            Change::Update {
                expected: PreviousValue::MustExistAndMatch(previous) | PreviousValue::ExistingMustMatch(previous),
                ..
            }
            | Change::Delete {
                expected: PreviousValue::MustExistAndMatch(previous) | PreviousValue::ExistingMustMatch(previous),
                ..
            } => previous,
            _ => return None,
        }
        .to_ref()
        .into()
    }
}

/// A reference that is to be changed
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct RefEdit {
    /// The change itself
    pub change: Change,
    /// The name of the reference to apply the change to
    pub name: FullName,
    /// If set, symbolic references  identified by `name`  will be dereferenced to have the `change` applied to their target.
    /// This flag has no effect if the reference isn't symbolic.
    pub deref: bool,
}

/// Lifecycle
impl RefEdit {
    /// Create an edit that applies `change` to `name` without dereferencing symbolic references.
    pub fn new(name: FullName, change: Change) -> Self {
        RefEdit {
            change,
            name,
            deref: false,
        }
    }

    /// Create an update that sets `name` to `new` if its current state satisfies `expected`, recording
    /// `reflog_message` with standard reference-log handling and without dereferencing symbolic references.
    pub fn update(
        name: FullName,
        new: impl Into<Target>,
        expected: PreviousValue,
        reflog_message: impl Into<BString>,
    ) -> Self {
        RefEdit::update_with_log(
            name,
            new,
            expected,
            LogChange {
                message: reflog_message.into(),
                ..Default::default()
            },
        )
    }

    /// Create an update that sets `name` to `new` if its current state satisfies `expected`, using `log` to configure
    /// reference-log handling and without dereferencing symbolic references.
    pub fn update_with_log(name: FullName, new: impl Into<Target>, expected: PreviousValue, log: LogChange) -> Self {
        RefEdit::new(
            name,
            Change::Update {
                log,
                expected,
                new: new.into(),
            },
        )
    }

    /// Create a deletion of `name` and its reference log if its current state satisfies `expected`, without
    /// dereferencing symbolic references.
    pub fn delete(name: FullName, expected: PreviousValue) -> Self {
        RefEdit::delete_with_log(name, expected, RefLog::AndReference)
    }

    /// Create a deletion of `name` if its current state satisfies `expected`, using `log` to configure reference-log
    /// handling and without dereferencing symbolic references.
    pub fn delete_with_log(name: FullName, expected: PreviousValue, log: RefLog) -> Self {
        RefEdit::new(name, Change::Delete { expected, log })
    }
}

/// Builders
impl RefEdit {
    /// Set whether symbolic references are dereferenced before applying the edit.
    pub fn with_deref(mut self, deref: bool) -> Self {
        self.deref = deref;
        self
    }
}

/// The way to deal with the Reflog in deletions.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
pub enum RefLog {
    /// Delete or update the reference and the log
    AndReference,
    /// Delete or update only the reflog
    Only,
}

mod ext;
pub use ext::RefEditsExt;

//! Compact loose references into the shared packed-refs file.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::{
    Repository, Result,
    error::{ResultExt, bail, message},
};
use gix_ref::{
    Target,
    transaction::{PreviousValue, RefEdit},
};

/// Settings for [`Repository::pack_references()`].
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Pack every eligible shared reference, instead of only tags.
    pub all: bool,
    /// Remove loose copies after publishing their packed values. Defaults to true.
    pub prune: bool,
    /// Validate and lock the transaction without publishing it.
    pub dry_run: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            all: false,
            prune: true,
            dry_run: false,
        }
    }
}

/// References selected for packing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// Number of loose references written to packed-refs, or selected by a dry run.
    pub packed: usize,
}

impl Repository {
    /// Pack loose shared references without changing their values or reflogs.
    ///
    /// Symbolic references, per-worktree references, and direct references to missing
    /// objects are left loose. Tags are peeled into the packed file. Existing packed
    /// entries are preserved and overridden by selected loose entries when necessary.
    /// Maintenance covers the main repository regardless of the calling worktree or
    /// reference namespace, and ignores object replacements.
    ///
    /// Compare-and-swap checks under reference locks reject changed loose values.
    /// Publish packed-refs before pruning loose files and hold their locks through
    /// removal. A failure can leave redundant loose copies, but does not change any
    /// reference target. No reflog entry is added or removed.
    pub fn pack_references(&self, options: Options, interrupt: &AtomicBool) -> Result<Outcome> {
        let mut repo = self.main_repo()?;
        repo.clear_namespace();
        repo.objects.ignore_replacements = true;
        let mut edits = Vec::new();
        for reference in repo.refs.loose_iter().or_error()? {
            check_interrupt(interrupt)?;
            let reference = reference?;
            if reference
                .name
                .category()
                .is_some_and(|category| category.is_worktree_private())
                || (!options.all && !reference.name.as_bstr().starts_with(b"refs/tags/"))
            {
                continue;
            }
            let Target::Object(object_id) = reference.target else {
                continue;
            };
            if repo.try_find_object(object_id)?.is_none() {
                continue;
            }
            edits.push(RefEdit::update(
                reference.name,
                Target::Object(object_id),
                PreviousValue::MustExistAndMatch(Target::Object(object_id)),
                "",
            ));
        }
        check_interrupt(interrupt)?;
        let outcome = Outcome { packed: edits.len() };
        if edits.is_empty() {
            return Ok(outcome);
        }
        use gix_ref::file::transaction::PackedRefs;
        let transaction = repo
            .refs
            .transaction()
            .packed_refs(if options.prune {
                PackedRefs::DeletionsAndNonSymbolicUpdatesRemoveLooseSourceReference(Box::new(&repo))
            } else {
                PackedRefs::DeletionsAndNonSymbolicUpdates(Box::new(&repo))
            })
            .prepare(
                edits,
                gix_lock::acquire::Fail::Immediately,
                gix_lock::acquire::Fail::Immediately,
            )?;
        check_interrupt(interrupt)?;
        if !options.dry_run {
            transaction.commit(None)?;
            // Refresh the original handle too, including clones that use a different store snapshot.
            self.refs.force_refresh_packed_buffer()?;
        }
        Ok(outcome)
    }
}

fn check_interrupt(interrupt: &AtomicBool) -> Result<()> {
    if interrupt.load(Ordering::Relaxed) {
        bail!(message("Reference packing interrupted").cancelled());
    }
    Ok(())
}

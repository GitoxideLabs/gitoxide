use std::collections::BTreeSet;

use gix_error::{ResultExt, bail, message, validation};
use gix_ref::transaction::{PreviousValue, RefEdit};

use crate::{Result, bstr::ByteSlice, remote::fetch::RefMap};

/// Control pruning of local references according to an advertised fetch mapping.
#[derive(Default, Debug, Clone, Copy)]
pub struct Options {
    /// Report deletions without changing references or taking reference locks.
    pub dry_run: bool,
}

impl crate::Repository {
    /// Delete stale local references covered by the explicit refspecs in `ref_map`.
    ///
    /// Reverse-map local reference names through the fetch refspecs, including negative exclusions.
    /// A reference is stale when it matches a destination but no explicit remote mapping still supplies
    /// that destination. This preserves destinations shared by overlapping refspecs and handles source
    /// names that Git expands, such as `main`. Symbolic references are retained, including dangling ones.
    ///
    /// Like `git remote prune`, implicit tag mappings do not prune local tags. An explicit tag refspec
    /// does. Checked-out branches are protected even if an unusual fetch refspec covers them.
    ///
    /// The caller must supply a complete, current advertisement for the explicit refspecs in `ref_map`.
    /// Only references visible through this repository's namespace are considered. Deletions check
    /// each reference's observed target in one transaction, so a preparation failure leaves them intact.
    /// Return the performed edits, or the proposed edits when [`Options::dry_run`] is enabled.
    pub fn prune_fetch_refs(&self, ref_map: &RefMap, options: Options) -> Result<Vec<RefEdit>> {
        let mut repo = self.clone();
        repo.make_paths_absolute()?;
        let mut references = repo.references()?.all()?.collect::<Result<Vec<_>>>()?;
        references.retain(|reference| reference.target().try_id().is_some());
        let supplied_destinations: BTreeSet<_> = ref_map
            .mappings
            .iter()
            .filter(|mapping| {
                matches!(
                    mapping.spec_index,
                    crate::remote::fetch::refmap::SpecIndex::ExplicitInRemote(_)
                )
            })
            .filter_map(|mapping| mapping.local.as_ref().map(|name| name.as_bstr()))
            .collect();
        let reverse =
            gix_refspec::MatchGroup::from_fetch_specs(ref_map.refspecs.iter().map(gix_refspec::RefSpec::to_ref))
                .match_rhs(references.iter().map(|reference| gix_refspec::match_group::Item {
                    full_ref_name: reference.name().as_bstr(),
                    target: reference.inner.target.id(),
                    object: None,
                }));
        let stale_indices: BTreeSet<_> = reverse
            .mappings
            .iter()
            .filter(|mapping| {
                !mapping
                    .rhs
                    .as_deref()
                    .is_some_and(|destination| supplied_destinations.contains(destination))
            })
            .filter_map(|mapping| mapping.item_index)
            .collect();
        let checked_out = repo.checked_out_branches()?;
        let mut edits = Vec::with_capacity(stale_indices.len());
        for index in stale_indices {
            let reference = &references[index];
            if let Some(worktrees) = checked_out.get(reference.name()) {
                bail!(validation(format!(
                    "Cannot prune checked-out branch {} used by {worktrees:?}",
                    reference.name().as_bstr()
                )));
            }
            edits.push(RefEdit::delete(
                reference.name().to_owned(),
                PreviousValue::MustExistAndMatch(reference.target().into_owned()),
            ));
        }
        if options.dry_run || edits.is_empty() {
            return Ok(edits);
        }
        repo.edit_references_as(edits, None)
            .or_raise(|| message("Could not prune stale fetch references"))
    }
}

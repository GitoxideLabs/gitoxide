use std::cmp::Ordering;

use crate::State;

impl State {
    /// Assure our entries are consistent.
    pub fn verify_entries(&self) -> Result<(), gix_error::Exn<gix_error::CorruptionError>> {
        use gix_error::ErrorExt;

        let _span = gix_features::trace::coarse!("gix_index::File::verify_entries()");
        let mut previous = None::<&crate::Entry>;
        for (idx, entry) in self.entries.iter().enumerate() {
            if let Some(prev) = previous
                && prev.cmp(entry, self) != Ordering::Less
            {
                return Err(gix_error::CorruptionError::new(format!(
                    "Entry '{}' (stage = {}) at index {idx} should order after prior entry '{}' (stage = {})",
                    entry.path(self),
                    entry.flags.stage() as u8,
                    prev.path(self),
                    prev.flags.stage() as u8
                ))
                .raise());
            }
            previous = Some(entry);
        }
        Ok(())
    }

    /// Note: `objects` cannot be `Option<F>` as we can't call it with a closure then due to the indirection through `Some`.
    pub fn verify_extensions(
        &self,
        use_find: bool,
        objects: impl gix_object::Find,
    ) -> Result<(), gix_error::Exn<gix_error::CorruptionError>> {
        if let Some(tree) = self.tree() {
            tree.verify(use_find, objects)?;
            tree.verify_entries_count(self.entries.len())?;
        }
        // TODO: verify links by running the whole set of tests on the index
        //       - do that once we load it as well, or maybe that's lazy loaded? Too many questions for now.
        Ok(())
    }
}

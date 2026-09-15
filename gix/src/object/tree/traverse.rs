use crate::Tree;

/// Traversal
impl<'repo> Tree<'repo> {
    /// Obtain a platform for initiating a variety of traversals.
    pub fn traverse(&self) -> Platform<'_, 'repo> {
        Platform {
            root: self,
            breadthfirst: BreadthFirstPresets { root: self },
        }
    }
}

/// An intermediate object to start traversing the parent tree from.
pub struct Platform<'a, 'repo> {
    root: &'a Tree<'repo>,
    /// Provides easy access to presets for common breadth-first traversal.
    // TODO: remove this - it's a bit too much of a fixed function, or go all in once it's clear it's needed,
    //       but probably with depth-first.
    pub breadthfirst: BreadthFirstPresets<'a, 'repo>,
}

/// Presets for common choices in breadth-first traversal.
#[derive(Copy, Clone)]
pub struct BreadthFirstPresets<'a, 'repo> {
    root: &'a Tree<'repo>,
}

impl BreadthFirstPresets<'_, '_> {
    /// Returns all entries and their file paths, recursively, as reachable from this tree.
    pub fn files(&self) -> Result<Vec<gix_traverse::tree::recorder::Entry>, crate::Error> {
        let mut recorder = gix_traverse::tree::Recorder::default();
        Platform {
            root: self.root,
            breadthfirst: *self,
        }
        .breadthfirst(&mut recorder)?;
        Ok(recorder.records)
    }
}

impl Platform<'_, '_> {
    /// Start a breadth-first, recursive traversal using `delegate`, for which a [`Recorder`](gix_traverse::tree::Recorder) can be used to get started.
    ///
    /// # Note
    ///
    /// - Results are returned in sort order as per tree-sorting rules, files first, then directories, one level at a time.
    /// - for obtaining the direct children of the tree, use [Tree::iter()] instead.
    pub fn breadthfirst<V>(&self, delegate: &mut V) -> Result<(), crate::Error>
    where
        V: gix_traverse::tree::Visit,
    {
        let root = gix_object::TreeRefIter::from_bytes(&self.root.data, self.root.id.kind());
        let state = gix_traverse::tree::breadthfirst::State::default();
        gix_traverse::tree::breadthfirst(root, state, &self.root.repo.objects, delegate)
            .map_err(gix_error::Exn::into_error)
    }

    /// Start a depth-first, recursive traversal using `delegate`, for which a [`Recorder`](gix_traverse::tree::Recorder) can be used to get started.
    ///
    /// # Note
    ///
    /// For obtaining the direct children of the tree, use [Tree::iter()] instead.
    pub fn depthfirst<V>(&self, delegate: &mut V) -> Result<(), crate::Error>
    where
        V: gix_traverse::tree::Visit,
    {
        let state = gix_traverse::tree::depthfirst::State::default();
        gix_traverse::tree::depthfirst(self.root.id, state, &self.root.repo.objects, delegate)
            .map_err(gix_error::Exn::into_error)
    }
}

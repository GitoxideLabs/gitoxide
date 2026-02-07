use crate::{file, file::Commit, File, Graph, Position};

/// Access
impl Graph {
    /// Returns the commit at the given position `pos`.
    ///
    /// # Panics
    /// If `pos` is greater or equal to [`num_commits()`][Graph::num_commits()].
    pub fn commit_at(&self, pos: Position) -> Commit<'_> {
        let r = self.lookup_by_pos(pos);
        r.file.commit_at(r.pos)
    }

    /// The kind of hash used in this `Graph`.
    ///
    /// Note that it is always conforming to the hash used in the owning repository.
    ///
    /// # Panics
    ///
    /// If the graph does not contain any `File`.
    pub fn object_hash(&self) -> gix_hash::Kind {
        self.files
            .first()
            .map(super::File::object_hash)
            .expect("graph to have at least one file")
    }

    /// Returns the commit matching the given `id`.
    pub fn commit_by_id(&self, id: impl AsRef<gix_hash::oid>) -> Option<Commit<'_>> {
        let r = self.lookup_by_id(id.as_ref())?;
        Some(r.file.commit_at(r.file_pos))
    }

    /// Returns the `hash` at the given position `pos`.
    ///
    /// # Panics
    /// If `pos` is greater or equal to [`num_commits()`][Graph::num_commits()].
    pub fn id_at(&self, pos: Position) -> &gix_hash::oid {
        let r = self.lookup_by_pos(pos);
        r.file.id_at(r.pos)
    }

    /// Iterate over commits in unsorted order.
    pub fn iter_commits(&self) -> impl Iterator<Item = Commit<'_>> {
        self.files.iter().flat_map(File::iter_commits)
    }

    /// Iterate over commit IDs in unsorted order.
    pub fn iter_ids(&self) -> impl Iterator<Item = &gix_hash::oid> {
        self.files.iter().flat_map(File::iter_ids)
    }

    /// Translate the given `id` to its position in the file.
    pub fn lookup(&self, id: impl AsRef<gix_hash::oid>) -> Option<Position> {
        Some(self.lookup_by_id(id.as_ref())?.graph_pos)
    }

    /// Returns the number of commits stored in this file.
    pub fn num_commits(&self) -> u32 {
        self.files.iter().map(File::num_commits).sum()
    }
}

/// Access fundamentals
impl Graph {
    fn lookup_by_id(&self, id: &gix_hash::oid) -> Option<LookupByIdResult<'_>> {
        let mut current_file_start = 0;
        for file in &self.files {
            if let Some(lex_pos) = file.lookup(id) {
                return Some(LookupByIdResult {
                    file,
                    file_pos: lex_pos,
                    graph_pos: Position(current_file_start + lex_pos.0),
                });
            }
            current_file_start += file.num_commits();
        }
        None
    }

    fn lookup_by_pos(&self, pos: Position) -> LookupByPositionResult<'_> {
        let mut remaining = pos.0;
        for (file_index, file) in self.files.iter().enumerate() {
            match remaining.checked_sub(file.num_commits()) {
                Some(v) => remaining = v,
                None => {
                    return LookupByPositionResult {
                        file,
                        _file_index: file_index,
                        pos: file::Position(remaining),
                    }
                }
            }
        }
        panic!("graph position too large: {}", pos.0);
    }
}

#[derive(Clone)]
struct LookupByIdResult<'a> {
    pub file: &'a File,
    pub graph_pos: Position,
    pub file_pos: file::Position,
}

#[derive(Clone)]
struct LookupByPositionResult<'a> {
    pub file: &'a File,
    pub _file_index: usize,
    pub pos: file::Position,
}

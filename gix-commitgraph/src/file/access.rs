use std::{
    fmt::{Debug, Formatter},
    path::Path,
};

use crate::{
    File,
    file::{self, COMMIT_DATA_ENTRY_SIZE_SANS_HASH, commit::Commit},
};

/// Access
impl File {
    /// The number of base graphs that this file depends on.
    pub fn base_graph_count(&self) -> u8 {
        self.base_graph_count
    }

    /// Returns the commit data for the commit located at the given lexicographical position.
    ///
    /// `pos` must range from 0 to `self.num_commits()`.
    ///
    /// # Panics
    ///
    /// Panics if `pos` is out of bounds.
    pub fn commit_at(&self, pos: file::Position) -> Commit<'_> {
        Commit::new(self, pos)
    }

    /// The kind of hash used in this File.
    ///
    /// Note that it is always conforming to the hash used in the owning repository.
    pub fn object_hash(&self) -> gix_hash::Kind {
        self.object_hash
    }

    /// Returns an object id at the given index in our list of (sorted) hashes.
    /// The position ranges from 0 to `self.num_commits()`
    // copied from gix-odb/src/pack/index/ext
    pub fn id_at(&self, pos: file::Position) -> &gix_hash::oid {
        assert!(
            pos.0 < self.num_commits(),
            "expected lexicographical position less than {}, got {}",
            self.num_commits(),
            pos.0
        );
        let pos: usize = pos
            .0
            .try_into()
            .expect("an architecture able to hold 32 bits of integer");
        let start = self.oid_lookup_offset + (pos * self.hash_len);
        gix_hash::oid::from_bytes_unchecked(&self.data[start..][..self.hash_len])
    }

    /// Return an iterator over all object hashes stored in the base graph.
    pub fn iter_base_graph_ids(&self) -> impl Iterator<Item = &gix_hash::oid> {
        let start = self.base_graphs_list_offset.unwrap_or(0);
        let base_graphs_list = &self.data[start..][..self.hash_len * usize::from(self.base_graph_count)];
        base_graphs_list
            .chunks_exact(self.hash_len)
            .map(gix_hash::oid::from_bytes_unchecked)
    }

    /// return an iterator over all commits in this file.
    pub fn iter_commits(&self) -> impl Iterator<Item = Commit<'_>> {
        (0..self.num_commits()).map(move |i| self.commit_at(file::Position(i)))
    }

    /// Return an iterator over all object hashes stored in this file.
    pub fn iter_ids(&self) -> impl Iterator<Item = &gix_hash::oid> {
        (0..self.num_commits()).map(move |i| self.id_at(file::Position(i)))
    }

    /// Translate the given object hash to its position within this file, if present.
    // copied from gix-odb/src/pack/index/ext
    pub fn lookup(&self, id: impl AsRef<gix_hash::oid>) -> Option<file::Position> {
        self.lookup_inner(id.as_ref())
    }

    fn lookup_inner(&self, id: &gix_hash::oid) -> Option<file::Position> {
        let first_byte = usize::from(id.first_byte());
        let mut upper_bound = self.fan[first_byte];
        let mut lower_bound = if first_byte != 0 { self.fan[first_byte - 1] } else { 0 };

        while lower_bound < upper_bound {
            let mid = u32::midpoint(lower_bound, upper_bound);
            let mid_sha = self.id_at(file::Position(mid));

            use std::cmp::Ordering::*;
            match id.cmp(mid_sha) {
                Less => upper_bound = mid,
                Equal => return Some(file::Position(mid)),
                Greater => lower_bound = mid + 1,
            }
        }
        None
    }

    /// Returns the number of commits in this graph file.
    ///
    /// The maximum valid `file::Position` that can be used with this file is one less than
    /// `num_commits()`.
    pub fn num_commits(&self) -> u32 {
        self.fan[255]
    }

    /// Returns the path to this file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl File {
    /// Returns the byte slice for the given commit in this file's Commit Data (CDAT) chunk.
    pub(crate) fn commit_data_bytes(&self, pos: file::Position) -> &[u8] {
        assert!(
            pos.0 < self.num_commits(),
            "expected lexicographical position less than {}, got {}",
            self.num_commits(),
            pos.0
        );
        let pos: usize = pos
            .0
            .try_into()
            .expect("an architecture able to hold 32 bits of integer");
        let entry_size = self.hash_len + COMMIT_DATA_ENTRY_SIZE_SANS_HASH;
        let start = self.commit_data_offset + (pos * entry_size);
        &self.data[start..][..entry_size]
    }

    /// Returns the byte slice for this file's entire Extra Edge List (EDGE) chunk.
    pub(crate) fn extra_edges_data(&self) -> Option<&[u8]> {
        Some(&self.data[self.extra_edges_list_range.clone()?])
    }
}

impl Debug for File {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, r#"File("{:?}")"#, self.path.display())
    }
}

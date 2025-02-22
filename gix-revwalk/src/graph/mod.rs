use std::{fmt::Formatter, ops::Index};

use gix_hash::oid;
use smallvec::SmallVec;

use crate::Graph;

/// A mapping between an object id and arbitrary data, and produced when calling [`Graph::detach()`].
pub type IdMap<T> = gix_hashtable::HashMap<gix_hash::ObjectId, T>;

///
pub mod commit;

mod errors {
    ///
    pub mod insert_parents {
        use crate::graph::commit::iter_parents;

        /// The error returned by [`insert_parents()`](crate::Graph::insert_parents()).
        #[derive(Debug, thiserror::Error)]
        #[allow(missing_docs)]
        pub enum Error {
            #[error(transparent)]
            Lookup(#[from] gix_object::find::existing_iter::Error),
            #[error("A commit could not be decoded during traversal")]
            Decode(#[from] gix_object::decode::Error),
            #[error(transparent)]
            Parent(#[from] iter_parents::Error),
        }
    }

    ///
    pub mod get_or_insert_default {
        use crate::graph::commit::to_owned;

        /// The error returned by [`try_lookup_or_insert_default()`](crate::Graph::try_lookup_or_insert_default()).
        #[derive(Debug, thiserror::Error)]
        #[allow(missing_docs)]
        pub enum Error {
            #[error(transparent)]
            Lookup(#[from] gix_object::find::existing_iter::Error),
            #[error(transparent)]
            ToOwned(#[from] to_owned::Error),
        }
    }
}
pub use errors::{get_or_insert_default, insert_parents};
use gix_date::SecondsSinceUnixEpoch;

/// The generation away from the HEAD of graph, useful to limit algorithms by topological depth as well.
///
/// 0 would mean the starting point of the hierarchy, and 1 their parents.
/// This number is only available natively if there is a commit-graph.
pub type Generation = u32;

impl<T: std::fmt::Debug> std::fmt::Debug for Graph<'_, '_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.map, f)
    }
}

impl<'cache, T: Default> Graph<'_, 'cache, T> {
    /// Lookup `id` without failing if the commit doesn't exist, and assure that `id` is inserted into our set.
    /// If it wasn't, associate it with the default value. Assure `update_data(data)` gets run.
    /// Return the commit when done.
    /// Note that none of the data updates happen if there was no commit named `id`.
    pub fn try_lookup_or_insert(
        &mut self,
        id: gix_hash::ObjectId,
        update_data: impl FnOnce(&mut T),
    ) -> Result<Option<LazyCommit<'_, 'cache>>, get_or_insert_default::Error> {
        self.try_lookup_or_insert_default(id, T::default, update_data)
    }
}

/// Access and mutation
impl<'cache, T> Graph<'_, 'cache, T> {
    /// Returns the amount of entries in the graph.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns `true` if there is no entry in the graph.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Returns true if `id` has data associated with it, meaning that we processed it already.
    pub fn contains(&self, id: &gix_hash::oid) -> bool {
        self.map.contains_key(id.as_ref())
    }

    /// Returns the data associated with `id` if available.
    pub fn get(&self, id: &gix_hash::oid) -> Option<&T> {
        self.map.get(id)
    }

    /// Returns the data associated with `id` if available as mutable reference.
    pub fn get_mut(&mut self, id: &gix_hash::oid) -> Option<&mut T> {
        self.map.get_mut(id)
    }

    /// Insert `id` into the graph and associate it with `value`, returning the previous value associated with `id` if it existed.
    pub fn insert(&mut self, id: gix_hash::ObjectId, value: T) -> Option<T> {
        self.map.insert(id, value)
    }

    /// Insert `id` into the graph and associate it with the value returned by `make_data`,
    /// and returning the previous value associated with `id` if it existed.
    /// Fail if `id` doesn't exist in the object database.
    pub fn insert_data<E>(
        &mut self,
        id: gix_hash::ObjectId,
        mut make_data: impl FnMut(LazyCommit<'_, 'cache>) -> Result<T, E>,
    ) -> Result<Option<T>, E>
    where
        E: From<gix_object::find::existing_iter::Error>,
    {
        let value = make_data(self.lookup(&id).map_err(E::from)?)?;
        Ok(self.map.insert(id, value))
    }

    /// Remove all data from the graph to start over.
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Insert the parents of commit named `id` to the graph and associate new parents with data
    /// by calling `new_parent_data(parent_id, committer_timestamp)`, or update existing parents
    /// data with `update_existing(parent_id, &mut existing_data)`.
    /// If `first_parent` is `true`, only the first parent of commits will be looked at.
    pub fn insert_parents(
        &mut self,
        id: &gix_hash::oid,
        new_parent_data: &mut dyn FnMut(gix_hash::ObjectId, SecondsSinceUnixEpoch) -> T,
        update_existing: &mut dyn FnMut(gix_hash::ObjectId, &mut T),
        first_parent: bool,
    ) -> Result<(), insert_parents::Error> {
        let commit = self.lookup(id)?;
        let parents: SmallVec<[_; 2]> = commit.iter_parents().collect();
        for parent_id in parents {
            let parent_id = parent_id?;
            match self.map.entry(parent_id) {
                gix_hashtable::hash_map::Entry::Vacant(entry) => {
                    let parent = match try_lookup(&parent_id, &*self.find, self.cache, &mut self.parent_buf)? {
                        Some(p) => p,
                        None => continue, // skip missing objects, this is due to shallow clones for instance.
                    };

                    let parent_commit_date = parent.committer_timestamp().unwrap_or_default();
                    entry.insert(new_parent_data(parent_id, parent_commit_date));
                }
                gix_hashtable::hash_map::Entry::Occupied(mut entry) => {
                    update_existing(parent_id, entry.get_mut());
                }
            }
            if first_parent {
                break;
            }
        }
        Ok(())
    }

    /// Insert the parents of commit named `id` to the graph and associate new parents with data
    /// as produced by `parent_data(parent_id, parent_info, maybe-existing-data &mut T) -> T`, which is always
    /// provided the full parent commit information.
    /// It will be provided either existing data, along with complete information about the parent,
    /// and produces new data even though it's only used in case the parent isn't stored in the graph yet.
    #[allow(clippy::type_complexity)]
    pub fn insert_parents_with_lookup<E>(
        &mut self,
        id: &gix_hash::oid,
        parent_data: &mut dyn FnMut(gix_hash::ObjectId, LazyCommit<'_, 'cache>, Option<&mut T>) -> Result<T, E>,
    ) -> Result<(), E>
    where
        E: From<gix_object::find::existing_iter::Error>
            + From<gix_object::decode::Error>
            + From<commit::iter_parents::Error>,
    {
        let commit = self.lookup(id).map_err(E::from)?;
        let parents: SmallVec<[_; 2]> = commit.iter_parents().collect();
        for parent_id in parents {
            let parent_id = parent_id.map_err(E::from)?;
            let parent = match try_lookup(&parent_id, &*self.find, self.cache, &mut self.parent_buf).map_err(E::from)? {
                Some(p) => p,
                None => continue, // skip missing objects, this is due to shallow clones for instance.
            };

            match self.map.entry(parent_id) {
                gix_hashtable::hash_map::Entry::Vacant(entry) => {
                    entry.insert(parent_data(parent_id, parent, None)?);
                }
                gix_hashtable::hash_map::Entry::Occupied(mut entry) => {
                    parent_data(parent_id, parent, Some(entry.get_mut()))?;
                }
            }
        }
        Ok(())
    }

    /// Turn ourselves into the underlying graph structure, which is a mere mapping between object ids and their data.
    pub fn detach(self) -> IdMap<T> {
        self.map
    }
}

/// Initialization
impl<'find, 'cache, T> Graph<'find, 'cache, T> {
    /// Create a new instance with `objects` to retrieve commits and optionally `cache` to accelerate commit access.
    ///
    /// ### Performance
    ///
    /// `find` should be optimized to access the same object repeatedly, ideally with an object cache to keep the last couple of
    /// most recently used commits.
    /// Furthermore, **none-existing commits should not trigger the pack-db to be refreshed.** Otherwise, performance may be sub-optimal
    /// in shallow repositories as running into non-existing commits will trigger a refresh of the `packs` directory.
    pub fn new(objects: impl gix_object::Find + 'find, cache: Option<&'cache gix_commitgraph::Graph>) -> Self {
        Graph {
            find: Box::new(objects),
            cache,
            map: gix_hashtable::HashMap::default(),
            buf: Vec::new(),
            parent_buf: Vec::new(),
        }
    }
}

/// Commit based methods
impl<T> Graph<'_, '_, Commit<T>> {
    /// Lookup `id` in the graph, but insert it if it's not yet present by looking it up without failing if the commit doesn't exist.
    /// Call `new_data()` to obtain data for a newly inserted commit.
    /// `update_data(data)` gets run either on existing or on new data.
    ///
    /// Note that none of the data updates happen if `id` didn't exist.
    pub fn get_or_insert_commit_default(
        &mut self,
        id: gix_hash::ObjectId,
        new_data: impl FnOnce() -> T,
        update_data: impl FnOnce(&mut T),
    ) -> Result<Option<&mut Commit<T>>, get_or_insert_default::Error> {
        match self.map.entry(id) {
            gix_hashtable::hash_map::Entry::Vacant(entry) => {
                let res = try_lookup(&id, &*self.find, self.cache, &mut self.buf)?;
                let commit = match res {
                    None => return Ok(None),
                    Some(commit) => commit,
                };
                let mut commit = commit.to_owned(new_data)?;
                update_data(&mut commit.data);
                entry.insert(commit);
            }
            gix_hashtable::hash_map::Entry::Occupied(mut entry) => {
                update_data(&mut entry.get_mut().data);
            }
        }
        Ok(self.map.get_mut(&id))
    }

    /// For each stored commit, call `clear` on its data.
    pub fn clear_commit_data(&mut self, mut clear: impl FnMut(&mut T)) {
        self.map.values_mut().for_each(|c| clear(&mut c.data));
    }
}

/// Commit based methods
impl<T: Default> Graph<'_, '_, Commit<T>> {
    /// Lookup `id` in the graph, but insert it if it's not yet present by looking it up without failing if the commit doesn't exist.
    /// Newly inserted commits are populated with default data.
    /// `update_data(data)` gets run either on existing or on new data.
    ///
    /// Note that none of the data updates happen if `id` didn't exist.
    ///
    /// If only commit data is desired without the need for attaching custom data, use
    /// [`try_lookup(id).to_owned()`][Graph::try_lookup()] instead.
    pub fn get_or_insert_commit(
        &mut self,
        id: gix_hash::ObjectId,
        update_data: impl FnOnce(&mut T),
    ) -> Result<Option<&mut Commit<T>>, get_or_insert_default::Error> {
        self.get_or_insert_commit_default(id, T::default, update_data)
    }

    /// Lookup `id` in the graph, but insert it if it's not yet present by looking it up without failing if the commit doesn't exist.
    /// `update_commit(commit)` gets run either on existing or on new data.
    ///
    /// Note that none of the data updates happen if `id` didn't exist in the graph.
    pub fn get_or_insert_full_commit(
        &mut self,
        id: gix_hash::ObjectId,
        update_commit: impl FnOnce(&mut Commit<T>),
    ) -> Result<Option<&mut Commit<T>>, get_or_insert_default::Error> {
        match self.map.entry(id) {
            gix_hashtable::hash_map::Entry::Vacant(entry) => {
                let res = try_lookup(&id, &*self.find, self.cache, &mut self.buf)?;
                let commit = match res {
                    None => return Ok(None),
                    Some(commit) => commit,
                };
                let mut commit = commit.to_owned(T::default)?;
                update_commit(&mut commit);
                entry.insert(commit);
            }
            gix_hashtable::hash_map::Entry::Occupied(mut entry) => {
                update_commit(entry.get_mut());
            }
        }
        Ok(self.map.get_mut(&id))
    }
}

/// Lazy commit access
impl<'cache, T> Graph<'_, 'cache, T> {
    /// Lookup `id` without failing if the commit doesn't exist or `id` isn't a commit,
    /// and assure that `id` is inserted into our set
    /// with a `default` value assigned to it.
    /// `update_data(data)` gets run either on existing or no new data.
    /// Return the commit when done.
    ///
    /// Note that none of the data updates happen if `id` didn't exist.
    ///
    /// If only commit data is desired without the need for attaching custom data, use
    /// [`try_lookup(id)`][Graph::try_lookup()] instead.
    pub fn try_lookup_or_insert_default(
        &mut self,
        id: gix_hash::ObjectId,
        default: impl FnOnce() -> T,
        update_data: impl FnOnce(&mut T),
    ) -> Result<Option<LazyCommit<'_, 'cache>>, get_or_insert_default::Error> {
        let res = try_lookup(&id, &*self.find, self.cache, &mut self.buf)?;
        Ok(res.map(|commit| {
            match self.map.entry(id) {
                gix_hashtable::hash_map::Entry::Vacant(entry) => {
                    let mut data = default();
                    update_data(&mut data);
                    entry.insert(data);
                }
                gix_hashtable::hash_map::Entry::Occupied(mut entry) => {
                    update_data(entry.get_mut());
                }
            }
            commit
        }))
    }

    /// Try to lookup `id` and return a handle to it for accessing its data, but don't fail if the commit doesn't exist
    /// or isn't a commit.
    ///
    /// It's possible that commits don't exist if the repository is shallow.
    pub fn try_lookup(
        &mut self,
        id: &gix_hash::oid,
    ) -> Result<Option<LazyCommit<'_, 'cache>>, gix_object::find::existing_iter::Error> {
        try_lookup(id, &*self.find, self.cache, &mut self.buf)
    }

    /// Lookup `id` and return a handle to it, or fail if it doesn't exist or is no commit.
    pub fn lookup(
        &mut self,
        id: &gix_hash::oid,
    ) -> Result<LazyCommit<'_, 'cache>, gix_object::find::existing_iter::Error> {
        self.try_lookup(id)?
            .ok_or(gix_object::find::existing_iter::Error::NotFound { oid: id.to_owned() })
    }
}

fn try_lookup<'graph, 'cache>(
    id: &gix_hash::oid,
    objects: &dyn gix_object::Find,
    cache: Option<&'cache gix_commitgraph::Graph>,
    buf: &'graph mut Vec<u8>,
) -> Result<Option<LazyCommit<'graph, 'cache>>, gix_object::find::existing_iter::Error> {
    if let Some(cache) = cache {
        if let Some(pos) = cache.lookup(id) {
            return Ok(Some(LazyCommit {
                backing: Either::Right((cache, pos)),
            }));
        }
    }
    #[allow(clippy::manual_map)]
    Ok(
        match objects
            .try_find(id, buf)
            .map_err(gix_object::find::existing_iter::Error::Find)?
        {
            Some(data) => data.kind.is_commit().then_some(LazyCommit {
                backing: Either::Left(buf),
            }),
            None => None,
        },
    )
}

impl<'a, T> Index<&'a gix_hash::oid> for Graph<'_, '_, T> {
    type Output = T;

    fn index(&self, index: &'a oid) -> &Self::Output {
        &self.map[index]
    }
}

/// A commit that contains all information we can obtain through the commit-graph, which is typically enough to fuel any graph iteration.
pub struct Commit<T> {
    /// The parents of the commit.
    pub parents: SmallVec<[gix_hash::ObjectId; 1]>,
    /// The time at which the commit was created.
    pub commit_time: SecondsSinceUnixEpoch,
    /// The generation of the commit, if available.
    pub generation: Option<u32>,
    /// Any kind of data to associate with this commit.
    pub data: T,
}

impl<T> std::fmt::Debug for Commit<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Commit")
            .field("parents", &self.parents)
            .field("commit_time", &self.commit_time)
            .field("generation", &self.generation)
            .field("data", &self.data)
            .finish()
    }
}

impl<T> Clone for Commit<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Commit {
            parents: self.parents.clone(),
            commit_time: self.commit_time,
            generation: self.generation,
            data: self.data.clone(),
        }
    }
}

/// A commit that provides access to graph-related information, on demand.
///
/// The owned version of this type is called [`Commit`] and can be obtained by calling [`LazyCommit::to_owned()`].
pub struct LazyCommit<'graph, 'cache> {
    backing: Either<&'graph [u8], (&'cache gix_commitgraph::Graph, gix_commitgraph::Position)>,
}

enum Either<T, U> {
    Left(T),
    Right(U),
}

use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::Arc,
};

use gix_object::Data;

use crate::{Cache, find::Header};

/// An object database to read from any implementation but write to memory.
/// Previously written objects can be returned from memory upon query, which makes the view of objects consistent.
/// In-Memory objects can be disabled by [taking out its storage](Proxy::take_object_memory). From there in-memory
/// object can also be persisted one by one.
///
/// It's possible to turn off the memory by removing it from the instance.
pub struct Proxy<T> {
    /// The actual odb implementation
    inner: T,
    /// The kind of hash to produce when writing new objects.
    object_hash: gix_hash::Kind,
    /// The storage for in-memory objects.
    /// If `None`, the proxy will always read from and write-through to `inner`.
    memory: Option<RefCell<Storage>>,
}

/// Lifecycle
impl<T> Proxy<T> {
    /// Create a new instance using `odb` as actual object provider, with an empty in-memory store for
    /// objects that are to be written.
    /// Use `object_hash` to determine the kind of hash to produce when writing new objects.
    pub fn new(odb: T, object_hash: gix_hash::Kind) -> Proxy<T> {
        Proxy {
            inner: odb,
            object_hash,
            memory: Some(Default::default()),
        }
    }

    /// Turn ourselves into our inner object database, while deallocating objects stored in memory.
    pub fn into_inner(self) -> T {
        self.inner
    }

    /// Strip object memory off this instance, which means that writes will go through to the inner object database
    /// right away.
    /// This mode makes the proxy fully transparent.
    pub fn with_write_passthrough(mut self) -> Self {
        self.memory.take();
        self
    }
}

impl Proxy<Cache<crate::store::Handle<Arc<crate::Store>>>> {
    /// No op, as we are containing an arc handle already.
    pub fn into_arc(self) -> std::io::Result<Proxy<Cache<crate::store::Handle<Arc<crate::Store>>>>> {
        Ok(self)
    }
}

impl Proxy<Cache<crate::store::Handle<Rc<crate::Store>>>> {
    /// Create an entirely new instance, but with the in-memory objects moving between them.
    pub fn into_arc(self) -> std::io::Result<Proxy<Cache<crate::store::Handle<Arc<crate::Store>>>>> {
        Ok(Proxy {
            inner: self.inner.into_arc()?,
            object_hash: self.object_hash,
            memory: self.memory,
        })
    }
}

impl From<crate::Handle> for Proxy<crate::Handle> {
    fn from(odb: crate::Handle) -> Self {
        let object_hash = odb.store.object_hash;
        Proxy::new(odb, object_hash)
    }
}

/// Memory Access
impl<T> Proxy<T> {
    /// Take all the objects in memory so far, with the memory storage itself and return it.
    ///
    /// The instance will remain in a state where it won't be able to store objects in memory at all,
    /// they will now be stored in the underlying object database.
    /// This mode makes the proxy fully transparent.
    ///
    /// To avoid that, use [`reset_object_memory()`](Self::reset_object_memory()) or return the storage
    /// using [`set_object_memory()`](Self::set_object_memory()).
    pub fn take_object_memory(&mut self) -> Option<Storage> {
        self.memory.take().map(RefCell::into_inner)
    }

    /// Set the object storage to contain only `new` objects, and return whichever objects were there previously.
    pub fn set_object_memory(&mut self, new: Storage) -> Option<Storage> {
        let previous = self.take_object_memory();
        self.memory = Some(RefCell::new(new));
        previous
    }

    /// If objects aren't written to memory yet, this will happen after the call.
    ///
    /// Otherwise, no change will be performed.
    pub fn enable_object_memory(&mut self) -> &mut Self {
        if self.memory.is_none() {
            self.memory = Some(Default::default());
        }
        self
    }

    /// Reset the internal storage to be empty, and return the previous storage, with all objects
    /// it contained.
    ///
    /// Note that this does nothing if this instance didn't contain object memory in the first place.
    /// In that case, set it explicitly.
    pub fn reset_object_memory(&self) -> Option<Storage> {
        self.memory.as_ref().map(|m| std::mem::take(&mut *m.borrow_mut()))
    }

    /// Return the amount of objects currently stored in memory.
    pub fn num_objects_in_memory(&self) -> usize {
        self.memory.as_ref().map_or(0, |m| m.borrow().len())
    }
}

impl<T> Clone for Proxy<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Proxy {
            inner: self.inner.clone(),
            object_hash: self.object_hash,
            memory: self.memory.clone(),
        }
    }
}

impl<T> gix_object::Find for Proxy<T>
where
    T: gix_object::Find,
{
    fn try_find<'a>(
        &self,
        id: &gix_hash::oid,
        buffer: &'a mut Vec<u8>,
    ) -> Result<Option<Data<'a>>, gix_object::find::Error> {
        if let Some(map) = self.memory.as_ref() {
            let map = map.borrow();
            if let Some((kind, data)) = map.get(id) {
                buffer.clear();
                buffer.extend_from_slice(data);
                return Ok(Some(Data {
                    kind: *kind,
                    object_hash: id.kind(),
                    data: &*buffer,
                }));
            }
        }
        self.inner.try_find(id, buffer)
    }
}

impl<T> gix_object::Exists for Proxy<T>
where
    T: gix_object::Exists,
{
    fn exists(&self, id: &gix_hash::oid) -> bool {
        self.memory.as_ref().is_some_and(|map| map.borrow().contains_key(id)) || self.inner.exists(id)
    }
}

impl<T> crate::Header for Proxy<T>
where
    T: crate::Header,
{
    fn try_header(&self, id: &gix_hash::oid) -> Result<Option<Header>, gix_object::find::Error> {
        if let Some(map) = self.memory.as_ref() {
            let map = map.borrow();
            if let Some((kind, data)) = map.get(id) {
                return Ok(Some(Header::Loose {
                    kind: *kind,
                    size: data.len() as u64,
                }));
            }
        }
        self.inner.try_header(id)
    }
}

impl<T> gix_object::FindHeader for Proxy<T>
where
    T: gix_object::FindHeader,
{
    fn try_header(&self, id: &gix_hash::oid) -> Result<Option<gix_object::Header>, gix_object::find::Error> {
        if let Some(map) = self.memory.as_ref() {
            let map = map.borrow();
            if let Some((kind, data)) = map.get(id) {
                return Ok(Some(gix_object::Header {
                    kind: *kind,
                    size: data.len() as u64,
                }));
            }
        }
        self.inner.try_header(id)
    }
}

impl<T> gix_object::Write for Proxy<T>
where
    T: gix_object::Write,
{
    fn write_stream(
        &self,
        kind: gix_object::Kind,
        size: u64,
        from: &mut dyn std::io::Read,
    ) -> Result<gix_hash::ObjectId, gix_object::write::Error> {
        let Some(map) = self.memory.as_ref() else {
            return self.inner.write_stream(kind, size, from);
        };

        let mut buf = Vec::new();
        from.read_to_end(&mut buf)?;

        let id = gix_object::compute_hash(self.object_hash, kind, &buf)?;
        map.borrow_mut().insert(id, (kind, buf));
        Ok(id)
    }
}

impl<T> Deref for Proxy<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Proxy<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// A mapping between an object id and all data corresponding to an object, acting like a `HashMap<ObjectID, (Kind, Data)>`.
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Storage(gix_hashtable::HashMap<gix_hash::ObjectId, (gix_object::Kind, Vec<u8>)>);

impl Deref for Storage {
    type Target = gix_hashtable::HashMap<gix_hash::ObjectId, (gix_object::Kind, Vec<u8>)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Storage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

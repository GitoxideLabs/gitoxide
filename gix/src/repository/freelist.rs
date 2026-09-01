use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
};

/// A buffer that is returned to the free-list after usage.
#[derive(Clone)]
pub struct Buffer<'repo> {
    /// The buffer that would be returned to the freelist of `repo`.
    /// Note that buffers without capacity (i.e. without allocation) aren't returned.
    pub inner: Vec<u8>,
    /// The repository from whose free-list the `inner` buffer was taken, and to which it will be returned.
    pub repo: &'repo crate::Repository,
}

impl From<Buffer<'_>> for Vec<u8> {
    fn from(mut value: Buffer<'_>) -> Self {
        std::mem::take(&mut value.inner)
    }
}

impl Deref for Buffer<'_> {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Buffer<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Drop for Buffer<'_> {
    fn drop(&mut self) {
        self.repo.reuse_buffer(&mut self.inner);
    }
}

/// Internal
impl crate::Repository {
    /// Note that the returned buffer might still have data in it.
    #[inline]
    pub(crate) fn free_buf(&self) -> Vec<u8> {
        self.bufs
            .as_ref()
            .and_then(|bufs| bufs.borrow_mut().pop())
            .unwrap_or_default()
    }

    /// This method is commonly called from the destructor of objects that previously claimed an entry
    /// in the free-list with [crate::Repository::free_buf].
    /// They are welcome to take out the data themselves, for instance when the object is detached, to avoid
    /// it to be reclaimed.
    #[inline]
    pub(crate) fn reuse_buffer(&self, data: &mut Vec<u8>) {
        if data.capacity() > 0
            && let Some(bufs) = self.bufs.as_ref()
        {
            bufs.borrow_mut().push(std::mem::take(data));
        }
    }
}

/// Freelist configuration
///
/// The free-list is an internal and 'transparent' mechanism for obtaining and re-using memory buffers when
/// reading objects. That way, trashing is avoided as buffers are re-used and re-written.
///
/// However, there are circumstances when releasing memory early is preferred, for instance on the server side.
///
/// Also note that the free-list isn't cloned, so each clone of this instance starts with an empty one.
impl crate::Repository {
    /// Return an empty buffer which is tied to this repository instance, and reuse its memory allocation by
    /// keeping it around even after it drops.
    pub fn empty_reusable_buffer(&self) -> Buffer<'_> {
        let mut inner = self.free_buf();
        inner.clear();
        Buffer { inner, repo: self }
    }

    /// Set the currently used freelist to `list`. If `None`, it will be disabled entirely.
    ///
    /// Return the currently previously allocated free-list, a list of reusable buffers typically used when reading objects.
    /// May be `None` if there was no free-list.
    pub fn set_freelist(&mut self, list: Option<Vec<Vec<u8>>>) -> Option<Vec<Vec<u8>>> {
        let previous = self.bufs.take();
        self.bufs = list.map(RefCell::new);
        previous.map(RefCell::into_inner)
    }

    /// A builder method to disable the free-list on a newly created instance.
    pub fn without_freelist(mut self) -> Self {
        self.bufs.take();
        self
    }
}

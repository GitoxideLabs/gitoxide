use std::marker::PhantomData;

/// SAFETY: This type is used to allow access to a size-optimized vec of items that form a
/// tree, and we need to access it concurrently with each thread taking its own root node,
/// and working its way through all the reachable leaves.
///
/// The tree was built by decoding a pack whose entries refer to exactly one base. OFS_DELTA entries point
/// backwards, while in-pack REF_DELTA entries are attached once their base object id is known.
/// Ref children are removed from a shared lookup before attachment, so no child can be reached by two threads.
///
/// Thus I believe it's impossible for this data structure to end up in a place where it violates its assumption.
pub(super) struct ItemSliceSync<'a, T>
where
    T: Send,
{
    items: *mut T,
    #[cfg(debug_assertions)]
    len: usize,
    phantom: PhantomData<&'a mut T>,
}

impl<'a, T> ItemSliceSync<'a, T>
where
    T: Send,
{
    pub(super) fn new(items: &'a mut [T]) -> Self {
        ItemSliceSync {
            items: items.as_mut_ptr(),
            #[cfg(debug_assertions)]
            len: items.len(),
            phantom: PhantomData,
        }
    }

    // SAFETY: The index must point into the slice and must not be reused concurrently.
    #[expect(unsafe_code)]
    pub(super) unsafe fn get_mut(&self, index: usize) -> &'a mut T {
        #[cfg(debug_assertions)]
        if index >= self.len {
            panic!("index out of bounds: the len is {} but the index is {index}", self.len);
        }
        // SAFETY:
        //    - The index is within the slice (required by documentation)
        //    - We have mutable access to `items` as ensured by Self::new()
        //    - This is the only method on this type giving access to items
        //    - The documentation requires that this access is unique
        unsafe { &mut *self.items.add(index) }
    }
}

// SAFETY: This is logically an &mut T, which is Send if T is Send
// (note: this is different from &T, which also needs T: Sync)
#[expect(unsafe_code)]
unsafe impl<T> Send for ItemSliceSync<'_, T> where T: Send {}
// SAFETY: This is logically an &mut T, which is Sync if T is Sync
#[expect(unsafe_code)]
unsafe impl<T> Sync for ItemSliceSync<'_, T> where T: Send {}

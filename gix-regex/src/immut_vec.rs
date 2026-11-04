// Derived from posix-regex 0.1.4, upstream revision
// 063882aa6051b5075062d20d00b41ddc0fb8cd89.
// Copyright (c) 2018 jD91mZM2. Licensed under the MIT license; see LICENSE-MIT.

use alloc::vec::Vec;
use core::cell::RefCell;

pub struct ImmutVecItem<T> {
    prev: Option<usize>,
    data: T,
}

#[derive(Copy, Clone)]
pub struct ImmutVec<'a, T> {
    inner: &'a RefCell<Vec<ImmutVecItem<T>>>,
    id: Option<usize>,
}

impl<'a, T> ImmutVec<'a, T> {
    pub fn new(inner: &'a RefCell<Vec<ImmutVecItem<T>>>) -> Self {
        Self { inner, id: None }
    }
    #[must_use = "push does nothing to the original vector"]
    pub fn push(self, item: T) -> Self {
        let mut inner = self.inner.borrow_mut();
        let id = inner.len();
        inner.push(ImmutVecItem {
            prev: self.id,
            data: item,
        });
        Self { id: Some(id), ..self }
    }
}

impl<'a, T: Copy> ImmutVec<'a, T> {
    #[must_use = "pop does nothing to the original vector"]
    pub fn pop(self) -> (Self, Option<T>) {
        let inner = self.inner.borrow();
        let id = match self.id {
            None => return (self, None),
            Some(id) => id,
        };
        let item = &inner[id];
        (Self { id: item.prev, ..self }, Some(item.data))
    }
    pub fn iter_rev(self) -> ImmutVecIter<'a, T> {
        ImmutVecIter(self)
    }
}

pub struct ImmutVecIter<'a, T: Copy>(ImmutVec<'a, T>);
impl<T: Copy> Iterator for ImmutVecIter<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let (new, item) = self.0.pop();
        self.0 = new;
        item
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;
    use core::cell::RefCell;

    use super::ImmutVec;

    #[test]
    fn immutvec_iter_works() {
        let cell = RefCell::new(Vec::new());
        let mut iv = ImmutVec::new(&cell);

        for i in 0..=100 {
            iv = iv.push(i);
        }
        assert_eq!(Some(100), iv.id);

        assert_eq!(
            iv.iter_rev().collect::<Vec<_>>(),
            (0..=100).rev().collect::<Vec<_>>(),
            "the persistent sequence is visited from newest to oldest"
        );
    }

    #[test]
    fn push_to_rewound_vec_sets_correct_id() {
        let cell = RefCell::new(Vec::new());
        let mut iv = ImmutVec::new(&cell);

        for i in 0..10 {
            iv = iv.push(i);
        }
        assert_eq!(Some(9), iv.id);

        for _ in 0..10 {
            (iv, _) = iv.pop();
        }
        assert_eq!(
            10,
            iv.inner.borrow().len(),
            "Length shouldn't change from popping items"
        );

        // Pushing an item should append to the end regardless of where we are in the vector
        assert_eq!(None, iv.id);
        iv = iv.push(10);
        assert_eq!(11, iv.inner.borrow().len());
        assert_eq!(Some(10), iv.id);
    }
}

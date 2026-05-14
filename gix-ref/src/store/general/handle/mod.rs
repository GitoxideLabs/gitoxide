#![allow(dead_code)]
use crate::{Namespace, store};

#[derive(Clone)]
pub(crate) enum State {
    Loose { store: crate::file::Store },
}

impl crate::Store {
    /// Return a new handle which sees all references if `namespace` is `None` or all read and write operations are limited
    /// to the given `namespace` if `Some`.
    pub fn to_handle(&self) -> store::Handle {
        Self::new_handle_inner(&self.inner, None)
    }

    /// As above, but supports a namespace to be set
    pub fn to_handle_namespaced(&self, namespace: Option<Namespace>) -> store::Handle {
        Self::new_handle_inner(&self.inner, namespace)
    }

    fn new_handle_inner(state: &store::State, namespace: Option<Namespace>) -> store::Handle {
        store::Handle {
            state: match state {
                store::State::Loose { store } => store::handle::State::Loose {
                    store: {
                        let mut store = store.clone();
                        store.namespace = namespace;
                        store
                    },
                },
            },
        }
    }
}

///
pub mod find;

mod iter {
    // impl store::Handle {
    //     pub fn iter<'p, 's>(&'s self, packed: Option<&'p packed::Buffer>) -> std::io::Result<LooseThenPacked<'p, 's>> {
    // }
}

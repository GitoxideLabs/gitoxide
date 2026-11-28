use std::{
    ops::Deref,
    sync::{Arc, atomic::AtomicBool},
};

#[derive(Clone)]
/// An interrupt flag owned by the caller or shared with a process-global flag.
pub enum OwnedOrStaticAtomicBool {
    /// An owned flag which can be shared across threads.
    Owned {
        /// The interrupt flag.
        flag: Arc<AtomicBool>,
        /// Whether the flag was created for a single operation. Iterators use a separate flag on drop.
        private: bool,
    },
    /// A process-global interrupt flag.
    Shared(&'static AtomicBool),
}

impl Default for OwnedOrStaticAtomicBool {
    fn default() -> Self {
        OwnedOrStaticAtomicBool::Owned {
            flag: Arc::new(AtomicBool::default()),
            private: true,
        }
    }
}

impl Deref for OwnedOrStaticAtomicBool {
    type Target = std::sync::atomic::AtomicBool;

    fn deref(&self) -> &Self::Target {
        match self {
            OwnedOrStaticAtomicBool::Owned { flag, .. } => flag,
            OwnedOrStaticAtomicBool::Shared(flag) => flag,
        }
    }
}

impl From<&'static AtomicBool> for OwnedOrStaticAtomicBool {
    fn from(value: &'static AtomicBool) -> Self {
        OwnedOrStaticAtomicBool::Shared(value)
    }
}

impl<'a> From<&'a Arc<AtomicBool>> for OwnedOrStaticAtomicBool {
    fn from(value: &'a Arc<AtomicBool>) -> Self {
        OwnedOrStaticAtomicBool::Owned {
            flag: value.clone(),
            private: false,
        }
    }
}

impl From<Arc<AtomicBool>> for OwnedOrStaticAtomicBool {
    fn from(flag: Arc<AtomicBool>) -> Self {
        OwnedOrStaticAtomicBool::Owned { flag, private: false }
    }
}

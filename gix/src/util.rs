use std::{
    ops::Deref,
    sync::{Arc, atomic::AtomicBool},
};

#[derive(Clone)]
pub enum OwnedOrStaticAtomicBool {
    Owned {
        flag: Arc<AtomicBool>,
        #[cfg_attr(not(feature = "parallel"), allow(dead_code))]
        private: bool,
    },
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
#[cfg(feature = "parallel")]
#[allow(clippy::type_complexity)]
pub fn parallel_iter_drop<T, U, V>(
    mut rx_and_join: Option<(
        std::sync::mpsc::Receiver<T>,
        std::thread::JoinHandle<U>,
        Option<std::thread::JoinHandle<V>>,
    )>,
    should_interrupt: &OwnedOrStaticAtomicBool,
) {
    let Some((rx, handle, maybe_handle)) = rx_and_join.take() else {
        return;
    };
    let prev = should_interrupt.swap(true, std::sync::atomic::Ordering::Relaxed);
    let undo = match &should_interrupt {
        OwnedOrStaticAtomicBool::Shared(flag) => *flag,
        OwnedOrStaticAtomicBool::Owned { flag, private: false } => flag.as_ref(),
        OwnedOrStaticAtomicBool::Owned { private: true, .. } => {
            // Leak the handle to let it shut down in the background, so drop returns more quickly.
            drop((rx, handle, maybe_handle));
            return;
        }
    };
    // Do not for the remaining threads. Everything but index-from-tree is interruptible, and that wouldn't
    // take very long even with huge trees.
    // If this every becomes a problem, just make `index::from-tree` interruptible, and keep waiting for handles here.
    drop((maybe_handle, handle));
    undo.fetch_update(
        std::sync::atomic::Ordering::SeqCst,
        std::sync::atomic::Ordering::SeqCst,
        |current| current.then_some(prev),
    )
    .ok();
}

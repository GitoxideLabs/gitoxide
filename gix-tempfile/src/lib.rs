//! git-style registered tempfiles that are removed upon typical termination signals.
//!
//! To register signal handlers in a typical application that doesn't have its own, call
//! [`gix_tempfile::signal::setup(Default::default())`][signal::setup()] before creating the first tempfile.
//!
//! Signal handlers are powered by [`signal-hook`] to get notified when the application is told to shut down
//! to assure tempfiles are deleted. The deletion is filtered by process id to allow forks to have their own
//! set of tempfiles that won't get deleted when the parent process exits.
//!
//! ### Initial Setup
//!
//! As no handlers for `TERMination` are installed, it is required to call [`signal::setup()`] before creating
//! the first tempfile. This also allows to control how this crate integrates with
//! other handlers under application control.
//!
//! As a general rule of thumb, use `Default::default()` as argument to emulate the default behaviour and
//! abort the process after cleaning temporary files. Read more about options in [`signal::handler::Mode`].
//!
//! # Limitations
//!
//! ## Tempfiles might remain on disk
//!
//! * Uninterruptible signals are received like `SIGKILL`
//! * The application is performing a write operation on the tempfile when a signal arrives, preventing this tempfile to be removed,
//!   but not others. Any other operation dealing with the tempfile suffers from the same issue.
//!
//! [`signal-hook`]: https://docs.rs/signal-hook
//!
//! ## Examples
//!
//! ```
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use std::io::Write;
//!
//! # let dir = tempfile::tempdir()?;
//! # let destination = dir.path().join("config");
//! let mut tempfile = gix_tempfile::writable_at(
//!     dir.path().join("config.lock"),
//!     gix_tempfile::ContainingDirectory::Exists,
//!     gix_tempfile::AutoRemove::Tempfile,
//! )?;
//! tempfile.write_all(b"new = value\n")?;
//! tempfile.persist(&destination)?;
//!
//! assert_eq!(std::fs::read_to_string(&destination)?, "new = value\n");
//! # Ok(()) }
//! ```
//!
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![deny(missing_docs, unsafe_code)]

use std::{
    io,
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::atomic::AtomicUsize,
};

use std::sync::LazyLock;

#[cfg(feature = "hp-hashmap")]
type HashMap<K, V> = dashmap::DashMap<K, V>;

#[cfg(not(feature = "hp-hashmap"))]
mod hashmap {
    use std::collections::HashMap;

    use parking_lot::Mutex;

    // TODO(performance): use the `gix-hashtable` slot-map once available. It seems quite fast already though, so experiment.
    pub struct Concurrent<K, V> {
        inner: Mutex<HashMap<K, V>>,
    }

    impl<K, V> Default for Concurrent<K, V>
    where
        K: Eq + std::hash::Hash,
    {
        fn default() -> Self {
            Concurrent {
                inner: Default::default(),
            }
        }
    }

    impl<K, V> Concurrent<K, V>
    where
        K: Eq + std::hash::Hash + Clone,
    {
        pub fn insert(&self, key: K, value: V) -> Option<V> {
            self.inner.lock().insert(key, value)
        }

        pub fn remove(&self, key: &K) -> Option<(K, V)> {
            self.inner.lock().remove(key).map(|v| (key.clone(), v))
        }

        pub fn for_each<F>(&self, cb: F)
        where
            Self: Sized,
            F: FnMut(&mut V),
        {
            if let Some(mut guard) = self.inner.try_lock() {
                guard.values_mut().for_each(cb);
            }
        }
    }
}

#[cfg(not(feature = "hp-hashmap"))]
type HashMap<K, V> = hashmap::Concurrent<K, V>;

pub use gix_fs::dir::{create as create_dir, remove as remove_dir};

/// signal setup and reusable handlers.
#[cfg(feature = "signals")]
pub mod signal;

mod forksafe;
use forksafe::ForksafeTempfile;

pub mod handle;
use crate::handle::{Closed, Writable};

///
pub mod registry;

static NEXT_MAP_INDEX: AtomicUsize = AtomicUsize::new(0);
static REGISTRY: LazyLock<HashMap<usize, Option<ForksafeTempfile>>> = LazyLock::new(|| {
    #[cfg(feature = "signals")]
    if signal::handler::MODE.load(std::sync::atomic::Ordering::SeqCst) != signal::handler::Mode::None as usize {
        for sig in signal_hook::consts::TERM_SIGNALS {
            // SAFETY: handlers are considered unsafe because a lot can go wrong. See `cleanup_tempfiles()` for details on safety.
            #[allow(unsafe_code)]
            unsafe {
                #[cfg(not(windows))]
                {
                    signal_hook_registry::register_sigaction(*sig, signal::handler::cleanup_tempfiles_nix)
                }
                #[cfg(windows)]
                {
                    signal_hook::low_level::register(*sig, signal::handler::cleanup_tempfiles_windows)
                }
            }
            .expect("signals can always be installed");
        }
    }
    HashMap::default()
});

/// A type expressing the ways we can deal with directories containing a tempfile.
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub enum ContainingDirectory {
    /// Assume the directory for the tempfile exists and cause failure if it doesn't
    Exists,
    /// Create the directory recursively with the given number of retries in a way that is somewhat race resistant
    /// depending on the amount of retries.
    CreateAllRaceProof(create_dir::Retries),
}

/// A type expressing the ways we cleanup after ourselves to remove resources we created.
/// Note that cleanup has no effect if the tempfile is persisted.
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum AutoRemove {
    /// Remove the temporary file after usage if it wasn't persisted.
    Tempfile,
    /// Remove the temporary file as well the containing directories if they are empty until the given `directory`.
    TempfileAndEmptyParentDirectoriesUntil {
        /// The directory which shall not be removed even if it is empty.
        boundary_directory: PathBuf,
    },
}

impl AutoRemove {
    fn execute_best_effort(self, directory_to_potentially_delete: &Path) -> Option<PathBuf> {
        match self {
            AutoRemove::Tempfile => None,
            AutoRemove::TempfileAndEmptyParentDirectoriesUntil { boundary_directory } => {
                remove_dir::empty_upward_until_boundary(directory_to_potentially_delete, &boundary_directory).ok();
                Some(boundary_directory)
            }
        }
    }
}

/// A registered temporary file which will delete itself on drop or if the program is receiving signals that
/// should cause it to terminate.
///
/// # Note
///
/// Signals interrupting the calling thread right after taking ownership of the registered tempfile
/// will cause all but this tempfile to be removed automatically. In the common case it will persist on disk as destructors
/// were not called or didn't get to remove the file.
///
/// In the best case the file is a true temporary with a non-clashing name that 'only' fills up the disk,
/// in the worst case the temporary file is used as a lock file which may leave the repository in a locked
/// state forever.
///
/// This kind of raciness exists whenever [`take()`][Handle::take()] is used and can't be circumvented.
#[derive(Debug)]
#[must_use = "A handle that is immediately dropped doesn't lock a resource meaningfully"]
pub struct Handle<Marker: std::fmt::Debug> {
    id: usize,
    _marker: PhantomData<Marker>,
}

/// A shortcut to [`Handle::<Writable>::new()`], creating a writable temporary file with non-clashing name in a directory.
pub fn new(
    containing_directory: impl AsRef<Path>,
    directory: ContainingDirectory,
    cleanup: AutoRemove,
) -> io::Result<Handle<Writable>> {
    Handle::<Writable>::new(containing_directory, directory, cleanup)
}

/// A shortcut to [`Handle::<Writable>::at()`] providing a writable temporary file at the given path.
pub fn writable_at(
    path: impl AsRef<Path>,
    directory: ContainingDirectory,
    cleanup: AutoRemove,
) -> io::Result<Handle<Writable>> {
    Handle::<Writable>::at(path, directory, cleanup)
}

/// Like [`writable_at`], but allows to set the given filesystem `permissions`.
pub fn writable_at_with_permissions(
    path: impl AsRef<Path>,
    directory: ContainingDirectory,
    cleanup: AutoRemove,
    permissions: std::fs::Permissions,
) -> io::Result<Handle<Writable>> {
    Handle::<Writable>::at_with_permissions(path, directory, cleanup, permissions)
}

/// A shortcut to [`Handle::<Closed>::at()`] providing a closed temporary file to mark the presence of something.
pub fn mark_at(
    path: impl AsRef<Path>,
    directory: ContainingDirectory,
    cleanup: AutoRemove,
) -> io::Result<Handle<Closed>> {
    Handle::<Closed>::at(path, directory, cleanup)
}

/// Like [`mark_at`], but allows to set the given filesystem `permissions`.
pub fn mark_at_with_permissions(
    path: impl AsRef<Path>,
    directory: ContainingDirectory,
    cleanup: AutoRemove,
    permissions: std::fs::Permissions,
) -> io::Result<Handle<Closed>> {
    Handle::<Closed>::at_with_permissions(path, directory, cleanup, permissions)
}

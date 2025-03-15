// TODO: tests
use gix_features::threading::{get_mut, get_ref, MutableOnDemand, OwnShared};
use std::ops::Deref;

/// A structure holding enough information to reload a value if its on-disk representation changes as determined by its modified time.
#[derive(Debug)]
pub struct FileSnapshot<T: std::fmt::Debug> {
    value: T,
    modified: std::time::SystemTime,
}

/// Lifecycle
impl<T: std::fmt::Debug> FileSnapshot<T> {
    /// A way for users to create 'fake' snapshot from `value` that isn't actually linked to a file on disk.
    ///
    /// This is useful if there are alternative ways of obtaining the contained instance as fallback to trying
    /// to read it from disk.
    pub fn new(value: T) -> Self {
        FileSnapshot {
            value,
            modified: std::time::UNIX_EPOCH,
        }
    }
}

impl<T: std::fmt::Debug> From<T> for FileSnapshot<T> {
    fn from(value: T) -> Self {
        FileSnapshot::new(value)
    }
}

impl<T: Clone + std::fmt::Debug> Clone for FileSnapshot<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            modified: self.modified,
        }
    }
}

impl<T: Clone + std::fmt::Debug> FileSnapshot<T> {
    /// Return the contained instance if nobody else is holding it, or clone it otherwise.
    pub fn into_owned_or_cloned(self: OwnShared<Self>) -> T {
        match OwnShared::try_unwrap(self) {
            Ok(this) => this.value,
            Err(this) => this.value.clone(),
        }
    }
}

/// A snapshot of a resource which is up-to-date in the moment it is retrieved.
pub type SharedFileSnapshot<T> = OwnShared<FileSnapshot<T>>;

/// Use this type for fields in structs that are to store the [`FileSnapshot`], typically behind an [`OwnShared`].
///
/// Note that the resource itself is behind another [`OwnShared`] to allow it to be used without holding any kind of lock, hence
/// without blocking updates while it is used.
#[derive(Debug, Default)]
pub struct SharedFileSnapshotMut<T: std::fmt::Debug>(pub MutableOnDemand<Option<SharedFileSnapshot<T>>>);

impl<T: std::fmt::Debug> Deref for FileSnapshot<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: std::fmt::Debug> std::ops::DerefMut for FileSnapshot<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T: std::fmt::Debug> Deref for SharedFileSnapshotMut<T> {
    type Target = MutableOnDemand<Option<SharedFileSnapshot<T>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: std::fmt::Debug> SharedFileSnapshotMut<T> {
    /// Create a new instance of this type.
    ///
    /// Useful in case `Default::default()` isn't working for some reason.
    pub fn new() -> Self {
        SharedFileSnapshotMut(MutableOnDemand::new(None))
    }

    /// Refresh `state` forcefully by re-`open`ing the resource. Note that `open()` returns `None` if the resource isn't
    /// present on disk, and that it's critical that the modified time is obtained _before_ opening the resource.
    pub fn force_refresh<E>(
        &self,
        open: impl FnOnce() -> Result<Option<(std::time::SystemTime, T)>, E>,
    ) -> Result<(), E> {
        let mut state = get_mut(&self.0);
        *state = open()?.map(|(modified, value)| OwnShared::new(FileSnapshot { value, modified }));
        Ok(())
    }

    /// Assure that the resource in `state` is up-to-date by comparing the `current_modification_time` with the one we know in `state`
    /// and by acting accordingly.
    /// Returns the potentially updated/reloaded resource if it is still present on disk, which then represents a snapshot that is up-to-date
    /// in that very moment, or `None` if the underlying file doesn't exist.
    ///
    /// Note that even though this is racy, each time a request is made there is a chance to see the actual state.
    pub fn recent_snapshot<E>(
        &self,
        mut current_modification_time: impl FnMut() -> Option<std::time::SystemTime>,
        open: impl FnOnce() -> Result<Option<T>, E>,
    ) -> Result<Option<SharedFileSnapshot<T>>, E> {
        let state = get_ref(self);
        let recent_modification = current_modification_time();
        let buffer = match (&*state, recent_modification) {
            (None, None) => (*state).clone(),
            (Some(_), None) => {
                drop(state);
                let mut state = get_mut(self);
                *state = None;
                (*state).clone()
            }
            (Some(snapshot), Some(modified_time)) => {
                if snapshot.modified < modified_time {
                    drop(state);
                    let mut state = get_mut(self);

                    if let (Some(_snapshot), Some(modified_time)) = (&*state, current_modification_time()) {
                        *state = open()?.map(|value| {
                            OwnShared::new(FileSnapshot {
                                value,
                                modified: modified_time,
                            })
                        });
                    }

                    (*state).clone()
                } else {
                    // Note that this relies on sub-section precision or else is a race when the packed file was just changed.
                    // It's nothing we can know though, so… up to the caller unfortunately.
                    Some(snapshot.clone())
                }
            }
            (None, Some(_modified_time)) => {
                drop(state);
                let mut state = get_mut(self);
                // Still in the same situation? If so, load the buffer. This compensates for the trampling herd
                // during lazy-loading at the expense of another mtime check.
                if let (None, Some(modified_time)) = (&*state, current_modification_time()) {
                    *state = open()?.map(|value| {
                        OwnShared::new(FileSnapshot {
                            value,
                            modified: modified_time,
                        })
                    });
                }
                (*state).clone()
            }
        };
        Ok(buffer)
    }
}

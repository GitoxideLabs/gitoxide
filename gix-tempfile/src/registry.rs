use crate::REGISTRY;

/// Remove all tempfiles still registered on our global registry, and leak their data to be signal-safe.
/// This happens on a best-effort basis with all errors being ignored.
///
/// # Safety
/// Note that Mutexes of any kind are not allowed, and so aren't allocation or deallocation of memory.
/// We are using lock-free datastructures and sprinkle in `std::mem::forget` to avoid deallocating.
/// Most importantly, we use `try_lock()` which uses an atomic int only without blocking, making our register method safe to use,
/// at the expense of possibly missing a lock file if another thread wants to obtain it or put it back
/// (i.e. mutates the registry shard).
pub fn cleanup_tempfiles_signal_safe() {
    let current_pid = std::process::id();
    #[cfg(feature = "hp-hashmap")]
    {
        use std::sync::atomic::Ordering;

        use crate::NEXT_MAP_INDEX;

        let one_past_last_index = NEXT_MAP_INDEX.load(Ordering::SeqCst);
        for idx in 0..one_past_last_index {
            if let Some(entry) = REGISTRY.try_entry(idx) {
                entry.and_modify(|tempfile| {
                    if tempfile.as_ref().is_some_and(|tf| tf.owning_process_id == current_pid)
                        && let Some(tempfile) = tempfile.take()
                    {
                        tempfile.drop_without_deallocation();
                    }
                });
            }
        }
    }
    #[cfg(not(feature = "hp-hashmap"))]
    {
        REGISTRY.for_each(|tf| {
            if tf.as_ref().is_some_and(|tf| tf.owning_process_id == current_pid) {
                if let Some(tf) = tf.take() {
                    tf.drop_without_deallocation();
                }
            }
        });
    }
}

/// Remove all tempfiles still registered on our global registry.
/// This happens on a best-effort basis with all errors being ignored.
///
/// # Note
///
/// Must not be called from within signal hooks. For that, use [`cleanup_tempfiles_signal_safe()`].
pub fn cleanup_tempfiles() {
    let current_pid = std::process::id();
    #[cfg(feature = "hp-hashmap")]
    REGISTRY.iter_mut().for_each(|mut tf| {
        if tf.as_ref().is_some_and(|tf| tf.owning_process_id == current_pid) {
            tf.take();
        }
    });
    #[cfg(not(feature = "hp-hashmap"))]
    REGISTRY.for_each(|tf| {
        if tf.as_ref().is_some_and(|tf| tf.owning_process_id == current_pid) {
            tf.take();
        }
    });
}

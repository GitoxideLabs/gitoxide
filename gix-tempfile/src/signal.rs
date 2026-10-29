use std::sync::LazyLock;

use crate::REGISTRY;

/// Initialize signal handlers and other state to keep track of tempfiles, and **must be called before the first tempfile is created**,
/// allowing to set the `mode` in which signal handlers are installed.
///
/// Only has an effect the first time it is called.
///
/// Note that it is possible to not call this function and instead call
/// [`registry::cleanup_tempfiles_signal_safe()`][crate::registry::cleanup_tempfiles_signal_safe()]
/// from a signal handler under the application's control.
pub fn setup(mode: handler::Mode) {
    handler::MODE.store(mode as usize, std::sync::atomic::Ordering::SeqCst);
    LazyLock::force(&REGISTRY);
}

///
pub mod handler {
    use std::sync::atomic::AtomicUsize;

    pub(crate) static MODE: AtomicUsize = AtomicUsize::new(Mode::None as usize);

    /// Define how our signal handlers act
    #[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
    #[non_exhaustive]
    pub enum Mode {
        /// Do not install a signal handler at all, but have somebody else call our handler directly.
        None = 0,
        /// Delete all remaining registered tempfiles on termination.
        DeleteTempfilesOnTermination = 1,
        /// Delete all remaining registered tempfiles on termination and emulate the default handler behaviour.
        ///
        /// This typically leads to the process being aborted.
        DeleteTempfilesOnTerminationAndRestoreDefaultBehaviour = 2,
    }

    impl Default for Mode {
        /// By default we will emulate the default behaviour and abort the process.
        ///
        /// While testing, we will not abort the process.
        fn default() -> Self {
            if cfg!(test) {
                Mode::DeleteTempfilesOnTermination
            } else {
                Mode::DeleteTempfilesOnTerminationAndRestoreDefaultBehaviour
            }
        }
    }

    /// On linux we can handle the actual signal as we know it.
    #[cfg(not(windows))]
    pub(crate) fn cleanup_tempfiles_nix(sig: &libc::siginfo_t) {
        crate::registry::cleanup_tempfiles_signal_safe();
        let restore_original_behaviour = Mode::DeleteTempfilesOnTerminationAndRestoreDefaultBehaviour as usize;
        if MODE.load(std::sync::atomic::Ordering::SeqCst) == restore_original_behaviour {
            signal_hook::low_level::emulate_default_handler(sig.si_signo).ok();
        }
    }

    /// On windows, assume sig-term and emulate sig-term unconditionally.
    #[cfg(windows)]
    pub(crate) fn cleanup_tempfiles_windows() {
        crate::registry::cleanup_tempfiles_signal_safe();
        let restore_original_behaviour = Mode::DeleteTempfilesOnTerminationAndRestoreDefaultBehaviour as usize;
        if MODE.load(std::sync::atomic::Ordering::SeqCst) == restore_original_behaviour {
            signal_hook::low_level::emulate_default_handler(signal_hook::consts::SIGTERM).ok();
        }
    }

    #[cfg(test)]
    mod tests {
        use std::path::Path;

        use crate::{AutoRemove, ContainingDirectory};

        fn filecount_in(path: impl AsRef<Path>) -> usize {
            std::fs::read_dir(path).expect("valid dir").count()
        }

        #[test]
        fn various_termination_signals_remove_tempfiles_unconditionally() -> gix_testtools::TestResult {
            crate::signal::setup(Default::default());
            let dir = tempfile::tempdir()?;
            for sig in signal_hook::consts::TERM_SIGNALS {
                let _tempfile = crate::new(dir.path(), ContainingDirectory::Exists, AutoRemove::Tempfile)?;
                assert_eq!(
                    filecount_in(dir.path()),
                    1,
                    "only one tempfile exists no matter the iteration"
                );
                signal_hook::low_level::raise(*sig)?;
                assert_eq!(
                    filecount_in(dir.path()),
                    0,
                    "the signal triggers removal but won't terminate the process (anymore)"
                );
            }
            Ok(())
        }
    }
}

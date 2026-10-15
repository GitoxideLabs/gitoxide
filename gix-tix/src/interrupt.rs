use std::{
    io,
    sync::{
        Arc, LazyLock,
        atomic::{AtomicBool, Ordering},
    },
};

use gix::{
    Result,
    error::{ResultExt, message},
};

#[derive(Default)]
struct Sources {
    interrupted: Arc<AtomicBool>,
    terminated: Arc<AtomicBool>,
}

static SOURCES: LazyLock<Sources> = LazyLock::new(Sources::default);

pub(crate) struct Guard {
    core: Option<gix::interrupt::Deregister>,
    observers: Vec<signal_hook::SigId>,
}

pub(crate) fn install() -> Result<Guard> {
    let mut guard = Guard {
        core: Some(gix::interrupt::init_handler_default(1)?),
        observers: Vec::new(),
    };
    SOURCES.interrupted.store(false, Ordering::SeqCst);
    SOURCES.terminated.store(false, Ordering::SeqCst);
    let signals = signal_hook::consts::TERM_SIGNALS.iter().copied();
    #[cfg(unix)]
    let signals = signals.chain([signal_hook::consts::SIGHUP]);
    for signal in signals {
        let source = if signal == signal_hook::consts::SIGINT {
            &SOURCES.interrupted
        } else {
            &SOURCES.terminated
        };
        guard.observers.push(
            signal_hook::flag::register(signal, Arc::clone(source))
                .or_raise(|| message!("could not observe signal {signal}"))?,
        );
    }
    Ok(guard)
}

impl Drop for Guard {
    fn drop(&mut self) {
        for observer in self.observers.drain(..) {
            signal_hook::low_level::unregister(observer);
        }
        if let Some(core) = self.core.take()
            && let Err(err) = core.with_reset(true).deregister()
        {
            tracing::warn!(%err, "could not restore termination signal handlers");
        }
        SOURCES.interrupted.store(false, Ordering::SeqCst);
        SOURCES.terminated.store(false, Ordering::SeqCst);
    }
}

pub(crate) fn requested() -> bool {
    if SOURCES.interrupted.load(Ordering::SeqCst) || SOURCES.terminated.load(Ordering::SeqCst) {
        // A signal can race with clearing the foreground program's SIGINT request.
        gix::interrupt::trigger();
    }
    gix::interrupt::is_triggered()
}

pub(crate) fn check() -> io::Result<()> {
    if requested() {
        Err(io::Error::new(io::ErrorKind::Interrupted, "Tix shutdown requested"))
    } else {
        Ok(())
    }
}

pub(crate) fn finish_external() {
    // Ctrl-C belongs to the foreground program; other termination signals still end Tix.
    if SOURCES.interrupted.swap(false, Ordering::SeqCst) && !SOURCES.terminated.load(Ordering::SeqCst) {
        gix::interrupt::reset();
    }
}

#[cfg(test)]
mod tests {
    use gix_testtools::TestResult;

    use super::*;

    #[test]
    fn terminating_signals_survive_external_programs() -> TestResult {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let signals = [signal_hook::consts::SIGTERM].into_iter();
        #[cfg(unix)]
        let signals = signals.chain([signal_hook::consts::SIGQUIT, signal_hook::consts::SIGHUP]);
        for signal in signals {
            let guard = install()?;
            check()?;
            signal_hook::low_level::raise(signal)?;
            assert_eq!(
                check().expect_err("termination requests cancel work").kind(),
                io::ErrorKind::Interrupted,
                "cancellation retains the native interruption kind"
            );
            finish_external();
            assert!(
                requested(),
                "returning from an external program preserves signal {signal}"
            );
            drop(guard);
            assert!(!requested(), "the final guard clears state after terminal restoration");
        }
        Ok(())
    }

    #[test]
    fn foreground_ctrl_c_resumes_tix() -> TestResult {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let _guard = install()?;
        signal_hook::low_level::raise(signal_hook::consts::SIGINT)?;
        assert!(requested(), "SIGINT still requests shutdown while Tix owns input");
        finish_external();
        assert!(!requested(), "foreground-program Ctrl-C is consumed when it returns");
        Ok(())
    }

    #[test]
    fn foreground_ctrl_c_does_not_erase_termination() -> TestResult {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let _inherited = gix::interrupt::init_handler_default(3)?
            .with_reset(true)
            .auto_deregister();
        let _guard = install()?;
        signal_hook::low_level::raise(signal_hook::consts::SIGTERM)?;
        signal_hook::low_level::raise(signal_hook::consts::SIGINT)?;
        finish_external();
        assert!(
            requested(),
            "a later Ctrl-C cannot overwrite a pending termination signal"
        );
        Ok(())
    }

    #[test]
    fn recorded_termination_restores_shared_worker_cancellation() -> TestResult {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let _guard = install()?;
        signal_hook::low_level::raise(signal_hook::consts::SIGTERM)?;
        // Model foreground SIGINT cleanup resetting the flag after a concurrent signal set it.
        gix::interrupt::reset();
        assert!(requested(), "the recorded signal still requests shutdown");
        assert!(
            gix::interrupt::is_triggered(),
            "workers observing only the shared flag must also stop"
        );
        Ok(())
    }

    #[test]
    fn existing_programmatic_interruption_is_preserved() -> TestResult {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        gix::interrupt::trigger();
        let _guard = install()?;
        assert!(requested(), "installation preserves an existing cancellation request");
        finish_external();
        assert!(requested(), "external programs only consume observed SIGINT requests");
        Ok(())
    }
}

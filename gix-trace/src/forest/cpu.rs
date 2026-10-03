use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};

use super::tree::CpuTime;

pub(super) fn initial_time() -> Option<CpuTime> {
    cfg!(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd"
    ))
    .then_some(CpuTime::default())
}

// Shared counters identify spans across registries, which can reuse the same span
// IDs, and accumulate concurrent entries. They do not retain tracing span handles.
#[derive(Clone)]
pub(super) struct Span(Arc<Mutex<Option<CpuTime>>>);

impl Default for Span {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(initial_time())))
    }
}

impl Span {
    pub(super) fn enter(&self) {
        let now = sample();
        if ACTIVE.try_with(|active| active.borrow_mut().enter(self, now)).is_err() {
            self.add(None);
        }
    }

    pub(super) fn exit(&self) {
        let now = sample();
        if ACTIVE.try_with(|active| active.borrow_mut().exit(self, now)).is_err() {
            self.add(None);
        }
    }

    pub(super) fn time(&self) -> Option<CpuTime> {
        *self
            .0
            .lock()
            .expect("CPU accounting does not panic while holding its lock")
    }

    fn add(&self, elapsed: Option<CpuTime>) {
        let mut total = self
            .0
            .lock()
            .expect("CPU accounting does not panic while holding its lock");
        *total = total
            .zip(elapsed)
            .and_then(|(total, elapsed)| total.checked_add(elapsed));
    }
}

thread_local! {
    static ACTIVE: RefCell<State> = RefCell::default();
}

#[derive(Default)]
struct State {
    active: smallvec::SmallVec<[Entry; 8]>,
    previous: Option<CpuTime>,
}

struct Entry {
    span: Span,
    entries: usize,
}

impl State {
    fn advance(&mut self, now: Option<CpuTime>) {
        if let Some(entry) = self.active.last() {
            entry.span.add(
                now.zip(self.previous)
                    .and_then(|(now, previous)| now.checked_sub(previous)),
            );
        }
        self.previous = now;
    }

    fn enter(&mut self, span: &Span, now: Option<CpuTime>) {
        self.advance(now);
        // The registry keeps its current span when an ancestor is re-entered.
        if let Some(entry) = self.active.iter_mut().find(|entry| Arc::ptr_eq(&entry.span.0, &span.0)) {
            entry.entries = entry
                .entries
                .checked_add(1)
                .expect("each entry requires a live span guard");
        } else {
            self.active.push(Entry {
                span: span.clone(),
                entries: 1,
            });
        }
    }

    fn exit(&mut self, span: &Span, now: Option<CpuTime>) {
        self.advance(now);
        let index = self
            .active
            .iter()
            .position(|entry| Arc::ptr_eq(&entry.span.0, &span.0))
            .expect("forest exits a span only after entering it on this thread");
        self.active[index].entries -= 1;
        if self.active[index].entries == 0 {
            self.active.remove(index);
        }
        if self.active.is_empty() {
            self.previous = None;
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd"))]
fn sample() -> Option<CpuTime> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    // SAFETY: the output buffer has the required type and is read only after a
    // successful call. RUSAGE_THREAD reads the calling thread's own counters.
    #[expect(unsafe_code)]
    let usage = unsafe {
        if libc::getrusage(libc::RUSAGE_THREAD, usage.as_mut_ptr()) != 0 {
            return None;
        }
        usage.assume_init()
    };
    Some(CpuTime {
        user: duration(usage.ru_utime.tv_sec, usage.ru_utime.tv_usec)?,
        system: duration(usage.ru_stime.tv_sec, usage.ru_stime.tv_usec)?,
    })
}

#[cfg(target_os = "macos")]
fn sample() -> Option<CpuTime> {
    let mut info = std::mem::MaybeUninit::<libc::thread_basic_info>::uninit();
    let mut count = libc::THREAD_BASIC_INFO_COUNT;
    // SAFETY: this borrows the current live pthread's Mach port without acquiring
    // a send right to release. The buffer matches THREAD_BASIC_INFO and is read
    // only after success with the expected number of initialized integer fields.
    #[expect(unsafe_code)]
    let info = unsafe {
        let thread = libc::pthread_mach_thread_np(libc::pthread_self());
        if libc::thread_info(
            thread,
            libc::THREAD_BASIC_INFO as libc::thread_flavor_t,
            info.as_mut_ptr().cast(),
            &mut count,
        ) != libc::KERN_SUCCESS
            || count != libc::THREAD_BASIC_INFO_COUNT
        {
            return None;
        }
        info.assume_init()
    };
    Some(CpuTime {
        user: duration(info.user_time.seconds, info.user_time.microseconds)?,
        system: duration(info.system_time.seconds, info.system_time.microseconds)?,
    })
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd"
)))]
fn sample() -> Option<CpuTime> {
    None
}

#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd"
))]
fn duration(seconds: impl TryInto<u64>, microseconds: impl TryInto<u32>) -> Option<std::time::Duration> {
    let microseconds = microseconds.try_into().ok()?;
    if microseconds >= 1_000_000 {
        return None;
    }
    Some(std::time::Duration::new(seconds.try_into().ok()?, microseconds * 1_000))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn time(user: u64, system: u64) -> Option<CpuTime> {
        Some(CpuTime {
            user: Duration::from_micros(user),
            system: Duration::from_micros(system),
        })
    }

    fn span() -> Span {
        Span(Arc::new(Mutex::new(time(0, 0))))
    }

    #[test]
    fn nested_spans_and_reentered_ancestors_charge_each_interval_once() {
        let (parent, child) = (span(), span());
        let mut state = State::default();
        state.enter(&parent, time(100, 200));
        state.enter(&child, time(110, 205));
        state.enter(&parent, time(120, 215));
        state.exit(&parent, time(125, 222));
        state.exit(&child, time(140, 230));
        state.exit(&parent, time(160, 240));
        assert_eq!(
            parent.time(),
            time(30, 15),
            "the parent receives only its own execution"
        );
        assert_eq!(
            child.time(),
            time(30, 25),
            "re-entering an ancestor keeps the child current"
        );
        assert!(state.active.is_empty(), "all entries, including re-entry, are removed");
    }

    #[test]
    fn exiting_an_outer_span_keeps_the_inner_span_current() {
        let (parent, child) = (span(), span());
        let mut state = State::default();
        state.enter(&parent, time(0, 0));
        state.enter(&child, time(2, 3));
        state.exit(&parent, time(5, 7));
        state.exit(&child, time(9, 11));
        assert_eq!(
            parent.time(),
            time(2, 3),
            "dropping an outer guard never charges the current child to it"
        );
        assert_eq!(
            child.time(),
            time(7, 8),
            "the child keeps running after the outer guard exits"
        );
    }

    #[test]
    fn parallel_entries_and_migration_use_independent_thread_counters() {
        let shared = span();
        let (mut first, mut second) = (State::default(), State::default());
        first.enter(&shared, time(1, 10));
        second.enter(&shared, time(1_000, 2_000));
        second.exit(&shared, time(1_007, 2_011));
        first.exit(&shared, time(4, 14));
        assert_eq!(
            shared.time(),
            time(10, 15),
            "concurrent entries add CPU time from both threads"
        );

        second.enter(&shared, time(20_000, 30_000));
        second.exit(&shared, time(20_004, 30_005));
        assert_eq!(
            shared.time(),
            time(14, 20),
            "a later poll excludes CPU consumed between entries"
        );
    }

    #[test]
    fn failed_or_backwards_samples_invalidate_only_affected_work() {
        for (start, end) in [
            (time(10, 20), None),
            (None, time(10, 20)),
            (time(10, 20), time(9, 21)),
            (time(10, 20), time(11, 19)),
        ] {
            let (affected, later) = (span(), span());
            let mut state = State::default();
            state.enter(&affected, start);
            state.exit(&affected, end);
            assert_eq!(
                affected.time(),
                None,
                "incomplete counters must not become zero or a partial measurement"
            );
            state.enter(&later, time(100, 200));
            state.exit(&later, time(105, 207));
            assert_eq!(
                later.time(),
                time(5, 7),
                "an independent span can measure after a sampling failure"
            );
        }
    }
}

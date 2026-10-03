#![cfg(feature = "forest-cpu-time")]

use std::sync::mpsc;

use gix_error::{ErrorExt, TestResult, message};
use gix_trace::{
    ForestLayer,
    forest::{Formatter, Tree, printer::Pretty, processor, tree::CpuTime},
};
use tracing_subscriber::{Registry, layer::SubscriberExt};

fn collector() -> (tracing::Dispatch, mpsc::Receiver<Tree>) {
    let (sender, receiver) = mpsc::channel();
    let processor = processor::from_fn(move |tree| {
        sender
            .send(tree)
            .map_err(|err| processor::error(err.0, message("tree receiver dropped").raise()))
    });
    (
        tracing::Dispatch::new(Registry::default().with(ForestLayer::from(processor))),
        receiver,
    )
}

#[test]
fn unentered_spans_distinguish_zero_cpu_from_unavailable() -> TestResult {
    let (dispatch, receiver) = collector();
    tracing::dispatcher::with_default(&dispatch, || {
        let root = tracing::info_span!("unentered");
        let _child = tracing::info_span!(parent: &root, "child");
    });
    let tree = receiver.try_recv()?;
    let expected = cfg!(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd"
    ))
    .then_some(CpuTime::default());
    for span in [tree.span()?, tree.span()?.nodes()[0].span()?] {
        assert_eq!(span.total_cpu_time(), expected, "unentered spans have no CPU work");
        assert_eq!(span.base_cpu_time(), expected, "unentered spans have no own CPU work");
        assert_eq!(span.inner_cpu_time(), expected, "unentered children have no CPU work");
    }
    let rendered = Pretty.fmt(&tree)?;
    assert_eq!(
        rendered.matches("[ user: 0.00ns | sys: 0.00ns ]").count(),
        if expected.is_some() { 2 } else { 0 },
        "both spans display CPU times only when available"
    );
    assert!(
        !rendered.contains("CPU unavailable"),
        "unavailable CPU measurements are omitted rather than labelled"
    );
    Ok(())
}

#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd"
))]
mod supported {
    use super::*;
    use std::{future::Future, io::Read, sync::Barrier, task::Context, thread, time::Duration};

    fn assert_total(span: &gix_trace::forest::tree::Span) -> TestResult {
        let own = span
            .base_cpu_time()
            .expect("the platform supplies per-thread CPU counters");
        let children = span.inner_cpu_time().expect("child CPU measurements are complete");
        assert_eq!(
            span.total_cpu_time(),
            Some(CpuTime {
                user: own.user + children.user,
                system: own.system + children.system,
            }),
            "inclusive CPU time adds own work and child work, including parallel workers"
        );
        Ok(())
    }

    #[test]
    fn syscalls_record_kernel_cpu_and_roll_up_into_the_parent() -> TestResult {
        let (dispatch, receiver) = collector();
        let mut zero = std::fs::File::open("/dev/zero")?;
        tracing::dispatcher::with_default(&dispatch, || -> std::io::Result<()> {
            let _root = tracing::info_span!("root").entered();
            for _ in 0..250_000 {
                zero.read_exact(&mut [0])?;
            }
            let _child = tracing::info_span!("reads").entered();
            // Enough actual system calls to cross the OS accounting resolution.
            for _ in 0..250_000 {
                zero.read_exact(&mut [0])?;
            }
            Ok(())
        })?;
        let tree = receiver.try_recv()?;
        let root = tree.span()?;
        let child = root.nodes()[0].span()?;
        let cpu = child
            .total_cpu_time()
            .expect("the kernel supplies CPU accounting for this thread");
        assert!(cpu.system > Duration::ZERO, "reads consume measurable kernel CPU time");
        assert!(
            root.base_cpu_time().expect("the parent also measures CPU time").system > Duration::ZERO,
            "the parent retains its own system calls in addition to its child's work"
        );
        assert_eq!(
            root.inner_cpu_time(),
            Some(cpu),
            "child CPU time is retained by its parent"
        );
        assert_total(root)?;
        assert_total(child)?;
        let rendered = Pretty.fmt(&tree)?;
        assert_eq!(
            rendered.matches("[ user:").count(),
            2,
            "each span displays user CPU time"
        );
        assert_eq!(
            rendered.matches(" | sys:").count(),
            2,
            "each span displays kernel CPU time"
        );
        Ok(())
    }

    #[test]
    fn simultaneous_entries_and_workers_share_one_completed_tree() -> TestResult {
        let (dispatch, receiver) = collector();
        tracing::dispatcher::with_default(&dispatch, || {
            let root = tracing::info_span!("root");
            let entered = root.enter();
            let ready = Barrier::new(3);
            thread::scope(|scope| {
                for _ in 0..2 {
                    let root = &root;
                    let dispatch = &dispatch;
                    let ready = &ready;
                    scope.spawn(move || {
                        tracing::dispatcher::with_default(dispatch, || {
                            let _parent = root.enter();
                            let _worker = tracing::info_span!("worker").entered();
                            ready.wait();
                        });
                    });
                }
                ready.wait();
                drop(entered);
            });
        });
        let tree = receiver.try_recv()?;
        let root = tree.span()?;
        assert_eq!(root.nodes().len(), 2, "each worker remains a distinct child");
        assert_total(root)?;
        for child in root.nodes() {
            assert_total(child.span()?)?;
        }
        assert!(receiver.try_recv().is_err(), "concurrent entries close a single root");
        Ok(())
    }

    #[test]
    fn future_polls_can_move_between_threads() -> TestResult {
        use tracing::Instrument;

        let (dispatch, receiver) = collector();
        let mut future = tracing::dispatcher::with_default(&dispatch, || {
            let mut first_poll = true;
            Box::pin(
                std::future::poll_fn(move |_| {
                    if std::mem::take(&mut first_poll) {
                        std::task::Poll::Pending
                    } else {
                        std::task::Poll::Ready(())
                    }
                })
                .instrument(tracing::info_span!("future")),
            )
        });
        tracing::dispatcher::with_default(&dispatch, || {
            assert!(
                future
                    .as_mut()
                    .poll(&mut Context::from_waker(std::task::Waker::noop()))
                    .is_pending(),
                "the first thread leaves the future suspended"
            );
        });
        thread::spawn(move || {
            tracing::dispatcher::with_default(&dispatch, move || {
                assert!(
                    future
                        .as_mut()
                        .poll(&mut Context::from_waker(std::task::Waker::noop()))
                        .is_ready(),
                    "another thread completes the future"
                );
            });
        })
        .join()
        .expect("moving a suspended future between threads must not panic");
        assert_total(receiver.try_recv()?.span()?)?;
        Ok(())
    }
}

#![cfg(feature = "forest")]

use std::{error::Error, sync::mpsc, thread, time::Duration};

use gix_error::{ErrorExt, TestResult, message};
use gix_trace::{
    ForestLayer,
    forest::{Formatter, Printer, Processor, Tag, Tree, printer::Pretty, processor},
};
use tracing_subscriber::{Layer, Registry, filter::LevelFilter, layer::SubscriberExt};

fn collector() -> (tracing::Dispatch, mpsc::Receiver<Tree>) {
    let (processor, receiver) = collecting_processor();
    (
        tracing::Dispatch::new(Registry::default().with(ForestLayer::from(processor))),
        receiver,
    )
}

fn collecting_processor() -> (impl Processor, mpsc::Receiver<Tree>) {
    let (sender, receiver) = mpsc::channel();
    let processor = processor::from_fn(move |tree| {
        sender
            .send(tree)
            .map_err(|err| processor::error(err.0, message("tree receiver dropped").raise()))
    });
    (processor, receiver)
}

#[test]
fn in_thread_keeps_nested_workers_in_the_captured_tree() -> Result<(), Box<dyn Error>> {
    let (dispatch, receiver) = collector();
    let work = tracing::dispatcher::with_default(&dispatch, || {
        let _root = gix_trace::coarse!("root");
        gix_trace::in_thread(|| {
            let _worker = gix_trace::coarse!("worker");
            thread::scope(|scope| {
                scope.spawn(gix_trace::in_thread(|| {
                    let _nested = gix_trace::coarse!("nested");
                    gix_trace::info!("finished");
                }));
            });
        })
    });
    assert!(
        matches!(receiver.try_recv(), Err(mpsc::TryRecvError::Empty)),
        "the captured parent keeps the tree open after its original guard has dropped"
    );
    thread::spawn(move || {
        let (other_dispatch, other_receiver) = collector();
        tracing::dispatcher::with_default(&other_dispatch, || {
            let _other = gix_trace::coarse!("other");
            work();
            gix_trace::info!("restored");
        });
        let tree = other_receiver.try_recv().expect("the other root has closed");
        let other = tree.span().expect("the other subscriber receives its own root");
        assert_eq!(other.nodes().len(), 1, "worker spans stay with the captured subscriber");
        assert_eq!(
            other.nodes()[0].event().expect("the only child is an event").message(),
            Some("restored"),
            "the worker's prior subscriber and current span are restored"
        );
    })
    .join()
    .expect("workers finish without panicking");
    let tree = receiver.try_recv()?;
    let root = tree.span()?;
    assert_eq!(root.name(), "root", "workers retain the captured parent");
    assert_eq!(root.nodes().len(), 1, "the worker is the root's only child");
    let worker = root.nodes()[0].span()?;
    assert_eq!(worker.name(), "worker", "unscoped workers inherit the captured span");
    assert_eq!(worker.nodes().len(), 1, "nested workers stay in their immediate parent");
    let nested = worker.nodes()[0].span()?;
    assert_eq!(nested.name(), "nested", "scoped workers inherit the worker span");
    assert_eq!(nested.nodes()[0].event()?.message(), Some("finished"));
    assert!(
        receiver.try_recv().is_err(),
        "all worker output belongs to one completed tree"
    );
    Ok(())
}

#[test]
fn recorded_fields_replace_values_and_fill_empty_fields() -> Result<(), Box<dyn Error>> {
    let (dispatch, receiver) = collector();
    tracing::dispatcher::with_default(&dispatch, || {
        let span = gix_trace::coarse!(
            "updates",
            initial = 1,
            deferred = tracing::field::Empty,
            untouched = "kept"
        );
        span.record("initial", 2)
            .record("initial", 3)
            .record("deferred", "ready");
    });
    let tree = receiver.try_recv()?;
    let fields = tree.span()?.fields();
    assert_eq!(
        fields
            .iter()
            .map(|field| (field.key(), field.value()))
            .collect::<Vec<_>>(),
        [("initial", "3"), ("untouched", "\"kept\""), ("deferred", "\"ready\"")],
        "recording replaces existing values without duplicate keys and appends newly filled fields"
    );
    Ok(())
}

#[test]
fn unentered_spans_format_with_zero_percentages() -> Result<(), Box<dyn Error>> {
    let (dispatch, receiver) = collector();
    tracing::dispatcher::with_default(&dispatch, || {
        let root = tracing::info_span!("unentered");
        let _child = tracing::info_span!(parent: &root, "child");
    });
    let tree = receiver.try_recv()?;
    let rendered = Pretty.fmt(&tree)?;
    assert_eq!(
        rendered.matches("[ 0.00ns | 0.00% ]").count(),
        2,
        "a root and child with no measured activity display finite zero percentages"
    );
    Ok(())
}

#[test]
fn recording_a_value_can_emit_an_event_from_its_debug_formatter() -> Result<(), Box<dyn Error>> {
    struct LogsWhenFormatted;

    impl std::fmt::Debug for LogsWhenFormatted {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            tracing::info!("formatting a recorded value");
            formatter.write_str("recorded")
        }
    }

    let (completed, completion) = mpsc::channel();
    let worker = thread::spawn(move || {
        let (dispatch, receiver) = collector();
        tracing::dispatcher::with_default(&dispatch, || {
            let span = gix_trace::coarse!("root", value = tracing::field::Empty);
            span.record("value", tracing::field::debug(LogsWhenFormatted));
        });
        completed
            .send(receiver.try_recv().expect("the root span has closed"))
            .expect("the test is waiting");
    });
    let tree = completion.recv_timeout(Duration::from_secs(10))?;
    worker.join().expect("recording and formatting must not panic");
    let root = tree.span()?;
    assert_eq!(
        root.fields()[0].value(),
        "recorded",
        "custom formatting supplies the final field value"
    );
    assert_eq!(
        root.nodes()[0].event()?.message(),
        Some("formatting a recorded value"),
        "field formatting can emit an event into the same span without holding its extension lock"
    );
    Ok(())
}

#[test]
fn global_level_filter_after_forest_filters_collection() -> Result<(), Box<dyn Error>> {
    let (processor, receiver) = collecting_processor();
    let subscriber = Registry::default()
        .with(ForestLayer::from(processor))
        .with(LevelFilter::INFO);
    tracing::subscriber::with_default(subscriber, || {
        tracing::trace!("filtered event");
        tracing::info!("retained event");
        drop(tracing::debug_span!("filtered span"));
        drop(tracing::info_span!("retained span"));
    });
    assert_eq!(
        receiver.try_recv()?.event()?.message(),
        Some("retained event"),
        "a global filter after the forest prevents verbose events from reaching the processor"
    );
    assert_eq!(
        receiver.try_recv()?.span()?.name(),
        "retained span",
        "a global filter after the forest prevents verbose spans from reaching the processor"
    );
    assert!(receiver.try_recv().is_err(), "filtered nodes produce no extra trees");
    Ok(())
}

#[test]
fn global_level_filter_before_forest_filters_collection() -> Result<(), Box<dyn Error>> {
    let (processor, receiver) = collecting_processor();
    let subscriber = Registry::default()
        .with(LevelFilter::INFO)
        .with(ForestLayer::from(processor));
    tracing::subscriber::with_default(subscriber, || {
        tracing::trace!("filtered event");
        tracing::info!("retained event");
        drop(tracing::debug_span!("filtered span"));
        drop(tracing::info_span!("retained span"));
    });
    assert_eq!(
        receiver.try_recv()?.event()?.message(),
        Some("retained event"),
        "a global filter before the forest prevents verbose events from reaching the processor"
    );
    assert_eq!(
        receiver.try_recv()?.span()?.name(),
        "retained span",
        "a global filter before the forest prevents verbose spans from reaching the processor"
    );
    assert!(receiver.try_recv().is_err(), "filtered nodes produce no extra trees");
    Ok(())
}

#[test]
fn forest_with_a_filter_skips_hidden_ancestors() -> Result<(), Box<dyn Error>> {
    let (processor, receiver) = collecting_processor();
    let subscriber = Registry::default()
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::sink))
        .with(ForestLayer::from(processor).with_filter(LevelFilter::INFO));
    tracing::subscriber::with_default(subscriber, || {
        let _root = tracing::info_span!("root").entered();
        let hidden = tracing::debug_span!("hidden parent");
        assert!(
            !hidden.is_disabled(),
            "the other layer keeps spans registered even when the forest filters them out"
        );
        let _hidden = hidden.entered();
        let _inner = tracing::trace_span!("hidden inner").entered();
        tracing::debug!("filtered event");
        tracing::info!("promoted event");
        tracing::warn_span!("visible child").in_scope(|| tracing::info!("visible event"));
    });
    let tree = receiver.try_recv()?;
    let root = tree.span()?;
    assert_eq!(root.name(), "root", "the nearest visible ancestor remains the root");
    assert_eq!(
        root.nodes().len(),
        2,
        "hidden spans and events are never collected, but their visible descendants are retained"
    );
    assert_eq!(
        root.nodes()[0].event()?.message(),
        Some("promoted event"),
        "events attach to the nearest visible ancestor across multiple hidden spans"
    );
    let child = root.nodes()[1].span()?;
    assert_eq!(
        child.name(),
        "visible child",
        "completed child spans also skip hidden ancestors"
    );
    assert_eq!(child.nodes().len(), 1, "the visible child retains its own event");
    assert_eq!(
        child.nodes()[0].event()?.message(),
        Some("visible event"),
        "visible descendants retain their visible parent"
    );
    assert!(receiver.try_recv().is_err(), "hidden spans never become separate trees");
    Ok(())
}

#[test]
fn forest_with_a_filter_promotes_children_of_a_hidden_root() -> Result<(), Box<dyn Error>> {
    let (processor, receiver) = collecting_processor();
    let subscriber = Registry::default()
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::sink))
        .with(ForestLayer::from(processor).with_filter(LevelFilter::INFO));
    tracing::subscriber::with_default(subscriber, || {
        tracing::debug_span!("hidden root").in_scope(|| {
            tracing::info!("promoted event");
            tracing::info_span!("promoted span").in_scope(|| tracing::info!("nested event"));
        });
    });
    assert_eq!(
        receiver.try_recv()?.event()?.message(),
        Some("promoted event"),
        "an event with no visible ancestor is processed as a root event"
    );
    let tree = receiver.try_recv()?;
    let root = tree.span()?;
    assert_eq!(
        root.name(),
        "promoted span",
        "a span with no visible ancestor is processed as a root span"
    );
    assert_eq!(root.nodes().len(), 1, "promotion preserves the visible subtree");
    assert_eq!(
        root.nodes()[0].event()?.message(),
        Some("nested event"),
        "the promoted span still owns its event"
    );
    assert!(
        receiver.try_recv().is_err(),
        "the hidden root never reaches the processor"
    );
    Ok(())
}

#[test]
fn unfiltered_forest_after_a_filtered_layer_retains_all_nodes() -> Result<(), Box<dyn Error>> {
    let (processor, receiver) = collecting_processor();
    let subscriber = Registry::default()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::sink)
                .with_filter(LevelFilter::INFO),
        )
        .with(ForestLayer::from(processor));
    tracing::subscriber::with_default(subscriber, || {
        tracing::debug_span!("unfiltered").in_scope(|| tracing::debug!("visible to forest"));
    });
    let tree = receiver.try_recv()?;
    let root = tree.span()?;
    assert_eq!(root.name(), "unfiltered", "another layer's filter does not hide spans");
    assert_eq!(root.nodes().len(), 1, "another layer's filter does not hide events");
    assert_eq!(
        root.nodes()[0].event()?.message(),
        Some("visible to forest"),
        "the unfiltered forest retains events inside spans filtered by an earlier layer"
    );
    assert!(receiver.try_recv().is_err(), "the span produces exactly one tree");
    Ok(())
}

#[test]
fn unfiltered_forest_before_a_filtered_layer_retains_all_nodes() -> Result<(), Box<dyn Error>> {
    let (processor, receiver) = collecting_processor();
    let subscriber = Registry::default().with(ForestLayer::from(processor)).with(
        tracing_subscriber::fmt::layer()
            .with_writer(std::io::sink)
            .with_filter(LevelFilter::INFO),
    );
    tracing::subscriber::with_default(subscriber, || {
        tracing::debug_span!("unfiltered").in_scope(|| tracing::debug!("visible to forest"));
    });
    let tree = receiver.try_recv()?;
    let root = tree.span()?;
    assert_eq!(root.name(), "unfiltered", "another layer's filter does not hide spans");
    assert_eq!(root.nodes().len(), 1, "another layer's filter does not hide events");
    assert_eq!(
        root.nodes()[0].event()?.message(),
        Some("visible to forest"),
        "the unfiltered forest retains events inside spans filtered by a later layer"
    );
    assert!(receiver.try_recv().is_err(), "the span produces exactly one tree");
    Ok(())
}

#[test]
fn dynamic_env_filter_can_enable_a_span_by_its_fields() -> Result<(), Box<dyn Error>> {
    let (processor, receiver) = collecting_processor();
    let subscriber = Registry::default()
        .with(tracing_subscriber::EnvFilter::try_new("[work{enabled=true}]=trace")?)
        .with(ForestLayer::from(processor));
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!("outside before entering");
        tracing::info_span!("work", enabled = true).in_scope(|| tracing::debug!("inside"));
        tracing::info!("outside after exiting");
    });
    let tree = receiver.try_recv()?;
    let root = tree.span()?;
    assert_eq!(root.name(), "work", "the field-matched span becomes the tree root");
    assert_eq!(
        root.nodes().len(),
        1,
        "only events inside the enabled span are collected"
    );
    assert_eq!(
        root.nodes()[0].event()?.message(),
        Some("inside"),
        "a matching span field enables otherwise filtered events in its scope"
    );
    assert!(
        receiver.try_recv().is_err(),
        "events outside the enabled scope are filtered"
    );
    Ok(())
}

#[test]
fn reloading_a_filter_during_field_formatting_keeps_existing_span_state() -> Result<(), Box<dyn Error>> {
    struct ReloadWhenFormatted<F>(F);

    impl<F: Fn()> std::fmt::Debug for ReloadWhenFormatted<F> {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            (self.0)();
            formatter.write_str("filter reloaded")
        }
    }

    let (processor, receiver) = collecting_processor();
    let (filter, handle) = tracing_subscriber::reload::Layer::new(LevelFilter::DEBUG);
    let subscriber = Registry::default().with(filter).with(ForestLayer::from(processor));
    let value = ReloadWhenFormatted(move || {
        handle
            .reload(LevelFilter::INFO)
            .expect("the subscriber remains installed while its span fields are formatted");
    });
    tracing::subscriber::with_default(subscriber, || {
        for _ in 0..2 {
            // The first span is accepted before formatting reloads the filter; the second uses the same callsite.
            drop(tracing::debug_span!("already accepted", value = ?value));
        }
        tracing::info!("after reload");
    });
    let tree = receiver.try_recv()?;
    let root = tree.span()?;
    assert_eq!(
        root.name(),
        "already accepted",
        "reloading cannot remove the state needed by an already-accepted span's close callback"
    );
    assert_eq!(
        root.fields()[0].value(),
        "filter reloaded",
        "field formatting completes even though it changes the filter"
    );
    assert_eq!(
        receiver.try_recv()?.event()?.message(),
        Some("after reload"),
        "events permitted by the reloaded filter still reach the processor"
    );
    assert!(
        receiver.try_recv().is_err(),
        "the reloaded filter rejects subsequent spans at the same callsite"
    );
    Ok(())
}

#[test]
fn formatting_filters_nodes_and_promotes_visible_descendants() -> Result<(), Box<dyn Error>> {
    let (dispatch, receiver) = collector();
    tracing::dispatcher::with_default(&dispatch, || {
        let _root = tracing::info_span!("root").entered();
        tracing::info!("first");
        tracing::debug_span!("hidden parent", private = %"hidden\nfield continuation").in_scope(|| {
            tracing::debug!("hidden\nmessage continuation");
            tracing::trace_span!("hidden inner").in_scope(|| tracing::info!("promoted"));
            tracing::warn_span!("visible child").in_scope(|| tracing::warn!("last"));
        });
        tracing::debug!("hidden trailing event");
    });
    let tree = receiver.try_recv()?;
    let full = Pretty.fmt(&tree)?;
    let filtered = Pretty.with_max_level(LevelFilter::INFO).fmt(&tree)?;
    assert!(
        !filtered.contains("hidden"),
        "omitted span names, fields, and events never render"
    );
    assert!(
        !filtered.contains("continuation"),
        "filtering also omits every line of multiline fields and messages"
    );
    assert_eq!(
        filtered.lines().count(),
        5,
        "hidden branches contribute only their visible descendants"
    );
    assert!(
        filtered.contains("┝━ ｉ [info]: first"),
        "visible earlier siblings use a fork"
    );
    assert!(
        filtered.contains("┝━ ｉ [info]: promoted"),
        "descendants pass through multiple hidden ancestors"
    );
    assert!(
        filtered.contains("┕━ visible child ["),
        "hidden trailing siblings leave no ghost connector"
    );
    assert!(
        filtered.contains("   ┕━ 🚧 [warn]: last"),
        "visible descendants retain their visible parent"
    );
    assert_eq!(
        Pretty.with_max_level(LevelFilter::TRACE).fmt(&tree)?,
        full,
        "the most verbose filter preserves the unfiltered formatter"
    );
    assert!(
        Pretty.with_max_level(LevelFilter::OFF).fmt(&tree)?.is_empty(),
        "OFF suppresses the entire tree"
    );
    assert_eq!(Pretty.fmt(&tree)?, full, "formatting never changes the collected tree");
    Ok(())
}

#[test]
fn filtering_a_root_preserves_its_timing_baseline() -> Result<(), Box<dyn Error>> {
    let (dispatch, receiver) = collector();
    tracing::dispatcher::with_default(&dispatch, || {
        let _root = tracing::debug_span!("hidden root").entered();
        tracing::info_span!("first child").in_scope(|| tracing::info!("first event"));
        tracing::info_span!("second child").in_scope(|| tracing::info!("second event"));
    });
    let tree = receiver.try_recv()?;
    let full = Pretty.fmt(&tree)?;
    let filtered = Pretty.with_max_level(LevelFilter::INFO).fmt(&tree)?;
    assert!(!filtered.contains("hidden root"), "a filtered root is omitted too");
    for name in ["first child", "second child"] {
        let timing = |output: &str| {
            output
                .lines()
                .find_map(|line| line.split_once(name).map(|(_, timing)| timing.to_owned()))
        };
        assert_eq!(
            timing(&filtered),
            timing(&full),
            "promotion retains original full-root percentages"
        );
        assert!(
            !filtered.contains(&format!("━ {name}")),
            "children of a hidden root render at the top level"
        );
    }
    assert_eq!(
        filtered.matches("┕━ ｉ [info]:").count(),
        2,
        "each promoted span still owns its event"
    );
    assert!(
        Pretty.with_max_level(LevelFilter::WARN).fmt(&tree)?.is_empty(),
        "a tree with no visible descendants is empty"
    );
    Ok(())
}

#[test]
fn nested_trees_preserve_fields_and_messages() -> Result<(), Box<dyn Error>> {
    let (dispatch, receiver) = collector();
    tracing::dispatcher::with_default(&dispatch, || {
        let _root = tracing::info_span!("root", operation = "scan").entered();
        tracing::info!(count = 2, "first");
        let _child = tracing::debug_span!("child", object = %"abc").entered();
        tracing::warn!(done = true, "complete");
    });

    let tree = receiver.try_recv()?;
    let root = tree.span()?;
    assert_eq!(root.name(), "root", "the outer span is the tree root");
    assert_eq!(root.level(), tracing::Level::INFO, "span levels survive collection");
    assert_eq!(
        root.fields()
            .iter()
            .map(|field| (field.key(), field.value()))
            .collect::<Vec<_>>(),
        [("operation", "\"scan\"")],
        "span fields retain their debug representation"
    );
    assert_eq!(root.nodes().len(), 2, "the event and child span share one parent");
    let event = root.nodes()[0].event()?;
    assert_eq!(
        event.message(),
        Some("first"),
        "event messages are stored separately from fields"
    );
    assert_eq!(
        event.fields()[0].value(),
        "2",
        "numeric fields keep their text representation"
    );
    assert_eq!(event.tag(), None, "the default layer does not assign explicit tags");
    let child = root.nodes()[1].span()?;
    assert_eq!(child.name(), "child", "nested spans remain attached to their parent");
    assert_eq!(child.fields()[0].value(), "abc", "display fields omit debug quotes");
    assert_eq!(
        child.nodes()[0].event()?.level(),
        tracing::Level::WARN,
        "event levels survive collection"
    );
    assert_eq!(
        root.inner_duration(),
        child.total_duration(),
        "parents sum their child span durations"
    );
    assert!(
        root.total_duration() >= root.inner_duration(),
        "nested duration never exceeds the reported total"
    );
    let expected_event: gix_trace::forest::tree::ExpectedEventError =
        tree.event().expect_err("a span cannot be accessed as an event");
    assert_eq!(
        expected_event.to_string(),
        "Expected an event, found a span",
        "the concrete error describes both the expected and actual node kinds"
    );
    assert!(
        expected_event.source().is_none(),
        "a kind mismatch has no causing error"
    );
    let expected_span: gix_trace::forest::tree::ExpectedSpanError = root.nodes()[0]
        .span()
        .expect_err("an event cannot be accessed as a span");
    assert_eq!(
        expected_span.to_string(),
        "Expected a span, found an event",
        "the concrete error describes both the expected and actual node kinds"
    );
    assert!(expected_span.source().is_none(), "a kind mismatch has no causing error");

    let formatted = Pretty.fmt(&tree)?;
    assert!(
        formatted.contains("┝━ ｉ [info]: first"),
        "non-final children use fork connectors"
    );
    assert!(
        formatted.contains("┕━ child [ "),
        "the final child uses a turn connector"
    );
    assert!(
        formatted.contains("   ┕━ 🚧 [warn]: complete"),
        "nested events keep their indentation"
    );
    assert!(
        receiver.try_recv().is_err(),
        "nested spans do not produce separate root trees"
    );
    Ok(())
}

#[test]
fn explicit_parents_override_context_and_follows_from_is_not_a_parent() -> Result<(), Box<dyn Error>> {
    let (dispatch, receiver) = collector();
    tracing::dispatcher::with_default(&dispatch, || {
        let target = tracing::info_span!("target");
        let _ambient = tracing::info_span!("ambient").entered();
        tracing::info!(parent: None, "detached");
        tracing::info_span!(parent: &target, "child").in_scope(|| {
            tracing::info!(parent: &target, "direct");
            tracing::info!("nested");
        });
        let linked = tracing::info_span!(parent: None, "linked");
        linked.follows_from(target.id());
        linked.in_scope(|| tracing::info!("linked event"));
    });

    let trees: Vec<_> = receiver.try_iter().collect();
    assert_eq!(
        trees.len(),
        4,
        "detached event, linked span, ambient span, and target each complete separately"
    );
    assert_eq!(
        trees[0].event()?.message(),
        Some("detached"),
        "parent: None ignores the current span"
    );
    assert_eq!(
        trees[1].span()?.name(),
        "linked",
        "follows_from leaves an explicitly root span independent"
    );
    assert!(
        trees[2].span()?.nodes().is_empty(),
        "explicit parents bypass the ambient span"
    );
    let target = trees[3].span()?;
    assert_eq!(target.name(), "target", "the explicit parent can remain unentered");
    assert_eq!(
        target.nodes().len(),
        2,
        "the explicit event and completed child attach to the target"
    );
    assert_eq!(
        target.nodes()[0].event()?.message(),
        Some("direct"),
        "explicit event parents override their current child span"
    );
    let child = target.nodes()[1].span()?;
    assert_eq!(child.name(), "child", "explicit span parents override the ambient span");
    assert_eq!(
        child.nodes()[0].event()?.message(),
        Some("nested"),
        "contextual events still use the worker span"
    );
    Ok(())
}

#[test]
fn interleaved_future_polls_keep_independent_trees() -> Result<(), Box<dyn Error>> {
    use std::{
        future::Future,
        task::{Context, Poll, Waker},
    };
    use tracing::Instrument;

    let (dispatch, receiver) = collector();
    tracing::dispatcher::with_default(&dispatch, || {
        let mut futures: Vec<_> = (0..2)
            .map(|client_id| {
                let mut first_poll = true;
                Box::pin(
                    std::future::poll_fn(move |_| {
                        tracing::info!(client_id, "poll");
                        if std::mem::take(&mut first_poll) {
                            Poll::Pending
                        } else {
                            Poll::Ready(())
                        }
                    })
                    .instrument(tracing::info_span!(parent: None, "client", client_id)),
                )
            })
            .collect();
        let mut context = Context::from_waker(Waker::noop());
        for future in &mut futures {
            assert!(
                future.as_mut().poll(&mut context).is_pending(),
                "each future suspends once"
            );
            assert!(
                tracing::Span::current().id().is_none(),
                "a suspended future leaves no span entered between polls"
            );
        }
        assert!(
            receiver.try_recv().is_err(),
            "pending futures keep their own span handles alive"
        );
        for mut future in futures.into_iter().rev() {
            assert!(
                future.as_mut().poll(&mut context).is_ready(),
                "the next poll completes each future"
            );
        }
    });

    let trees: Vec<_> = receiver.try_iter().collect();
    assert_eq!(trees.len(), 2, "each instrumented future completes its own tree");
    for (tree, expected_id) in trees.iter().zip(["1", "0"]) {
        let span = tree.span()?;
        assert_eq!(span.name(), "client", "each operation retains its own root span");
        assert_eq!(
            (span.fields()[0].key(), span.fields()[0].value()),
            ("client_id", expected_id),
            "trees are processed in completion order rather than first-poll order"
        );
        assert_eq!(span.nodes().len(), 2, "both polls are retained in the same span");
        for node in span.nodes() {
            let event = node.event()?;
            assert_eq!(event.message(), Some("poll"), "each poll records an event");
            assert_eq!(
                (event.fields()[0].key(), event.fields()[0].value()),
                ("client_id", expected_id),
                "interleaving polls never mixes fields between clients"
            );
        }
    }
    Ok(())
}

#[test]
fn workers_keep_the_root_open_and_attach_in_completion_order() -> Result<(), Box<dyn Error>> {
    let (dispatch, receiver) = collector();
    tracing::dispatcher::with_default(&dispatch, || {
        let root = tracing::info_span!("root").entered();
        let (ready, readiness) = mpsc::channel();
        let mut releases = Vec::new();
        let mut workers = Vec::new();
        for worker_id in 0..2 {
            let parent = tracing::Span::current();
            let dispatch = tracing::dispatcher::get_default(Clone::clone);
            let ready = ready.clone();
            let (release, wait) = mpsc::channel();
            releases.push(release);
            workers.push(thread::spawn(move || {
                tracing::dispatcher::with_default(&dispatch, move || {
                    let worker = tracing::info_span!(parent: &parent, "worker", worker_id).entered();
                    ready.send(()).expect("the main thread waits for both workers to enter");
                    drop(ready);
                    wait.recv().expect("the main thread must release each worker");
                    tracing::info!("finished");
                    drop(worker);
                    drop(parent);
                });
            }));
        }
        drop(ready);
        for _ in 0..2 {
            readiness
                .recv_timeout(Duration::from_secs(10))
                .expect("both workers must enter before their parent is dropped");
        }
        drop(root);
        assert!(
            receiver.try_recv().is_err(),
            "worker references retain the root after its owner drops it"
        );

        releases[1].send(()).expect("the second worker is waiting");
        workers
            .pop()
            .expect("two workers were spawned")
            .join()
            .expect("the second worker must not panic");
        assert!(
            receiver.try_recv().is_err(),
            "one remaining worker still retains the entire tree"
        );

        releases[0].send(()).expect("the first worker is waiting");
        workers
            .pop()
            .expect("one worker remains")
            .join()
            .expect("the first worker must not panic");
    });

    let tree = receiver.try_recv()?;
    let root = tree.span()?;
    assert_eq!(root.name(), "root", "both threads contribute to the captured root");
    assert_eq!(root.nodes().len(), 2, "each thread gets its own child span");
    for (node, expected_id) in root.nodes().iter().zip(["1", "0"]) {
        let worker = node.span()?;
        assert_eq!(
            worker.fields()[0].value(),
            expected_id,
            "siblings are attached in completion order"
        );
        assert_eq!(
            worker.nodes()[0].event()?.message(),
            Some("finished"),
            "worker events remain inside their own span"
        );
    }
    assert_eq!(
        root.inner_duration(),
        root.nodes()
            .iter()
            .map(|node| node
                .span()
                .expect("all root children are worker spans")
                .total_duration())
            .sum::<std::time::Duration>(),
        "concurrent child durations are aggregated"
    );
    assert!(
        root.total_duration() >= root.inner_duration(),
        "the upstream duration floor is preserved"
    );
    assert!(receiver.try_recv().is_err(), "the root is processed exactly once");
    Ok(())
}

#[test]
fn processor_errors_preserve_native_causes_and_the_tree() -> TestResult {
    use std::io::{Cursor, ErrorKind};

    let (dispatch, receiver) = collector();
    tracing::dispatcher::with_default(&dispatch, || tracing::info!(answer = 42, "retained"));
    let tree = receiver.try_recv()?;
    for (result, expected_kind) in [
        (
            Printer::new()
                .formatter(|_: &Tree| {
                    Err::<String, _>(std::io::Error::new(ErrorKind::BrokenPipe, "formatter unavailable"))
                })
                .process(tree.clone()),
            ErrorKind::BrokenPipe,
        ),
        (
            Printer::new().writer(|| Cursor::new([0u8; 0])).process(tree),
            ErrorKind::WriteZero,
        ),
    ] {
        let err = result.expect_err("both formatting and writing failures return the unprocessed tree");
        let source = err
            .source()
            .and_then(|source| source.downcast_ref::<gix_error::Error>())
            .expect("processor failures retain a gix-error cause");
        assert_eq!(
            source
                .downcast_any_ref::<std::io::Error>()
                .expect("the native I/O error remains available for recovery")
                .kind(),
            expected_kind,
            "converting formatter and writer failures preserves their concrete causes"
        );
        assert_eq!(
            err.to_string(),
            source.to_string(),
            "the processor error displays its cause"
        );
        let event = err.tree.event()?;
        assert_eq!(event.message(), Some("retained"), "failed processing retains the event");
        assert_eq!(
            event.fields()[0].value(),
            "42",
            "failed processing retains event fields"
        );
    }
    Ok(())
}

#[test]
fn processor_errors_preserve_existing_error_context() -> TestResult {
    let (dispatch, receiver) = collector();
    tracing::dispatcher::with_default(&dispatch, || tracing::info!("retained"));
    let processor = Printer::new().formatter(|_: &Tree| {
        Err::<String, _>(
            std::io::Error::from(std::io::ErrorKind::PermissionDenied).and_raise(message("formatter unavailable")),
        )
    });
    let err = processor
        .process(receiver.try_recv()?)
        .expect_err("the formatter returns an error with context");
    let source = err
        .source()
        .and_then(|source| source.downcast_ref::<gix_error::Error>())
        .expect("processor failures retain a gix-error cause");
    assert_eq!(
        source
            .error()
            .downcast_ref::<gix_error::Message>()
            .expect("an existing gix-error is propagated without another wrapper")
            .to_string(),
        "formatter unavailable",
        "the original diagnostic context is preserved"
    );
    assert_eq!(
        source
            .probable_cause()
            .downcast_ref::<std::io::Error>()
            .expect("the contextual error retains its native cause")
            .kind(),
        std::io::ErrorKind::PermissionDenied,
        "the cause below the context remains available for typed recovery"
    );
    Ok(())
}

#[test]
fn a_failed_formatter_passes_the_intact_tree_to_its_fallback() -> Result<(), Box<dyn Error>> {
    let (sender, receiver) = mpsc::channel();
    let primary = Printer::new().formatter(|_: &Tree| Err::<String, _>(std::io::Error::other("formatter unavailable")));
    let fallback = processor::from_fn(move |tree| {
        sender
            .send(tree)
            .map_err(|err| processor::error(err.0, message("tree receiver dropped").raise()))
    });
    let layer = ForestLayer::new(primary.or(fallback), |event: &tracing::Event<'_>| {
        Some(
            Tag::builder()
                .prefix("request")
                .level(*event.metadata().level())
                .build(),
        )
    });
    tracing::subscriber::with_default(Registry::default().with(layer), || tracing::info!("retained"));

    let tree = receiver.try_recv()?;
    let event = tree.event()?;
    assert_eq!(
        event.message(),
        Some("retained"),
        "formatter failures retain the original event"
    );
    assert_eq!(
        event.tag().map(|tag| tag.to_string()),
        Some("request.info".into()),
        "custom tags survive fallback processing"
    );
    assert!(receiver.try_recv().is_err(), "the fallback receives the tree once");
    Ok(())
}

#[test]
fn pretty_event_output_preserves_the_selected_color_mode() -> Result<(), Box<dyn Error>> {
    let (dispatch, receiver) = collector();
    tracing::dispatcher::with_default(&dispatch, || tracing::info!(answer = 42, "hello"));
    let output = Pretty.fmt(&receiver.try_recv()?)?;
    #[cfg(not(feature = "forest-ansi"))]
    let expected = "INFO     ｉ [info]: hello | answer: 42\n";
    #[cfg(feature = "forest-ansi")]
    let expected = "\u{1b}[1;32mINFO    \u{1b}[0m ｉ [info]: hello | \u{1b}[2;37manswer\u{1b}[0m: 42\n";
    assert_eq!(
        output, expected,
        "color mode changes only the level and field key styling"
    );
    Ok(())
}

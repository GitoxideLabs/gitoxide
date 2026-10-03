use std::path::Path;

use gix::{
    Result,
    error::{ResultExt, message},
};
use gix_trace::forest::Tree;
use parking_lot::Mutex;
use rusqlite::params;

use crate::trace::Output;

pub fn subscriber(db_path: impl AsRef<Path>, trace: u8, output: Output) -> Result<tracing::Dispatch> {
    let con = Mutex::new(
        rusqlite::Connection::open(db_path).or_raise(|| message("Could not open the corpus trace database"))?,
    );
    crate::trace::subscriber(
        trace,
        output,
        Some(Box::new(move |tree| {
            let run_id = tree
                .span()
                .ok()
                .filter(|span| span.name() == "run")
                .and_then(|span| span.fields().iter().find(|field| field.key() == "run_id"))
                .and_then(|field| field.value().parse::<super::db::Id>().ok());
            if let Some(run_id) = run_id {
                let json =
                    serde_json::to_string_pretty(&tree_json(tree)).expect("serialization to string always works");
                con.lock()
                    .execute("UPDATE run SET spans_json = ?1 WHERE id = ?2", params![json, run_id])
                    .or_raise(|| message!("Could not store the trace for corpus run {run_id}"))?;
            }
            Ok(())
        })),
    )
}

fn tree_json(tree: &Tree) -> serde_json::Value {
    let fields = |fields: &[gix_trace::forest::tree::Field]| {
        fields
            .iter()
            .map(|field| (field.key().to_owned(), serde_json::Value::from(field.value())))
            .collect::<serde_json::Map<_, _>>()
    };
    match tree {
        Tree::Event(event) => serde_json::json!({
            "Event": {
                "level": event.level().as_str(),
                "fields": fields(event.fields()),
                "message": event.message(),
                "tag": event.tag().map(|tag| tag.to_string()),
            }
        }),
        Tree::Span(span) => serde_json::json!({
            "Span": {
                "level": span.level().as_str(),
                "fields": fields(span.fields()),
                "name": span.name(),
                "nanos_total": span.total_duration().as_nanos(),
                "nanos_nested": span.inner_duration().as_nanos(),
                "nodes": span.nodes().iter().map(tree_json).collect::<Vec<_>>(),
            }
        }),
    }
}

#[cfg(test)]
mod tests {
    use gix::error::TestResult;
    use std::path::Path;

    use crate::{corpus::db, trace::Output};

    #[test]
    fn requested_trace_mode_controls_deferred_format_and_level() -> TestResult<()> {
        let fixture = tempfile::tempdir()?;

        let forest_info = messages(fixture.path(), 1)?;
        assert_eq!(forest_info.len(), 2);
        assert!(
            forest_info.iter().all(|line| line.contains("INFO")),
            "forest INFO output retains only INFO lines"
        );
        assert!(
            forest_info.iter().all(|line| line.contains("\x1b[")),
            "forest output includes ANSI styling"
        );

        let forest_debug = messages(fixture.path(), 2)?;
        assert_eq!(forest_debug.len(), 3);
        assert!(
            forest_debug.iter().any(|line| line.contains("DEBUG")),
            "forest DEBUG output includes DEBUG lines"
        );

        let flat_debug = messages(fixture.path(), 3)?;
        assert_eq!(flat_debug.len(), 3);
        assert!(
            flat_debug.iter().any(|line| line.contains("DEBUG")),
            "flat DEBUG output includes DEBUG lines"
        );
        assert!(
            flat_debug.iter().all(|line| line.contains("\x1b[")),
            "flat output includes ANSI styling"
        );
        assert!(flat_debug.iter().any(|line| line.contains("close")));
        assert!(
            !flat_debug.iter().any(|line| line.contains("TRACE")),
            "flat DEBUG output excludes TRACE lines"
        );

        let flat_trace = messages(fixture.path(), 4)?;
        assert_eq!(flat_trace.len(), 4);
        assert!(
            flat_trace.iter().any(|line| line.contains("TRACE")),
            "flat TRACE output includes TRACE lines"
        );
        Ok(())
    }

    #[test]
    fn display_modes_do_not_filter_the_stored_trace() -> TestResult<()> {
        let fixture = tempfile::tempdir()?;
        for trace in 0..=4 {
            let db_path = fixture.path().join(format!("stored-{trace}.db"));
            let connection = db::create(&db_path)?;
            connection.execute("INSERT INTO run (insertion_time) VALUES (0)", [])?;
            let run_id = u32::try_from(connection.last_insert_rowid()).expect("test run id fits in u32");

            let output = Output::default();
            let dispatch = super::subscriber(&db_path, trace, output.clone())?;
            tracing::dispatcher::with_default(&dispatch, || {
                tracing::info_span!("run", run_id).in_scope(|| {
                    tracing::debug!("stored debug event\nprivate continuation");
                    tracing::trace!("stored trace event");
                    tracing::info!("visible info event");
                });
            });

            let stored: String =
                connection.query_row("SELECT spans_json FROM run WHERE id = ?1", [run_id], |row| row.get(0))?;
            for message in [
                "stored debug event",
                "private continuation",
                "stored trace event",
                "visible info event",
            ] {
                assert!(
                    stored.contains(message),
                    "display mode {trace} leaves storage complete: {message}"
                );
            }
            let output = output.lock().expect("trace output lock is not poisoned");
            let rendered = String::from_utf8_lossy(&output);
            assert_eq!(rendered.is_empty(), trace == 0, "disabled display remains silent");
            for (message, visible) in [
                ("visible info event", trace != 0),
                ("stored debug event", trace >= 2),
                ("private continuation", trace >= 2),
                ("stored trace event", trace == 4),
            ] {
                assert_eq!(
                    rendered.contains(message),
                    visible,
                    "display mode {trace} filters complete events: {message}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn delayed_run_spans_keep_their_ids_and_unrelated_roots_only_display() -> TestResult<()> {
        let fixture = tempfile::tempdir()?;
        let db_path = fixture.path().join("run-ids.db");
        let connection = db::create(&db_path)?;
        connection.execute("INSERT INTO run (insertion_time) VALUES (0), (0)", [])?;
        let second_run_id = u32::try_from(connection.last_insert_rowid()).expect("test run id fits in u32");
        let first_run_id = second_run_id - 1;
        let output = Output::default();
        let dispatch = super::subscriber(&db_path, 1, output.clone())?;
        {
            let _guard = tracing::dispatcher::set_default(&dispatch);
            let first = tracing::info_span!("run", run_id = first_run_id);
            first.in_scope(|| tracing::info!("first run"));
            tracing::info_span!("run", run_id = second_run_id).in_scope(|| tracing::info!("second run"));
            drop(first);
            tracing::info!("unrelated event");
            tracing::info_span!("unrelated span", run_id = second_run_id)
                .in_scope(|| tracing::info!("unrelated span event"));
            tracing::info_span!("run").in_scope(|| tracing::info!("run without id"));
        }

        for (run_id, message) in [(first_run_id, "first run"), (second_run_id, "second run")] {
            let stored: String =
                connection.query_row("SELECT spans_json FROM run WHERE id = ?1", [run_id], |row| row.get(0))?;
            let stored: serde_json::Value = serde_json::from_str(&stored)?;
            assert_eq!(
                stored["Span"]["fields"]["run_id"],
                run_id.to_string(),
                "completed run spans retain their own ID regardless of closure order"
            );
            assert_eq!(
                stored["Span"]["nodes"][0]["Event"]["message"], message,
                "unrelated roots do not overwrite a run's stored tree"
            );
        }
        let output = output.lock().expect("trace output lock is not poisoned");
        let rendered = String::from_utf8_lossy(&output);
        for message in [
            "first run",
            "second run",
            "unrelated event",
            "unrelated span event",
            "run without id",
        ] {
            assert!(
                rendered.contains(message),
                "all requested trace output remains visible: {message}"
            );
        }
        Ok(())
    }

    #[test]
    fn concurrent_run_roots_share_a_dispatch_without_mixing_storage_or_output() -> TestResult<()> {
        let fixture = tempfile::tempdir()?;
        for trace in [0, 1, 3, 4] {
            let db_path = fixture.path().join(format!("concurrent-{trace}.db"));
            let connection = db::create(&db_path)?;
            let mut run_ids = Vec::new();
            for _ in 0..4 {
                connection.execute("INSERT INTO run (insertion_time) VALUES (0)", [])?;
                run_ids.push(db::Id::try_from(connection.last_insert_rowid())?);
            }
            let output = Output::default();
            let dispatch = super::subscriber(&db_path, trace, output.clone())?;
            let barrier = std::sync::Barrier::new(run_ids.len());
            std::thread::scope(|scope| {
                for &run_id in &run_ids {
                    let (dispatch, barrier) = (&dispatch, &barrier);
                    scope.spawn(move || {
                        let _guard = tracing::dispatcher::set_default(dispatch);
                        tracing::info_span!("run", run_id).in_scope(|| {
                            barrier.wait();
                            tracing::info!("run event {run_id}\ncontinuation {run_id}");
                        });
                    });
                }
            });
            let output = output.lock().expect("trace output lock is not poisoned");
            let rendered = String::from_utf8_lossy(&output);
            let lines: Vec<_> = rendered.lines().collect();
            assert_eq!(
                rendered.is_empty(),
                trace == 0,
                "concurrent storage does not enable display"
            );
            for run_id in run_ids {
                let stored: String =
                    connection.query_row("SELECT spans_json FROM run WHERE id = ?1", [run_id], |row| row.get(0))?;
                let stored: serde_json::Value = serde_json::from_str(&stored)?;
                let event = format!("run event {run_id}");
                let continuation = format!("continuation {run_id}");
                assert_eq!(
                    stored["Span"]["fields"]["run_id"],
                    run_id.to_string(),
                    "concurrent roots keep their own IDs"
                );
                assert_eq!(
                    stored["Span"]["nodes"][0]["Event"]["message"],
                    format!("{event}\n{continuation}"),
                    "each stored tree contains only its own event",
                );
                if trace != 0 {
                    let index = lines
                        .iter()
                        .position(|line| line.contains(&event))
                        .expect("each event is displayed");
                    assert!(
                        lines.get(index + 1).is_some_and(|line| line.contains(&continuation)),
                        "each multiline event is written contiguously in display mode {trace}",
                    );
                }
            }
        }
        Ok(())
    }

    fn messages(root: &Path, trace: u8) -> TestResult<Vec<String>> {
        let db_path = root.join(format!("trace-{trace}.db"));
        drop(db::create(&db_path)?);
        let output = Output::default();
        let dispatch = super::subscriber(&db_path, trace, output.clone())?;
        {
            let _guard = tracing::dispatcher::set_default(&dispatch);
            tracing::info_span!("root").in_scope(|| {
                tracing::info!("info event");
                tracing::debug!("debug event");
                tracing::trace!("trace event");
            });
        }
        let output = output.lock().expect("trace output lock is not poisoned");
        Ok(String::from_utf8_lossy(&output)
            .lines()
            .map(ToOwned::to_owned)
            .collect())
    }
}

#[cfg(test)]
mod serialization_tests {
    use super::*;
    use gix::error::{ErrorExt, TestResult, message};
    use gix_trace::{
        ForestLayer,
        forest::{Tag, processor},
    };
    use tracing_subscriber::layer::SubscriberExt;

    #[test]
    fn serialization_preserves_the_corpus_json_shape() -> TestResult<()> {
        let (sender, receiver) = std::sync::mpsc::channel();
        let processor = processor::from_fn(move |tree| {
            sender
                .send(tree)
                .map_err(|err| processor::error(err.0, message("tree receiver dropped").raise()))
        });
        let layer = ForestLayer::new(processor, |event: &tracing::Event<'_>| {
            (event.metadata().target() == "tagged").then(|| {
                Tag::builder()
                    .prefix("request")
                    .level(*event.metadata().level())
                    .build()
            })
        });
        tracing::subscriber::with_default(tracing_subscriber::Registry::default().with(layer), || {
            let root = tracing::info_span!("root", initial = 1, deferred = tracing::field::Empty);
            root.record("initial", 2).record("deferred", "ready");
            root.in_scope(|| {
                tracing::info!(count = 2, "complete");
                let _child = tracing::debug_span!("child").entered();
                tracing::warn!(target: "tagged", "retrying");
            });
        });
        let tree = receiver.try_recv()?;
        let root = tree.span()?;
        let child = root.nodes()[1].span()?;
        let json = serde_json::to_string_pretty(&tree_json(&tree))?;
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&json)?,
            serde_json::json!({
                "Span": {
                    "level": "INFO",
                    "fields": {"initial": "2", "deferred": "\"ready\""},
                    "name": "root",
                    "nanos_total": root.total_duration().as_nanos(),
                    "nanos_nested": child.total_duration().as_nanos(),
                    "nodes": [
                        {"Event": {
                            "level": "INFO",
                            "fields": {"count": "2"},
                            "message": "complete",
                            "tag": null
                        }},
                        {"Span": {
                            "level": "DEBUG",
                            "fields": {},
                            "name": "child",
                            "nanos_total": child.total_duration().as_nanos(),
                            "nanos_nested": 0,
                            "nodes": [{"Event": {
                                "level": "WARN",
                                "fields": {},
                                "message": "retrying",
                                "tag": "request.warn"
                            }}]
                        }}
                    ]
                }
            }),
            "tree variants, final field values, tags, and nanosecond durations retain the stored schema"
        );
        Ok(())
    }
}

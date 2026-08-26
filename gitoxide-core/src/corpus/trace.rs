use std::{
    io,
    path::Path,
    sync::{
        Arc, Mutex as StdMutex,
        atomic::{AtomicU32, Ordering},
    },
};

use parking_lot::Mutex;
use rusqlite::params;
use tracing_forest::tree::Tree;
use tracing_subscriber::{Layer, filter::LevelFilter, fmt::MakeWriter, layer::SubscriberExt};

type TraceOutput = Arc<StdMutex<Vec<u8>>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TraceFormat {
    Forest,
    Flat,
}

fn trace_settings(trace: u8) -> anyhow::Result<Option<(TraceFormat, LevelFilter)>> {
    Ok(match trace {
        0 => None,
        1 => Some((TraceFormat::Forest, LevelFilter::INFO)),
        2 => Some((TraceFormat::Forest, LevelFilter::DEBUG)),
        3 => Some((TraceFormat::Flat, LevelFilter::DEBUG)),
        4 => Some((TraceFormat::Flat, LevelFilter::TRACE)),
        _ => anyhow::bail!("trace level must be between zero and four"),
    })
}

pub fn override_thread_subscriber(
    db_path: impl AsRef<Path>,
    trace: u8,
    output: TraceOutput,
) -> anyhow::Result<(tracing::subscriber::DefaultGuard, Arc<AtomicU32>)> {
    let settings = trace_settings(trace)?;
    let current_id = Arc::new(AtomicU32::default());
    let forest_level = settings.and_then(|(format, level)| (format == TraceFormat::Forest).then_some(level));
    let processor = tracing_forest::Printer::new()
        .writer(TraceWriter(output.clone()))
        .formatter(StoreTreeToDb {
            con: Arc::new(Mutex::new(rusqlite::Connection::open(&db_path)?)),
            run_id: current_id.clone(),
            display_level: forest_level,
        });
    let forest = tracing_forest::ForestLayer::from(processor);
    let guard = match settings {
        None | Some((TraceFormat::Forest, _)) => {
            tracing::subscriber::set_default(tracing_subscriber::registry().with(forest))
        }
        Some((TraceFormat::Flat, level)) => tracing::subscriber::set_default(
            tracing_subscriber::registry().with(forest).with(
                tracing_subscriber::fmt::layer()
                    .with_ansi(true)
                    .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
                    .with_writer(TraceWriter(output))
                    .with_filter(level),
            ),
        ),
    };
    Ok((guard, current_id))
}

#[derive(Clone)]
struct TraceWriter(TraceOutput);

impl<'a> MakeWriter<'a> for TraceWriter {
    type Writer = TraceWriteGuard<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        TraceWriteGuard(self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner))
    }
}

struct TraceWriteGuard<'a>(std::sync::MutexGuard<'a, Vec<u8>>);

impl io::Write for TraceWriteGuard<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub struct StoreTreeToDb {
    con: Arc<Mutex<rusqlite::Connection>>,
    run_id: Arc<AtomicU32>,
    display_level: Option<LevelFilter>,
}

impl tracing_forest::printer::Formatter for StoreTreeToDb {
    type Error = rusqlite::Error;

    fn fmt(&self, tree: &Tree) -> Result<String, Self::Error> {
        let rendered = self.display_level.and_then(|level| filtered_forest(tree, level));
        // TODO: wait for new release of `tracing-forest` and load the ID from span fields.
        let json = serde_json::to_string_pretty(&tree).expect("serialization to string always works");
        let run_id = self.run_id.load(Ordering::SeqCst);
        self.con
            .lock()
            .execute("UPDATE run SET spans_json = ?1 WHERE id = ?2", params![json, run_id])?;
        Ok(rendered.unwrap_or_default())
    }
}

fn filtered_forest(tree: &Tree, level: LevelFilter) -> Option<String> {
    use tracing_forest::Formatter;

    let tree = tracing_forest::printer::Pretty.fmt(tree).ok()?;
    let mut out = String::new();
    // ponytail: filter Pretty's stable level prefix until tracing-forest can host two forest layers safely.
    for line in tree.lines().filter(|line| {
        let level_prefix = line
            .strip_prefix("\x1b[")
            .and_then(|line| line.split_once('m').map(|(_, line)| line))
            .unwrap_or(line);
        match level {
            LevelFilter::INFO => !level_prefix.starts_with("DEBUG") && !level_prefix.starts_with("TRACE"),
            LevelFilter::DEBUG => !level_prefix.starts_with("TRACE"),
            _ => true,
        }
    }) {
        out.push_str(line);
        out.push('\n');
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{TraceFormat, TraceOutput, trace_settings};
    use crate::corpus::db;
    use tracing_subscriber::filter::LevelFilter;

    #[test]
    fn trace_repetitions_choose_format_and_level() -> anyhow::Result<()> {
        assert_eq!(trace_settings(0)?, None);
        assert_eq!(trace_settings(1)?, Some((TraceFormat::Forest, LevelFilter::INFO)));
        assert_eq!(trace_settings(2)?, Some((TraceFormat::Forest, LevelFilter::DEBUG)));
        assert_eq!(trace_settings(3)?, Some((TraceFormat::Flat, LevelFilter::DEBUG)));
        assert_eq!(trace_settings(4)?, Some((TraceFormat::Flat, LevelFilter::TRACE)));
        assert_eq!(
            trace_settings(5)
                .expect_err("trace output has only four levels")
                .to_string(),
            "trace level must be between zero and four"
        );
        Ok(())
    }

    #[test]
    fn requested_trace_mode_controls_deferred_format_and_level() -> anyhow::Result<()> {
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
    fn forest_output_does_not_filter_the_stored_trace() -> anyhow::Result<()> {
        let fixture = tempfile::tempdir()?;
        let db_path = fixture.path().join("stored.db");
        let connection = db::create(&db_path)?;
        connection.execute("INSERT INTO run (insertion_time) VALUES (0)", [])?;
        let run_id = u32::try_from(connection.last_insert_rowid()).expect("test run id fits in u32");
        drop(connection);

        {
            let (_guard, current_id) = super::override_thread_subscriber(&db_path, 1, TraceOutput::default())?;
            current_id.store(run_id, std::sync::atomic::Ordering::SeqCst);
            tracing::info_span!("root").in_scope(|| tracing::debug!("stored debug event"));
        }

        let connection = rusqlite::Connection::open(db_path)?;
        let stored: String =
            connection.query_row("SELECT spans_json FROM run WHERE id = ?1", [run_id], |row| row.get(0))?;
        assert!(
            stored.contains("stored debug event"),
            "display filtering leaves storage complete"
        );
        Ok(())
    }

    fn messages(root: &Path, trace: u8) -> anyhow::Result<Vec<String>> {
        let db_path = root.join(format!("trace-{trace}.db"));
        drop(db::create(&db_path)?);
        let output = TraceOutput::default();
        {
            let (_guard, _current_id) = super::override_thread_subscriber(&db_path, trace, output.clone())?;
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

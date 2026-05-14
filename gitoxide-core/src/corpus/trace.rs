use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
};

use gix::progress::DoOrDiscard;
use parking_lot::Mutex;
use rusqlite::params;
use tracing_forest::tree::Tree;
use tracing_subscriber::layer::SubscriberExt;

type ProgressItem = DoOrDiscard<gix::progress::prodash::tree::Item>;

pub fn override_thread_subscriber(
    db_path: impl AsRef<Path>,
    progress: Option<ProgressItem>,
    reverse_lines: bool,
) -> anyhow::Result<(tracing::subscriber::DefaultGuard, Arc<AtomicU32>)> {
    let current_id = Arc::new(AtomicU32::default());
    let processor = tracing_forest::Printer::new().formatter(StoreTreeToDb {
        con: Arc::new(Mutex::new(rusqlite::Connection::open(&db_path)?)),
        run_id: current_id.clone(),
        progress: progress.map(Mutex::new),
        reverse_lines,
    });
    let subscriber = tracing_subscriber::Registry::default().with(tracing_forest::ForestLayer::from(processor));
    let guard = tracing::subscriber::set_default(subscriber);
    Ok((guard, current_id))
}

pub struct StoreTreeToDb {
    con: Arc<Mutex<rusqlite::Connection>>,
    run_id: Arc<AtomicU32>,
    progress: Option<Mutex<ProgressItem>>,
    reverse_lines: bool,
}

impl tracing_forest::printer::Formatter for StoreTreeToDb {
    type Error = rusqlite::Error;

    fn fmt(&self, tree: &Tree) -> Result<String, Self::Error> {
        if let Some((progress, tree)) = self
            .progress
            .as_ref()
            .map(Mutex::lock)
            .zip(tracing_forest::printer::Pretty.fmt(tree).ok())
        {
            use gix::Progress;
            if self.reverse_lines {
                for line in tree.lines().rev() {
                    progress.info(line.into());
                }
            } else {
                for line in tree.lines() {
                    progress.info(line.into());
                }
            }
        }
        // TODO: wait for new release of `tracing-forest` and load the ID from span fields.
        let json = serde_json::to_string_pretty(&tree).expect("serialization to string always works");
        let run_id = self.run_id.load(Ordering::SeqCst);
        self.con
            .lock()
            .execute("UPDATE run SET spans_json = ?1 WHERE id = ?2", params![json, run_id])?;
        Ok(String::new())
    }
}

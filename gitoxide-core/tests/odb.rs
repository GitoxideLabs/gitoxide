use std::sync::atomic::Ordering;

use gitoxide_core::{OutputFormat, repository::odb};
use gix::Count;
use gix_testtools::TestResult;

#[test]
fn extra_header_lookup_visits_all_packed_chunks_with_one_or_multiple_threads() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("odb-statistics.sh")?;
    for threads in [1, 2] {
        let repo = gix::open_opts(fixture.join("repo.git"), gix::open::Options::isolated())?;
        let root = gix::progress::tree::Root::new();
        let progress = root.add_child("statistics");
        let counter = progress.counter();
        let mut out = Vec::new();
        odb::statistics(
            repo,
            progress,
            &mut out,
            Vec::new(),
            odb::statistics::Options {
                format: OutputFormat::Json,
                thread_limit: Some(threads),
                extra_header_lookup: true,
            },
        )?;
        let stats: serde_json::Value = serde_json::from_slice(&out)?;
        assert_eq!(
            stats["total_objects"], 2_001,
            "every fixture object contributes to the total with {threads} threads"
        );
        assert_eq!(stats["packed_objects"], 2_001, "all fixture objects are packed");
        assert_eq!(
            stats["loose_objects"], 0,
            "no loose objects bypass packed-header lookup"
        );
        assert_eq!(stats["blobs"], 2_001, "every packed blob contributes to the statistics");
        assert_eq!(
            counter.load(Ordering::Relaxed),
            2_001,
            "the cold pass visits two full chunks and the final partial chunk with {threads} threads"
        );
    }
    Ok(())
}

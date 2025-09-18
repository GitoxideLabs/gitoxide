use gix_blame::{file, Options};
use std::path::PathBuf;

fn fixture_path() -> gix_testtools::Result<PathBuf> {
    gix_testtools::scripted_fixture_read_only("make_blame_repo.sh")
}

#[allow(unused)]
fn simple_performance_test() -> gix_testtools::Result<()> {
    let worktree_path = fixture_path()?;

    let git_dir = worktree_path.join(".git");
    let odb = gix_odb::at(git_dir.join("objects"))?;
    let store = gix_ref::file::Store::at(
        git_dir.clone(),
        gix_ref::store::init::Options {
            write_reflog: gix_ref::store::WriteReflog::Disable,
            ..Default::default()
        },
    );

    let mut reference = gix_ref::file::Store::find(&store, "HEAD")?;
    use gix_ref::file::ReferenceExt;
    let head_id = reference.peel_to_id_in_place(&store, &odb)?;

    let index = gix_index::File::at(git_dir.join("index"), gix_hash::Kind::Sha1, false, Default::default())?;
    let stack = gix_worktree::Stack::from_state_and_ignore_case(
        worktree_path.clone(),
        false,
        gix_worktree::stack::State::AttributesAndIgnoreStack {
            attributes: Default::default(),
            ignore: Default::default(),
        },
        &index,
        index.path_backing(),
    );

    let capabilities = gix_fs::Capabilities::probe(&git_dir);
    let mut resource_cache = gix_diff::blob::Platform::new(
        Default::default(),
        gix_diff::blob::Pipeline::new(
            gix_diff::blob::pipeline::WorktreeRoots {
                old_root: None,
                new_root: None,
            },
            gix_filter::Pipeline::new(Default::default(), Default::default()),
            vec![],
            gix_diff::blob::pipeline::Options {
                large_file_threshold_bytes: 0,
                fs: capabilities,
            },
        ),
        gix_diff::blob::pipeline::Mode::ToGit,
        stack,
    );

    let source_file_name = "simple.txt";

    // Run multiple iterations to get stable measurements
    let iterations = 50;
    let mut durations_without = Vec::new();
    let mut durations_with_empty = Vec::new();

    // Test without ignore revisions (zero-cost path)
    for _ in 0..5 {
        let start = std::time::Instant::now();
        for _ in 0..iterations {
            let _result = file(
                &odb,
                head_id,
                None,
                &mut resource_cache,
                source_file_name.into(),
                Options::default(),
            )?;
        }
        durations_without.push(start.elapsed());
    }

    // Test with empty ignore revisions (should be similar performance)
    for _ in 0..5 {
        let start = std::time::Instant::now();
        for _ in 0..iterations {
            let _result = file(
                &odb,
                head_id,
                None,
                &mut resource_cache,
                source_file_name.into(),
                Options::default().with_ignored_revisions(std::iter::empty()),
            )?;
        }
        durations_with_empty.push(start.elapsed());
    }

    // Calculate averages
    let avg_without = durations_without
        .iter()
        .map(std::time::Duration::as_nanos)
        .sum::<u128>()
        / durations_without.len() as u128;
    let avg_with_empty = durations_with_empty
        .iter()
        .map(std::time::Duration::as_nanos)
        .sum::<u128>()
        / durations_with_empty.len() as u128;

    println!(
        "Performance comparison (averaged over {} runs):",
        durations_without.len()
    );
    println!(
        "Without ignore revisions: {:?}",
        std::time::Duration::from_nanos(avg_without as u64)
    );
    println!(
        "With empty ignore revisions: {:?}",
        std::time::Duration::from_nanos(avg_with_empty as u64)
    );

    let overhead = (avg_with_empty as f64 / avg_without as f64 - 1.0) * 100.0;
    println!("Overhead: {overhead:.2}%");

    // Assert that overhead is reasonable (less than 20% to account for system variability and measurement noise)
    // The key point is that it's not 100%+ overhead, showing the abstraction has reasonable cost characteristics
    assert!(
        overhead.abs() < 20.0,
        "Overhead is outside reasonable bounds: {overhead:.2}%"
    );

    // Ensure we're not seeing massive degradation (within 20% either direction)
    let overhead_ratio = avg_with_empty as f64 / avg_without as f64;
    assert!(
        overhead_ratio > 0.8 && overhead_ratio < 1.2,
        "Performance difference is too large: {:.2}%",
        (overhead_ratio - 1.0) * 100.0
    );

    Ok(())
}
// Windows CI has noisy timers/scheduler; perf thresholds are flaky there.
// Keep this perf guard active on Unix; skip on Windows.
#[cfg_attr(windows, ignore = "unstable perf threshold on Windows CI")]
#[test]
fn zero_cost_abstraction_verification() {
    simple_performance_test().expect("Performance test should pass");
}

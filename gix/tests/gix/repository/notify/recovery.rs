use std::time::Instant;

use gix::{
    error::{ErrorExt, message},
    notify::{CoverageStatus, Options, WorktreeCoverage},
};
use gix_error::TestResult;

#[test]
fn coverage_status_distinguishes_inactivity_retry_and_verification() -> TestResult {
    let repo = crate::basic_repo()?;
    let open = || gix::open_opts(repo.git_dir(), crate::restricted());
    let mut inactive = repo.monitor(Options {
        metadata: false,
        worktree_coverage: None,
        ..Options::default()
    })?;
    assert_eq!(
        inactive.coverage_status(),
        CoverageStatus::Inactive,
        "disabled subscriptions do not need initialization"
    );
    assert_eq!(
        inactive
            .service(Instant::now(), || panic!("inactive coverage does not reopen"))
            .coverage,
        CoverageStatus::Inactive,
        "the inactive outcome agrees with the monitor"
    );

    let mut monitor = repo.monitor(Options {
        worktree_coverage: Some(WorktreeCoverage::All),
        ..Options::default()
    })?;
    assert_eq!(
        monitor.coverage_status(),
        CoverageStatus::Updating,
        "initial installation is not recovery from a failure"
    );
    let now = Instant::now();
    let initial = monitor.service(now, open);
    assert!(
        initial.errors.is_empty(),
        "initial registration succeeds: {:?}",
        initial.errors
    );
    assert_eq!(
        initial.coverage,
        CoverageStatus::Updating,
        "new registrations need verification"
    );
    let verified = monitor.service(now, open);
    assert!(
        verified.errors.is_empty(),
        "verification succeeds: {:?}",
        verified.errors
    );
    assert_eq!(
        verified.coverage,
        CoverageStatus::Healthy,
        "verified coverage is healthy"
    );

    monitor.rescan();
    let now = Instant::now();
    let failed = monitor.service(now, || Err(message("deliberate discovery failure").raise()));
    assert_eq!(failed.errors.len(), 1, "the discovery failure is reported");
    let retry = now + monitor.options().retry_interval;
    let recovering = CoverageStatus::Recovering { retry_at: Some(retry) };
    assert_eq!(failed.coverage, recovering, "failure exposes its retry deadline");

    let quiet = monitor.service(now, || panic!("retry is not due"));
    assert!(quiet.errors.is_empty(), "backoff does not repeat the error");
    assert_eq!(quiet.coverage, recovering, "quiet backoff retains recovery status");

    let repaired = monitor.service(retry, open);
    assert!(repaired.errors.is_empty(), "repair succeeds: {:?}", repaired.errors);
    assert_eq!(
        repaired.coverage,
        CoverageStatus::Updating,
        "repair still requires verification"
    );

    let verified = monitor.service(retry, open);
    assert!(
        verified.errors.is_empty(),
        "repair verification succeeds: {:?}",
        verified.errors
    );
    assert_eq!(
        verified.coverage,
        CoverageStatus::Healthy,
        "verification certifies recovery"
    );
    assert_eq!(
        monitor.coverage_status(),
        verified.coverage,
        "the getter agrees with the outcome"
    );
    Ok(())
}

#[test]
fn shared_metadata_does_not_prevent_linked_coverage_from_becoming_healthy() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_notify_worktrees.sh")?;
    let repo = gix::open_opts(fixture.join("main"), crate::restricted())?;
    let mut monitor = repo.monitor(Options {
        all_worktrees: true,
        worktree_coverage: Some(WorktreeCoverage::All),
        ..Options::default()
    })?;
    let now = Instant::now();
    let mut coverage = CoverageStatus::Updating;
    for _ in 0..3 {
        let outcome = monitor.service(now, || gix::open_opts(repo.git_dir(), crate::restricted()));
        assert!(
            outcome.errors.is_empty(),
            "linked registration succeeds: {:?}",
            outcome.errors
        );
        coverage = outcome.coverage;
    }
    assert_eq!(
        monitor.other_worktrees().count(),
        2,
        "both linked checkouts receive coverage"
    );
    assert_eq!(
        coverage,
        CoverageStatus::Healthy,
        "child metadata coverage is supplied by the parent"
    );
    assert_eq!(
        monitor.coverage_status(),
        coverage,
        "the getter agrees with the outcome for linked coverage"
    );
    Ok(())
}

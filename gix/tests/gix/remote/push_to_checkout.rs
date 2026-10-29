use std::sync::atomic::AtomicBool;

use gix::{
    bstr::ByteSlice,
    push::{Lease, Options, Rejection, Status, Update},
};
use gix_testtools::TestResult;

#[test]
fn push_to_checkout_matches_git_for_main_and_linked_worktrees() -> TestResult {
    // Linked-worktree registrations are absolute. Execute in the disposable
    // fixture itself so no operation can follow them into the fixture cache.
    let fixture = gix_testtools::scripted_fixture_writable_with_args(
        "make_push_to_checkout.sh",
        std::iter::empty::<&str>(),
        gix_testtools::Creation::Execute,
    )?;
    let source = gix::open_opts(fixture.path().join("source"), gix::open::Options::isolated())?;
    let tip_id = source.head_id()?.detach();
    for mode in [
        "main-accept",
        "main-reject",
        "linked-accept",
        "linked-reject",
        "main-configured-accept",
        "main-configured-reject",
        "linked-configured-accept",
        "linked-configured-reject",
        "missing",
    ] {
        if mode.contains("configured") && *gix_testtools::GIT_VERSION < (2, 55, 0) {
            continue;
        }
        let native_path = fixture.path().join(format!("native-{mode}"));
        let expected_path = fixture.path().join(format!("git-{mode}"));
        let branch = if mode.starts_with("linked") { "deploy" } else { "main" };
        let name = format!("refs/heads/{branch}");
        let outcome = source.remote_at(gix::path::into_bstr(&native_path)?)?.push(
            [Update {
                destination: name.as_str().try_into()?,
                new_id: Some(tip_id),
                force: false,
                lease: Lease::Any,
            }],
            Options::default(),
            gix::progress::Discard,
            |_, _| Ok(()),
            &AtomicBool::default(),
        )?;
        let native = gix::open_opts(&native_path, gix::open::Options::isolated())?;
        let expected = gix::open_opts(&expected_path, gix::open::Options::isolated())?;
        assert_eq!(
            native.find_reference(&name)?.id().detach(),
            expected.find_reference(&name)?.id().detach(),
            "reference outcome for {mode}"
        );
        let worktree_name = std::fs::read_to_string(fixture.path().join(format!("native-{mode}-worktree-name")))?;
        let worktree = fixture.path().join(worktree_name);
        if mode == "missing" {
            assert!(
                matches!(&outcome.updates[0].status, Status::Rejected(Rejection::Server(reason)) if reason.contains_str("without a push-to-checkout hook")),
                "the unsupported default tree transition has a specific diagnostic"
            );
            assert_eq!(std::fs::read(worktree.join("file"))?, b"old\n");
            continue;
        }
        let expected_status = std::fs::read_to_string(fixture.path().join(format!("{mode}-status")))?;
        assert_eq!(
            matches!(outcome.updates[0].status, Status::Updated { .. }),
            expected_status == "success",
            "hook outcome matches Git for {mode}: {:?}",
            outcome.updates[0].status
        );
        assert_eq!(
            std::fs::read_to_string(worktree.join("checkout-id"))?,
            format!("{tip_id}\n")
        );
        let owner = gix::open_opts(&worktree, gix::open::Options::isolated())?;
        let hook_git_dir = std::fs::read_to_string(worktree.join("checkout-git-dir"))?;
        assert_eq!(
            std::path::Path::new(hook_git_dir.trim_end()).canonicalize()?,
            owner.git_dir().canonicalize()?,
            "the hook uses the updated branch's owning worktree"
        );
        let expected_worktree_name = std::fs::read_to_string(fixture.path().join(format!("git-{mode}-worktree-name")))?;
        assert_eq!(
            std::fs::read(worktree.join("file"))?,
            std::fs::read(fixture.path().join(expected_worktree_name).join("file"))?,
            "deployed contents for {mode}"
        );
        if mode.ends_with("accept") {
            assert!(gix_testtools::git(&worktree, "diff --exit-code")?.is_empty());
            assert!(gix_testtools::git(&worktree, "diff --cached --exit-code")?.is_empty());
        }
    }
    Ok(())
}

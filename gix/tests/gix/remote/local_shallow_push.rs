use std::sync::atomic::AtomicBool;

use gix::{
    bstr::ByteSlice,
    push::{Lease, Options, Status, Update},
};
use gix_testtools::TestResult;

#[test]
fn local_shallow_push_matches_git_receiver_boundaries() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_writable("make_local_shallow_push.sh")?;
    for case in ["complete", "shallow", "denied", "allowed", "full-to-shallow"] {
        let source_name = std::fs::read_to_string(fixture.path().join(format!("{case}-source-name")))?;
        let source = gix::open_opts(fixture.path().join(source_name), gix::open::Options::isolated())?;
        let native_path = fixture.path().join(format!("native-{case}"));
        let expected_path = fixture.path().join(format!("git-{case}"));
        let expected = std::fs::read_to_string(fixture.path().join(format!("{case}-status")))?;
        let tip_id = source.head_id()?.detach();
        let outcome = source.remote_at(gix::path::into_bstr(&native_path)?)?.push(
            [Update {
                destination: "refs/heads/topic".try_into()?,
                new_id: Some(tip_id),
                force: false,
                lease: Lease::Any,
            }],
            Options::default(),
            gix::progress::Discard,
            |_, _| Ok(()),
            &AtomicBool::default(),
        )?;
        assert_eq!(
            matches!(outcome.updates[0].status, Status::Updated { .. }),
            expected == "success",
            "native receive agrees with Git for {case}: {:?}",
            outcome.updates[0].status
        );
        let native = gix::open_opts(&native_path, gix::open::Options::isolated())?;
        let expected = gix::open_opts(&expected_path, gix::open::Options::isolated())?;
        let ref_id = |repo: &gix::Repository| -> gix::Result<_> {
            Ok(repo
                .try_find_reference("refs/heads/topic")?
                .map(|reference| reference.id().detach()))
        };
        assert_eq!(ref_id(&native)?, ref_id(&expected)?, "reference outcome for {case}");
        let boundaries = |repo: &gix::Repository| -> gix::Result<Vec<_>> {
            Ok(repo
                .shallow_commits()?
                .map(|ids| ids.iter().copied().collect())
                .unwrap_or_default())
        };
        assert_eq!(
            boundaries(&native)?,
            boundaries(&expected)?,
            "shallow boundaries for {case}"
        );
        if case == "denied" {
            assert!(
                format!("{:?}", outcome.updates[0].status)
                    .as_bytes()
                    .contains_str("shallow update not allowed"),
                "a policy rejection identifies the required shallow update"
            );
        }
        gix_testtools::git(&native_path, "fsck --strict --no-reflogs")?;
    }
    Ok(())
}

#[test]
#[cfg(unix)]
fn shallow_boundaries_follow_hook_acceptance_and_are_visible_in_quarantine() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_writable("make_local_shallow_push.sh")?;
    let source = gix::open_opts(fixture.path().join("multi-source"), gix::open::Options::isolated())?;
    for mode in ["partial", "atomic", "prereject"] {
        let native_path = fixture.path().join(format!("native-hook-{mode}"));
        let expected_path = fixture.path().join(format!("git-hook-{mode}"));
        let updates = [("main", "accept"), ("other", "reject")]
            .map(|(source_ref, destination)| -> TestResult<_> {
                Ok(Update {
                    destination: format!("refs/heads/{destination}").try_into()?,
                    new_id: Some(
                        source
                            .find_reference(&format!("refs/heads/{source_ref}"))?
                            .id()
                            .detach(),
                    ),
                    force: false,
                    lease: Lease::Any,
                })
            })
            .into_iter()
            .collect::<TestResult<Vec<_>>>()?;
        let outcome = source.remote_at(gix::path::into_bstr(&native_path)?)?.push(
            updates,
            Options {
                atomic: mode == "atomic",
                ..Options::default()
            },
            gix::progress::Discard,
            |_, _| Ok(()),
            &AtomicBool::default(),
        )?;
        let native = gix::open_opts(&native_path, gix::open::Options::isolated())?;
        let expected = gix::open_opts(&expected_path, gix::open::Options::isolated())?;
        for (index, name) in ["accept", "reject"].into_iter().enumerate() {
            let name = format!("refs/heads/{name}");
            let native_id = native
                .try_find_reference(&name)?
                .map(|reference| reference.id().detach());
            let expected_id = expected
                .try_find_reference(&name)?
                .map(|reference| reference.id().detach());
            assert_eq!(native_id, expected_id, "{mode}: {name}");
            assert_eq!(
                matches!(outcome.updates[index].status, Status::Updated { .. }),
                expected_id.is_some(),
                "the status matches the reference transaction"
            );
        }
        assert_eq!(
            std::fs::read(native_path.join("hook-log"))?,
            std::fs::read(expected_path.join("hook-log"))?,
            "pre-receive sees the same incoming history under temporary boundaries"
        );
        if mode == "partial" {
            assert_eq!(
                std::fs::read(native.shallow_file()?)?,
                std::fs::read(expected.shallow_file()?)?,
                "only the accepted branch contributes a persistent boundary"
            );
        } else {
            assert!(
                !native.is_shallow()?,
                "rejected commands do not require persistent boundaries"
            );
        }
        if mode == "prereject" {
            assert!(
                native.objects.iter()?.next().is_none(),
                "rejected quarantine is discarded"
            );
        }
        gix_testtools::git(&native_path, "fsck --strict --no-reflogs")?;
    }
    Ok(())
}

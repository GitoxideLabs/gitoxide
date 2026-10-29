#![cfg(unix)]

use std::{os::unix::fs::PermissionsExt, sync::atomic::AtomicBool};

use gix::push::{Lease, Options, Rejection, Status, Update};
use gix_testtools::TestResult;

#[test]
fn namespaces_and_hidden_ref_patterns_match_git_receive_pack() -> TestResult {
    let source = crate::remote::repo("base");
    let tip_id = source.head_id()?.detach();
    let prefix = "refs/namespaces/team/refs/namespaces/project/";
    for native in [false, true] {
        let destination = gix_testtools::tempfile::tempdir()?;
        gix_testtools::git(destination.path(), "init --bare")?;
        let rules = [
            "refs/heads/private",
            "!refs/heads/private/open",
            "^refs/namespaces/team/refs/namespaces/project/refs/heads/full-hidden",
            "!^refs/namespaces/team/refs/namespaces/project/refs/heads/full-hidden/open",
        ];
        for rule in rules {
            let output = gix_testtools::git_command(destination.path())
                .args(["config", "--add", "receive.hideRefs", rule])
                .output()?;
            assert!(output.status.success());
        }
        for (name, script) in [
            (
                "pre-receive",
                r#"#!/bin/sh
test "$GIT_NAMESPACE" = team/project || exit 1
cat >pre-receive-input
"#,
            ),
            (
                "reference-transaction",
                "#!/bin/sh
cat >>transaction-input
",
            ),
        ] {
            let hook = destination.path().join("hooks").join(name);
            std::fs::write(&hook, script)?;
            std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))?;
        }
        let names = [
            "refs/heads/topic",
            "refs/heads/private/rejected",
            "refs/heads/private/open",
            "refs/heads/full-hidden/rejected",
            "refs/heads/full-hidden/open",
        ];
        if native {
            let outcome = source
                .remote_at(gix::path::into_bstr(destination.path())?)?
                .with_command_environment([("GIT_NAMESPACE".into(), "team/project".into())])
                .push(
                    names.iter().map(|name| Update {
                        destination: (*name).try_into().expect("valid fixture name"),
                        new_id: Some(tip_id),
                        lease: Lease::MustMatch(None),
                        force: false,
                    }),
                    Options::default(),
                    gix::progress::Discard,
                    |_, _| Ok(()),
                    &AtomicBool::default(),
                )?;
            for (index, update) in outcome.updates.iter().enumerate() {
                if [1, 3].contains(&index) {
                    assert_eq!(
                        update.status,
                        Status::Rejected(Rejection::Server("deny updating a hidden ref".into()))
                    );
                } else {
                    assert!(matches!(update.status, Status::Updated { .. }), "{update:?}");
                }
            }
        } else {
            let output = gix_testtools::git_command(source.git_dir())
                .env("GIT_NAMESPACE", "team/project")
                .arg("push")
                .arg(destination.path())
                .args(names.iter().map(|name| format!("{tip_id}:{name}")))
                .output()?;
            assert!(!output.status.success(), "hidden references cause a partial rejection");
        }
        let receiver = gix::open_opts(destination.path(), gix::open::Options::isolated())?;
        for (index, name) in names.iter().enumerate() {
            assert!(
                receiver.try_find_reference(*name)?.is_none(),
                "the default namespace is untouched"
            );
            assert_eq!(
                receiver
                    .try_find_reference(format!("{prefix}{name}").as_str())?
                    .is_some(),
                ![1, 3].contains(&index)
            );
        }
        let transaction = std::fs::read_to_string(destination.path().join("transaction-input"))?;
        assert!(
            transaction.contains(&format!(" {prefix}refs/heads/topic\n")),
            "reference-transaction hooks see physical reference names"
        );
        let received = std::fs::read_to_string(destination.path().join("pre-receive-input"))?;
        assert!(
            received.contains(" refs/heads/topic\n"),
            "receive hooks see advertised reference names"
        );
    }
    Ok(())
}

#[test]
fn namespaces_protect_only_the_physical_checked_out_reference() -> TestResult {
    let source = crate::remote::repo("base");
    let old_id = source.find_reference("refs/heads/g")?.id().detach();
    let new_id = source.head_id()?.detach();
    let destination = gix_testtools::tempfile::tempdir()?;
    gix_testtools::git(destination.path(), "init -b main")?;
    let remote = source
        .remote_at(gix::path::into_bstr(destination.path())?)?
        .with_command_environment([("GIT_NAMESPACE".into(), "team".into())]);
    let push = |new_id| {
        remote.push(
            [Update {
                destination: "refs/heads/main".try_into().expect("valid test reference"),
                new_id: Some(new_id),
                lease: Lease::Any,
                force: false,
            }],
            Options::default(),
            gix::progress::Discard,
            |_, _| Ok(()),
            &AtomicBool::default(),
        )
    };
    assert!(
        matches!(push(old_id)?.updates[0].status, Status::Updated { .. }),
        "a namespaced branch is independent of the checked-out branch with the same short name"
    );
    gix_testtools::git(
        destination.path(),
        "symbolic-ref HEAD refs/namespaces/team/refs/heads/main",
    )?;
    assert_eq!(
        push(new_id)?.updates[0].status,
        Status::Rejected(Rejection::Server("branch is currently checked out".into())),
        "protect the actual namespaced reference checked out by a worktree"
    );
    Ok(())
}

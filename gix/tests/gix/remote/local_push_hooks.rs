#![cfg(unix)]

use std::{path::Path, sync::atomic::AtomicBool};

use gix::{
    Repository,
    push::{Lease, Options, Outcome, Rejection, Status, Update},
};
use gix_testtools::TestResult;

fn hook(path: &Path, name: &str, script: &str) -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let path = path.join("hooks").join(name);
    std::fs::write(&path, script)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    Ok(())
}

fn configured_hook(path: &Path, event: &str, name: &str, script: &str) -> TestResult {
    hook(path, name, script)?;
    gix_testtools::git(path, &format!("config hook.{name}.command ./hooks/{name}"))?;
    gix_testtools::git(path, &format!("config hook.{name}.event {event}"))?;
    Ok(())
}

fn push(
    repo: &Repository,
    path: &Path,
    updates: &[&str],
    options: Options<'_>,
    mut output: impl FnMut(&[u8]) -> std::io::Result<()>,
) -> gix::Result<Outcome> {
    let tip_id = repo.head_id()?.detach();
    repo.remote_at(gix::path::into_bstr(path)?)?.push(
        updates.iter().map(|name| Update {
            destination: (*name).try_into().expect("valid test refname"),
            new_id: Some(tip_id),
            lease: Lease::Any,
            force: false,
        }),
        options,
        gix::progress::Discard,
        |_, data| output(data),
        &AtomicBool::default(),
    )
}

#[test]
fn pre_receive_sees_quarantined_objects_and_options_and_rejection_removes_objects() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let repo = crate::remote::repo("base");
    let tip_id = repo.head_id()?.detach();
    for native in [false, true] {
        let destination = gix_testtools::tempfile::Builder::new()
            .prefix("receive: \"")
            .tempdir()?;
        gix_testtools::git(destination.path(), "init --bare")?;
        gix_testtools::git(destination.path(), "config receive.advertisePushOptions true")?;
        hook(
            destination.path(),
            "pre-receive",
            "#!/bin/sh
set -eu
read old new name
test -n \"$GIT_QUARANTINE_PATH\"
test \"$GIT_PUSH_OPTION_COUNT\" = 2
test \"$GIT_PUSH_OPTION_0\" = 'space and = sign'
test \"$GIT_PUSH_OPTION_1\" = ''
git cat-file -e \"$new^{commit}\"
printf 'quarantine validated\\n'
exit 1
",
        )?;
        if native {
            let mut output = Vec::new();
            let result = push(
                &repo,
                destination.path(),
                &["refs/heads/topic"],
                Options {
                    push_options: &["space and = sign".into(), "".into()],
                    ..Options::default()
                },
                |data| {
                    output.extend_from_slice(data);
                    Ok(())
                },
            )?;
            assert_eq!(
                result.updates[0].status,
                Status::Rejected(Rejection::Server("pre-receive hook declined".into()))
            );
            assert!(String::from_utf8(output)?.contains("quarantine validated"));
        } else {
            let output = gix_testtools::git_command(repo.git_dir())
                .args(["push", "-o", "space and = sign", "-o", ""])
                .arg(destination.path())
                .arg(format!("{tip_id}:refs/heads/topic"))
                .output()?;
            assert!(!output.status.success());
            assert!(String::from_utf8(output.stderr)?.contains("quarantine validated"));
        }
        let remote = gix::open_opts(destination.path(), gix::open::Options::isolated())?;
        assert!(
            !remote.has_object(tip_id),
            "rejected quarantine never publishes incoming objects"
        );
        assert!(remote.try_find_reference("refs/heads/topic")?.is_none());
        assert!(
            !std::fs::read_dir(destination.path().join("objects"))?
                .any(|entry| entry.is_ok_and(|entry| entry.file_name().to_string_lossy().starts_with("incoming-")))
        );
    }
    Ok(())
}

#[test]
fn configured_receive_events_replay_inputs_and_reject_updates_like_git() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let repo = crate::remote::repo("base");
    let tip_id = repo.head_id()?.detach();
    let mut expected_logs = None;
    for native in [false, true] {
        if !native && *gix_testtools::GIT_VERSION < (2, 55, 0) {
            continue;
        }
        let destination = gix_testtools::tempfile::tempdir()?;
        gix_testtools::git(destination.path(), "init --bare")?;
        gix_testtools::git(destination.path(), "config receive.advertisePushOptions true")?;
        let events = [
            "pre-receive",
            "update",
            "reference-transaction",
            "post-receive",
            "post-update",
        ];
        for event in events {
            for label in ["first", "second", "traditional"] {
                let script = format!(
                    "#!/bin/sh
set -eu
event={event}
label={label}
case $event in
    pre-receive|post-receive)
        test \"$GIT_PUSH_OPTION_COUNT\" = 2
        test \"$GIT_PUSH_OPTION_0\" = 'space and = sign'
        test \"$GIT_PUSH_OPTION_1\" = ''
        if test \"$event\" = pre-receive; then
            test -n \"$GIT_QUARANTINE_PATH\"
        else
            test -z \"${{GIT_QUARANTINE_PATH-}}\"
        fi
        while read old new name; do
            git cat-file -e \"$new^{{commit}}\"
            printf '%s %s\\n' \"$label\" \"$name\" >>\"$event-log\"
        done
        test \"$event\" != post-receive
        ;;
    reference-transaction)
        while read old new name; do
            printf '%s %s %s\\n' \"$label\" \"$1\" \"$name\" >>\"$event-log\"
        done
        ;;
    *)
        printf '%s %s\\n' \"$label\" \"$1\" >>\"$event-log\"
        test \"$label:$1\" != first:refs/heads/rejected
        ;;
esac
"
                );
                if label == "traditional" {
                    hook(destination.path(), event, &script)?;
                } else {
                    configured_hook(destination.path(), event, &format!("{label}-{event}"), &script)?;
                }
            }
        }
        if native {
            let outcome = push(
                &repo,
                destination.path(),
                &["refs/heads/accepted", "refs/heads/rejected"],
                Options {
                    push_options: &["space and = sign".into(), "".into()],
                    ..Options::default()
                },
                |_| Ok(()),
            )?;
            assert_eq!(outcome.updates[0].status, Status::Updated { forced: false });
            assert_eq!(
                outcome.updates[1].status,
                Status::Rejected(Rejection::Server("hook declined".into())),
                "a configured update hook can reject a ref after all hooks for that event run"
            );
        } else {
            let output = gix_testtools::git_command(repo.git_dir())
                .args(["push", "-o", "space and = sign", "-o", ""])
                .arg(destination.path())
                .args([
                    format!("{tip_id}:refs/heads/accepted"),
                    format!("{tip_id}:refs/heads/rejected"),
                ])
                .output()?;
            assert!(!output.status.success(), "Git rejects one of the updates");
        }
        let logs: Vec<_> = events
            .iter()
            .map(|event| std::fs::read_to_string(destination.path().join(format!("{event}-log"))))
            .collect::<Result<_, _>>()?;
        for (event, log) in events.iter().zip(&logs) {
            assert!(log.contains("first "), "configured hook runs for {event}");
            assert!(log.contains("second "), "later configured hook runs for {event}");
            assert!(log.contains("traditional "), "traditional hook runs for {event}");
        }
        if let Some(expected) = &expected_logs {
            assert_eq!(
                &logs, expected,
                "event order, replayed stdin, arguments and rejection match Git"
            );
        } else {
            expected_logs = Some(logs);
        }
    }
    Ok(())
}

#[test]
fn disabled_receive_event_skips_configured_and_traditional_hooks() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let repo = crate::remote::repo("base");
    let destination = gix_testtools::tempfile::tempdir()?;
    gix_testtools::git(destination.path(), "init --bare")?;
    for event in [
        "pre-receive",
        "update",
        "reference-transaction",
        "post-receive",
        "post-update",
    ] {
        let script = "#!/bin/sh
touch unexpected-hook
exit 1
";
        hook(destination.path(), event, script)?;
        configured_hook(destination.path(), event, &format!("configured-{event}"), script)?;
        gix_testtools::git(destination.path(), &format!("config hook.{event}.enabled false"))?;
    }
    let outcome = push(
        &repo,
        destination.path(),
        &["refs/heads/topic"],
        Options::default(),
        |_| Ok(()),
    )?;
    assert_eq!(outcome.updates[0].status, Status::Updated { forced: false });
    assert!(
        !destination.path().join("unexpected-hook").exists(),
        "the whole event is disabled"
    );
    Ok(())
}

#[test]
fn update_and_post_hooks_observe_only_successful_references() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let repo = crate::remote::repo("base");
    let destination = gix_testtools::tempfile::tempdir()?;
    gix_testtools::git(destination.path(), "init --bare")?;
    hook(
        destination.path(),
        "update",
        "#!/bin/sh
printf 'update %s\\n' \"$1\" >>hook-log
test \"$1\" != refs/heads/rejected
",
    )?;
    hook(
        destination.path(),
        "post-receive",
        "#!/bin/sh
set -eu
test -z \"${GIT_QUARANTINE_PATH-}\"
while read old new name; do
    git cat-file -e \"$new\"
    printf 'post-receive %s\\n' \"$name\" >>hook-log
done
exit 1
",
    )?;
    hook(
        destination.path(),
        "post-update",
        "#!/bin/sh
printf 'post-update %s\\n' \"$@\" >>hook-log
",
    )?;
    let mut output = Vec::new();
    let outcome = push(
        &repo,
        destination.path(),
        &["refs/heads/accepted", "refs/heads/rejected"],
        Options::default(),
        |bytes| {
            output.extend_from_slice(bytes);
            Ok(())
        },
    )?;
    assert!(String::from_utf8(output)?.contains("error: hook declined to update refs/heads/rejected\n"));
    assert_eq!(outcome.updates[0].status, Status::Updated { forced: false });
    assert_eq!(
        outcome.updates[1].status,
        Status::Rejected(Rejection::Server("hook declined".into()))
    );
    assert_eq!(
        std::fs::read_to_string(destination.path().join("hook-log"))?,
        "update refs/heads/accepted
update refs/heads/rejected
post-receive refs/heads/accepted
post-update refs/heads/accepted
"
    );
    Ok(())
}

#[test]
fn reference_transaction_hook_can_reject_prepared_atomic_updates() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let repo = crate::remote::repo("base");
    let destination = gix_testtools::tempfile::tempdir()?;
    gix_testtools::git(destination.path(), "init --bare")?;
    hook(
        destination.path(),
        "reference-transaction",
        "#!/bin/sh
printf '%s\\n' \"$1\" >>transaction-log
while read old new name; do printf '%s\\n' \"$name\" >>transaction-log; done
test \"$1\" != prepared
",
    )?;
    let result = push(
        &repo,
        destination.path(),
        &["refs/heads/one", "refs/heads/two"],
        Options {
            atomic: true,
            ..Options::default()
        },
        |_| Ok(()),
    )?;
    assert!(
        result
            .updates
            .iter()
            .all(|update| matches!(update.status, Status::Rejected(Rejection::Server(_))))
    );
    let remote = gix::open_opts(destination.path(), gix::open::Options::isolated())?;
    assert!(remote.try_find_reference("refs/heads/one")?.is_none());
    assert!(remote.try_find_reference("refs/heads/two")?.is_none());
    assert_eq!(
        std::fs::read_to_string(destination.path().join("transaction-log"))?,
        "preparing
refs/heads/one
refs/heads/two
prepared
refs/heads/one
refs/heads/two
aborted
refs/heads/one
refs/heads/two
"
    );
    Ok(())
}

#[test]
fn reference_transaction_preparing_hook_rejects_before_locks_like_git() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let repo = crate::remote::repo("base");
    let tip_id = repo.head_id()?.detach();
    for native in [false, true] {
        if !native && *gix_testtools::GIT_VERSION < (2, 55, 0) {
            continue;
        }
        let destination = gix_testtools::tempfile::tempdir()?;
        gix_testtools::git(destination.path(), "init --bare")?;
        configured_hook(
            destination.path(),
            "reference-transaction",
            "before-locks",
            "#!/bin/sh
set -eu
printf '%s\\n' \"$1\" >>transaction-log
while read old new name; do printf '%s\\n' \"$name\" >>transaction-log; done
if test \"$1\" = preparing; then
    test ! -e refs/heads/one.lock
    test ! -e refs/heads/two.lock
    printf 'unlocked\\n' >>transaction-log
    exit 1
fi
",
        )?;
        if native {
            let outcome = push(
                &repo,
                destination.path(),
                &["refs/heads/one", "refs/heads/two"],
                Options {
                    atomic: true,
                    ..Options::default()
                },
                |_| Ok(()),
            )?;
            assert!(
                outcome
                    .updates
                    .iter()
                    .all(|update| matches!(update.status, Status::Rejected(Rejection::Server(_)))),
                "the preparing hook rejects the whole transaction"
            );
        } else {
            let output = gix_testtools::git_command(repo.git_dir())
                .args(["push", "--atomic"])
                .arg(destination.path())
                .args([format!("{tip_id}:refs/heads/one"), format!("{tip_id}:refs/heads/two")])
                .output()?;
            assert!(!output.status.success(), "Git rejects before taking reference locks");
        }
        let remote = gix::open_opts(destination.path(), gix::open::Options::isolated())?;
        for name in ["one", "two"] {
            assert!(remote.try_find_reference(&format!("refs/heads/{name}"))?.is_none());
            assert!(!destination.path().join(format!("refs/heads/{name}.lock")).exists());
        }
        assert_eq!(
            std::fs::read_to_string(destination.path().join("transaction-log"))?,
            "preparing
refs/heads/one
refs/heads/two
unlocked
aborted
refs/heads/one
refs/heads/two
",
            "preparing and aborted receive the original queued updates, matching Git"
        );
    }
    Ok(())
}

#[test]
fn progress_callback_failure_cancels_an_active_hook() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let repo = crate::remote::repo("base");
    let destination = gix_testtools::tempfile::tempdir()?;
    gix_testtools::git(destination.path(), "init --bare")?;
    configured_hook(
        destination.path(),
        "pre-receive",
        "cancel",
        "#!/bin/sh
while :; do printf 'hook output\\n'; done
",
    )?;
    hook(
        destination.path(),
        "pre-receive",
        "#!/bin/sh
touch unexpected-hook
",
    )?;
    let result = push(
        &repo,
        destination.path(),
        &["refs/heads/topic"],
        Options::default(),
        |_| Err(std::io::Error::other("progress cancelled")),
    );
    assert!(
        result
            .expect_err("callback stops the active hook")
            .to_string()
            .contains("progress cancelled")
    );
    let remote = gix::open_opts(destination.path(), gix::open::Options::isolated())?;
    assert!(remote.try_find_reference("refs/heads/topic")?.is_none());
    assert!(
        !destination.path().join("unexpected-hook").exists(),
        "callback cancellation stops later hooks in the event"
    );
    Ok(())
}

#[test]
fn successful_hook_can_close_input_early_without_deadlocking_output() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let repo = crate::remote::repo("base");
    let destination = gix_testtools::tempfile::tempdir()?;
    gix_testtools::git(destination.path(), "init --bare")?;
    hook(
        destination.path(),
        "pre-receive",
        "#!/bin/sh
i=0
while test $i -lt 2048; do
    printf 'hook output before input is consumed................................\\n'
    i=$((i + 1))
done
exit 0
",
    )?;
    let mut output = 0;
    let long = "x".repeat(160);
    let outcome = repo.remote_at(gix::path::into_bstr(destination.path())?)?.push(
        (0..512).map(|index| Update {
            destination: format!("refs/heads/{long}-{index}").try_into().expect("valid name"),
            new_id: None,
            lease: Lease::Any,
            force: false,
        }),
        Options {
            atomic: true,
            ..Options::default()
        },
        gix::progress::Discard,
        |_, data| {
            output += data.len();
            Ok(())
        },
        &AtomicBool::default(),
    )?;
    assert!(output > 128 * 1024, "hook output exceeds a pipe's buffer");
    assert!(
        outcome
            .updates
            .iter()
            .all(|update| matches!(update.status, Status::Updated { .. }))
    );
    Ok(())
}

#[test]
fn exited_hook_does_not_wait_for_descendants_holding_only_stdin() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let repo = crate::remote::repo("base");
    let destination = gix_testtools::tempfile::tempdir()?;
    gix_testtools::git(destination.path(), "init --bare")?;
    hook(
        destination.path(),
        "pre-receive",
        "#!/bin/sh
sleep 3 <&0 >/dev/null 2>&1 &
exit 0
",
    )?;
    let long = "x".repeat(160);
    let start = std::time::Instant::now();
    let result = repo.remote_at(gix::path::into_bstr(destination.path())?)?.push(
        (0..512).map(|index| Update {
            destination: format!("refs/heads/{long}-{index}").try_into().expect("valid name"),
            new_id: None,
            lease: Lease::Any,
            force: false,
        }),
        Options {
            atomic: true,
            ..Options::default()
        },
        gix::progress::Discard,
        |_, _| Ok(()),
        &AtomicBool::default(),
    )?;
    assert!(
        start.elapsed() < std::time::Duration::from_secs(2),
        "the hook has exited and its descendant must not delay the push"
    );
    assert!(
        result
            .updates
            .iter()
            .all(|update| matches!(update.status, Status::Updated { .. }))
    );
    Ok(())
}

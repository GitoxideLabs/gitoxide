use bstr::{BStr, BString, ByteSlice};
use gix_protocol::{
    push::{Command, Options, ReportStatus, Request},
    transport::client::Capabilities,
};

use crate::TestResult;
#[cfg(feature = "blocking-client")]
use gix_error::ResultExt;

fn command(name: &str, old: u8, new: u8) -> Command {
    Command {
        name: name.try_into().expect("valid full reference name"),
        old_id: gix_hash::ObjectId::from_hex(format!("{old:040x}").as_bytes()).expect("valid object ID"),
        new_id: gix_hash::ObjectId::from_hex(format!("{new:040x}").as_bytes()).expect("valid object ID"),
    }
}

fn capabilities(value: &str) -> Capabilities {
    Capabilities::from_bytes(format!("\0{value}").as_bytes())
        .expect("valid capabilities")
        .0
}

#[test]
fn request_negotiates_capabilities_and_preserves_compare_and_swap() -> TestResult {
    let options = [BString::from("ci.skip"), BString::from("")];
    let request = Request::new(
        vec![command("refs/heads/main", 1, 2), command("refs/heads/old", 2, 0)],
        gix_hash::Kind::Sha1,
        &capabilities(
            "report-status report-status-v2 side-band-64k atomic push-options delete-refs ofs-delta agent=git/2 object-format=sha1",
        ),
        Options {
            atomic: true,
            push_options: &options,
            agent: Some(BStr::new(b"git/gix")),
            ..Options::default()
        },
    )?;
    assert_eq!(request.features().report_status, ReportStatus::V2);
    assert!(request.features().side_band_64k);
    assert!(request.features().ofs_delta);
    assert!(request.features().thin_pack);
    assert_eq!(request.push_options(), options);
    assert_eq!(
        request.command_lines().collect::<Vec<_>>(),
        [
            BString::from(
                "0000000000000000000000000000000000000001 0000000000000000000000000000000000000002 refs/heads/main\0report-status-v2 side-band-64k atomic push-options object-format=sha1 agent=git/gix"
            ),
            BString::from(
                "0000000000000000000000000000000000000002 0000000000000000000000000000000000000000 refs/heads/old"
            ),
        ],
        "wire commands retain the expected old IDs and only advertise supported features"
    );
    Ok(())
}

#[test]
fn request_rejects_unsupported_requirements_and_packet_injection() -> TestResult {
    let base = capabilities("report-status no-thin");
    let commands = vec![command("refs/heads/main", 1, 2)];
    let request = Request::new(commands.clone(), gix_hash::Kind::Sha1, &base, Options::default())?;
    assert!(!request.features().thin_pack, "no-thin forbids omitted delta bases");
    assert_eq!(request.features().report_status, ReportStatus::V1);
    assert!(
        Request::new(
            commands.clone(),
            gix_hash::Kind::Sha1,
            &base,
            Options {
                atomic: true,
                ..Options::default()
            }
        )
        .is_err()
    );
    assert!(
        Request::new(
            vec![command("refs/heads/main", 1, 0)],
            gix_hash::Kind::Sha1,
            &base,
            Options::default()
        )
        .is_err()
    );
    assert!(
        Request::new(
            commands.clone(),
            gix_hash::Kind::Sha1,
            &capabilities("side-band-64k"),
            Options::default()
        )
        .is_err()
    );
    assert!(
        Request::new(
            commands.clone(),
            gix_hash::Kind::Sha1,
            &capabilities("report-status object-format=sha256"),
            Options::default()
        )
        .is_err()
    );
    assert!(
        Request::new(
            vec![commands[0].clone(), commands[0].clone()],
            gix_hash::Kind::Sha1,
            &base,
            Options::default()
        )
        .is_err()
    );
    for option in [
        "first
second",
        "first\0second",
    ] {
        assert!(
            Request::new(
                commands.clone(),
                gix_hash::Kind::Sha1,
                &capabilities("report-status push-options"),
                Options {
                    push_options: &[option.into()],
                    ..Options::default()
                }
            )
            .is_err(),
            "options cannot inject packet contents"
        );
    }
    assert!(
        Request::new(
            commands,
            gix_hash::Kind::Sha1,
            &capabilities("report-status agent=x"),
            Options {
                agent: Some(BStr::new(b"agent extra-capability")),
                ..Options::default()
            }
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn response_preserves_partial_success_and_rejection_reasons() -> TestResult {
    let request = Request::new(
        vec![command("refs/heads/main", 1, 2), command("refs/heads/other", 1, 3)],
        gix_hash::Kind::Sha1,
        &capabilities("report-status"),
        Options::default(),
    )?;
    let response = request.parse_response([
        b"unpack ok\n".as_slice(),
        b"ok refs/heads/main\n",
        b"ng refs/heads/other hook declined\n",
    ])?;
    assert_eq!(response.unpack_error, None);
    assert_eq!(response.refs[0].error, None);
    assert_eq!(
        response.refs[1].error.as_ref().map(|s| s.as_bstr()),
        Some(BStr::new(b"hook declined"))
    );
    for lines in [
        vec!["unpack ok", "ok refs/heads/main"],
        vec![
            "unpack ok",
            "ok refs/heads/main",
            "ok refs/heads/main",
            "ok refs/heads/other",
        ],
        vec!["unpack ok", "ok refs/heads/unknown", "ok refs/heads/other"],
        vec!["ok refs/heads/main", "ok refs/heads/other"],
        vec!["unpack ok", "ng refs/heads/main", "ok refs/heads/other"],
        vec![
            "unpack ok",
            "ok refs/heads/main",
            "option forced-update",
            "ok refs/heads/other",
        ],
    ] {
        assert!(
            request.parse_response(lines).is_err(),
            "incomplete or invalid reports must not be mistaken for success"
        );
    }
    Ok(())
}

#[test]
fn response_v2_reports_rewritten_and_multiple_destinations() -> TestResult {
    let request = Request::new(
        vec![command("refs/for/main", 1, 2)],
        gix_hash::Kind::Sha1,
        &capabilities("report-status-v2"),
        Options::default(),
    )?;
    let response = request.parse_response([
        "unpack ok",
        "ok refs/for/main",
        "option refname refs/heads/main",
        "option old-oid 0000000000000000000000000000000000000001",
        "option new-oid 0000000000000000000000000000000000000002",
        "option forced-update",
        "option future-extension value",
        "ok refs/for/main",
        "option refname refs/heads/second",
    ])?;
    assert_eq!(
        response.refs.len(),
        2,
        "a proc-receive command can update several references"
    );
    assert_eq!(
        response.refs[0]
            .report
            .ref_name
            .as_ref()
            .map(gix_ref::FullName::as_bstr),
        Some(BStr::new(b"refs/heads/main"))
    );
    assert!(response.refs[0].report.forced_update);
    assert_eq!(
        response.refs[0].report.new_id,
        Some(command("refs/heads/main", 1, 2).new_id)
    );
    assert_eq!(
        response.refs[1]
            .report
            .ref_name
            .as_ref()
            .map(gix_ref::FullName::as_bstr),
        Some(BStr::new(b"refs/heads/second"))
    );
    assert!(
        request
            .parse_response(["unpack ok", "option forced-update", "ok refs/for/main"])
            .is_err()
    );
    assert!(
        request
            .parse_response(["unpack ok", "ng refs/for/main denied", "option forced-update"])
            .is_err()
    );
    assert!(
        request
            .parse_response(["unpack ok", "ok refs/for/main", "option new-oid invalid"])
            .is_err()
    );
    let response = request.parse_response(["unpack invalid pack", "ng refs/for/main unpacker error"])?;
    assert_eq!(
        response.unpack_error.as_ref().map(|s| s.as_bstr()),
        Some(BStr::new(b"invalid pack"))
    );
    Ok(())
}

#[cfg(feature = "blocking-client")]
#[test]
fn requests_and_reports_interoperate_with_git_receive_pack() -> TestResult {
    use std::{io::Write, process::Stdio};

    let directory = gix_testtools::tempfile::tempdir()?;
    gix_testtools::git(directory.path(), "init --bare")?;
    let blob_id = gix_testtools::git(directory.path(), "hash-object -w --stdin")?;
    for name in ["first", "stale"] {
        gix_testtools::git(
            directory.path(),
            &format!("update-ref refs/tags/{name} {}", blob_id.trim()),
        )?;
    }
    let mut deletion = command("refs/tags/first", 1, 0);
    deletion.old_id = gix_hash::ObjectId::from_hex(blob_id.trim().as_bytes())?;
    std::fs::write(directory.path().join("other"), b"different blob")?;
    let other_blob_id = gix_testtools::git(directory.path(), "hash-object -w other")?;
    let mut stale = command("refs/tags/stale", 1, 0);
    stale.old_id = gix_hash::ObjectId::from_hex(other_blob_id.trim().as_bytes())?;
    let request = Request::new(
        vec![deletion, stale],
        gix_hash::Kind::Sha1,
        &capabilities("report-status delete-refs"),
        Options::default(),
    )?;
    let mut child = gix_testtools::git_command(directory.path())
        .args(["receive-pack", "--stateless-rpc", "."])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    {
        let mut input = child.stdin.take().expect("piped stdin");
        for line in request.command_lines() {
            gix_packetline::blocking_io::encode::data_to_write(&line, &mut input)?;
        }
        input.write_all(b"0000")?;
    }
    let output = child.wait_with_output()?;
    let mut bytes = output.stdout.as_slice();
    let mut lines = Vec::new();
    loop {
        match gix_packetline::decode::streaming(bytes)? {
            gix_packetline::decode::Stream::Complete { line, bytes_consumed } => {
                bytes = &bytes[bytes_consumed..];
                match line {
                    gix_packetline::PacketLineRef::Data(line) => lines.push(line),
                    gix_packetline::PacketLineRef::Flush => break,
                    _ => panic!("unexpected response delimiter"),
                }
            }
            gix_packetline::decode::Stream::Incomplete { .. } => panic!("incomplete Git response: {output:?}"),
        }
    }
    assert!(bytes.is_empty(), "the complete status report ends at the flush");
    let response = request.parse_response(lines)?;
    assert_eq!(response.unpack_error, None);
    assert_eq!(
        response.refs[0].error, None,
        "an advertised old ID authorizes the deletion"
    );
    assert!(
        response.refs[1].error.is_some(),
        "Git rejects a stale expected old ID under the reference lock"
    );
    let refs = gix_testtools::git(directory.path(), "for-each-ref --format=%(refname)")?;
    assert_eq!(
        refs.trim(),
        "refs/tags/stale",
        "the successful deletion is independent of the stale one"
    );
    Ok(())
}

#[cfg(feature = "blocking-client")]
#[test]
fn native_push_transfers_a_pack_and_empty_push_options_with_sidebands() -> TestResult {
    use gix_protocol::push::SendOptions;
    use std::{io::Write, process::Stdio, sync::atomic::AtomicBool};

    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let directory = gix_testtools::tempfile::tempdir()?;
    let source = directory.path().join("source");
    let destination = directory.path().join("destination");
    std::fs::create_dir(&source)?;
    std::fs::create_dir(&destination)?;
    gix_testtools::git(&source, "init --bare")?;
    gix_testtools::git(&destination, "init --bare")?;
    gix_testtools::git(&destination, "config receive.advertisePushOptions true")?;
    let blob_id = gix_testtools::git(&source, "hash-object -w --stdin")?;
    let mut pack = gix_testtools::git_command(&source)
        .args(["pack-objects", "--stdout", "--revs"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    pack.stdin.take().expect("piped input").write_all(blob_id.as_bytes())?;
    let pack = pack.wait_with_output()?;
    assert!(pack.status.success());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let hook = destination.join("hooks/pre-receive");
        std::fs::write(
            &hook,
            r#"#!/bin/sh
printf 'options:%s|%s|%s\n' "$GIT_PUSH_OPTION_COUNT" "$GIT_PUSH_OPTION_0" "$GIT_PUSH_OPTION_1"
cat >/dev/null
"#,
        )?;
        std::fs::set_permissions(hook, std::fs::Permissions::from_mode(0o755))?;
    }
    let mut transport = gix_transport::client::blocking_io::file::connect(
        destination.to_string_lossy().as_bytes().as_bstr().to_owned(),
        gix_transport::Protocol::V1,
        false,
    )?;
    let handshake = gix_protocol::handshake(
        &mut transport,
        gix_transport::Service::ReceivePack,
        |_| Ok(None),
        Vec::new(),
        &mut gix_utils::progress::Discard,
    )?;
    let mut update = command("refs/tags/new", 0, 1);
    update.new_id = gix_hash::ObjectId::from_hex(blob_id.trim().as_bytes())?;
    let request = Request::new(
        vec![update],
        gix_hash::Kind::Sha1,
        &handshake.capabilities,
        Options {
            atomic: true,
            push_options: &["".into(), "ci.skip".into()],
            ..Options::default()
        },
    )?;
    let mut sideband = Vec::new();
    let response = request
        .send(
            &mut transport,
            move |writer| {
                writer.write_all(&pack.stdout).or_error()?;
                Ok(())
            },
            |_, text| {
                sideband.extend_from_slice(text);
                Ok(())
            },
            &AtomicBool::new(false),
            SendOptions {
                server_protocol: handshake.server_protocol_version,
                ..SendOptions::default()
            },
        )?
        .expect("one command receives a status report");
    assert_eq!(response.unpack_error, None);
    assert_eq!(response.refs[0].error, None);
    assert_eq!(gix_testtools::git(&destination, "rev-parse refs/tags/new")?, blob_id);
    #[cfg(unix)]
    assert!(
        sideband.contains_str("options:2||ci.skip"),
        "receive hooks observe empty options and their output is delivered over sideband"
    );
    #[cfg(unix)]
    std::fs::remove_file(destination.join("hooks/pre-receive"))?;
    let mut transport = gix_transport::client::blocking_io::file::connect(
        destination.to_string_lossy().as_bytes().as_bstr().to_owned(),
        gix_transport::Protocol::V1,
        false,
    )?;
    gix_protocol::handshake(
        &mut transport,
        gix_transport::Service::ReceivePack,
        |_| Ok(None),
        Vec::new(),
        &mut gix_utils::progress::Discard,
    )?;
    let mut deletion = command("refs/tags/new", 1, 0);
    deletion.old_id = gix_hash::ObjectId::from_hex(blob_id.trim().as_bytes())?;
    let request = Request::new(
        vec![deletion],
        gix_hash::Kind::Sha1,
        &capabilities("report-status delete-refs"),
        Options::default(),
    )?;
    let response = request
        .send(
            &mut transport,
            |_| panic!("deletion-only pushes must not request a pack"),
            |_, _| Ok(()),
            &AtomicBool::new(false),
            SendOptions::default(),
        )?
        .expect("deletion is reported");
    assert_eq!(
        response.refs[0].error, None,
        "plain report-status works without sidebands"
    );
    assert!(
        gix_testtools::git(&destination, "for-each-ref")?.is_empty(),
        "the remote confirms a deletion without pack data"
    );

    Ok(())
}

#[cfg(feature = "blocking-client")]
#[test]
fn failed_pack_production_aborts_receive_pack_without_hanging() -> TestResult {
    use std::sync::atomic::AtomicBool;
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let directory = gix_testtools::tempfile::tempdir()?;
    gix_testtools::git(directory.path(), "init --bare")?;
    let mut transport = gix_transport::client::blocking_io::file::connect(
        directory.path().to_string_lossy().as_bytes().as_bstr().to_owned(),
        gix_transport::Protocol::V1,
        false,
    )?;
    let handshake = gix_protocol::handshake(
        &mut transport,
        gix_transport::Service::ReceivePack,
        |_| Ok(None),
        Vec::new(),
        &mut gix_utils::progress::Discard,
    )?;
    let request = Request::new(
        vec![command("refs/tags/new", 0, 1)],
        gix_hash::Kind::Sha1,
        &handshake.capabilities,
        Options::default(),
    )?;
    let start = std::time::Instant::now();
    let error = request
        .send(
            &mut transport,
            |writer| {
                writer.write_all(b"PACK").or_error()?;
                Err(gix_error::message("pack producer failed").validation_error())
            },
            |_, _| Ok(()),
            &AtomicBool::new(false),
            gix_protocol::push::SendOptions::default(),
        )
        .expect_err("a truncated pack must fail");
    assert!(
        format!("{error:#}").contains("pack producer failed"),
        "the producer's error is retained: {error:#}"
    );
    assert!(
        start.elapsed() < std::time::Duration::from_secs(10),
        "the blocked receive-pack process is interrupted promptly"
    );
    Ok(())
}

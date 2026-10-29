use bstr::{BStr, BString, ByteSlice};
use gix_protocol::{
    push::{Command, Options, ReportStatus, Request},
    transport::client::Capabilities,
};

use crate::TestResult;

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

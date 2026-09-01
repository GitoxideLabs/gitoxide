#[cfg(feature = "blocking-client")]
use std::io::{BufRead, Write};
use std::{error::Error, ops::Deref, sync::Arc};

use bstr::ByteSlice;
#[cfg(all(feature = "async-client", not(feature = "blocking-client")))]
use futures_lite::{AsyncBufReadExt, AsyncWriteExt, StreamExt};
#[cfg(all(feature = "async-client", not(feature = "blocking-client")))]
use gix_transport::client::{
    async_io::{Transport, TransportV2Ext},
    git::async_io::Connection,
};
#[cfg(feature = "blocking-client")]
use gix_transport::client::{
    blocking_io::{Transport, TransportV2Ext},
    git::blocking_io::Connection,
};
use gix_transport::{
    Protocol, Service, client,
    client::{TransportWithoutIO, git},
};
use parking_lot::Mutex;

use crate::fixture_bytes;

#[crate::bisync::bisync]
#[cfg_attr(feature = "blocking-client", test)]
#[cfg_attr(all(feature = "async-client", not(feature = "blocking-client")), async_std::test)]
async fn handshake_v1_and_request() -> crate::Result {
    let mut out = Vec::new();
    let server_response = fixture_bytes("v1/clone.response");
    let c = Connection::new(
        server_response.as_slice(),
        &mut out,
        Protocol::V1,
        "/foo.git",
        Some(("example.org", None)),
        git::ConnectMode::Daemon,
        false,
    );
    assert!(
        c.connection_persists_across_multiple_requests(),
        "tcp connections are stateful"
    );
    let c = c.custom_url(Some("anything".into()));
    assert_eq!(c.to_url().as_ref(), "anything");
    let mut c = c.custom_url(None);
    assert_eq!(c.to_url().as_ref(), "file:///foo.git");
    let mut res = c.handshake(Service::UploadPack, &[]).await?;
    assert_eq!(res.actual_protocol, Protocol::V1);
    assert_eq!(
        res.capabilities
            .iter()
            .map(|c| (c.name().to_owned(), c.value().map(ToOwned::to_owned)))
            .collect::<Vec<_>>(),
        [
            ("multi_ack", None),
            ("thin-pack", None),
            ("side-band", None),
            ("side-band-64k", None),
            ("ofs-delta", None),
            ("shallow", None),
            ("deepen-since", None),
            ("deepen-not", None),
            ("deepen-relative", None),
            ("no-progress", None),
            ("include-tag", None),
            ("multi_ack_detailed", None),
            ("symref", Some("HEAD:refs/heads/master")),
            ("object-format", Some("sha1")),
            ("agent", Some("git/2.28.0"))
        ]
        .iter()
        .map(|(n, v)| (
            n.as_bytes().as_bstr().to_owned(),
            v.map(|v| v.as_bytes().as_bstr().to_owned())
        ))
        .collect::<Vec<_>>()
    );
    let mut lines = res.refs.as_mut().expect("v1 protocol provides refs").lines();
    let mut refs = Vec::new();
    // needed in async version of test
    while let Some(line) = lines.next().await {
        refs.push(line?);
    }
    // needed for non-async version
    drop(lines);

    assert_eq!(
        refs,
        vec![
            "808e50d724f604f69ab93c6da2919c014667bedb HEAD",
            "808e50d724f604f69ab93c6da2919c014667bedb refs/heads/master"
        ]
    );
    drop(res);

    let writer = c.request(client::WriteMode::Binary, client::MessageKind::Flush, false)?;
    let nak_line = writer
        .into_read()
        .await?
        .lines()
        .next()
        .await
        .expect("exactly one line")?;
    assert_eq!(nak_line, "NAK");

    let mut writer = c.request(
        client::WriteMode::OneLfTerminatedLinePerWriteCall,
        client::MessageKind::Text(b"done"),
        false,
    )?;

    writer.write_all(b"hello").await?;
    writer.write_all(b"world").await?;

    let mut reader = writer.into_read().await?;
    let messages = Arc::new(Mutex::new(Vec::<String>::new()));
    reader.set_progress_handler(Some(Box::new({
        let sb = messages.clone();
        move |is_err, data| {
            assert!(!is_err);
            sb.deref()
                .lock()
                .push(std::str::from_utf8(data).expect("valid utf8").to_owned());
            std::ops::ControlFlow::Continue(())
        }
    })));

    let expected_entries = 3;
    #[cfg(all(feature = "async-client", not(feature = "blocking-client")))]
    let reader = futures_lite::io::BlockOn::new(reader);
    use gix_pack::data::input;
    let entries = gix_pack::data::input::BytesToEntriesIter::new_from_header(
        reader,
        input::Mode::Verify,
        input::EntryDataMode::Crc32,
        gix_hash::Kind::Sha1,
    )?;
    assert_eq!(entries.count(), expected_entries);

    let sidebands = Arc::try_unwrap(messages).expect("no other handle").into_inner();
    assert_eq!(sidebands.len(), 6, "…along with some status messages");

    assert_eq!(
        out.as_slice().as_bstr(),
        b"002egit-upload-pack /foo.git\x00host=example.org\x000000000ahello\n\
    000aworld\n\
    0009done\n"
            .as_bstr(),
        "it sends the correct request"
    );
    Ok(())
}

#[crate::bisync::bisync]
#[cfg_attr(feature = "blocking-client", test)]
#[cfg_attr(all(feature = "async-client", not(feature = "blocking-client")), async_std::test)]
async fn git_daemon_request_rejects_nul_and_lf() -> crate::Result {
    for (control, name) in [(b'\0', "NUL"), (b'\n', "newline")] {
        let mut invalid_path = b"/foo.git".to_vec();
        invalid_path.push(control);
        let mut invalid_host = b"example.org".to_vec();
        invalid_host.push(control);
        let cases: [(bstr::BString, String, &str, &str); 2] = [
            (
                invalid_path.into(),
                "example.org".into(),
                "path",
                "git daemon repository paths must not contain NUL or LF",
            ),
            (
                "/foo.git".into(),
                String::from_utf8(invalid_host).expect("the test host remains UTF-8"),
                "host",
                "git daemon virtual hosts must not contain NUL or LF",
            ),
        ];

        for (path, host, component, expected_error) in cases {
            let mut out = Vec::new();
            let server_response = fixture_bytes("v1/clone.response");
            let mut connection = Connection::new(
                server_response.as_slice(),
                &mut out,
                Protocol::V1,
                path,
                Some((host, None)),
                git::ConnectMode::Daemon,
                false,
            );
            let error = connection
                .handshake(Service::UploadPack, &[])
                .await
                .err()
                .expect("an invalid request must fail");
            assert_eq!(
                error.source().expect("the validation error is preserved").to_string(),
                expected_error,
                "a {name} in the {component} must prevent the request"
            );
            drop(connection);
            assert!(out.is_empty(), "an invalid request must not be written");
        }
    }
    Ok(())
}

#[crate::bisync::bisync]
#[cfg_attr(feature = "blocking-client", test)]
#[cfg_attr(all(feature = "async-client", not(feature = "blocking-client")), async_std::test)]
async fn push_v1_simulated() -> crate::Result {
    let mut out = Vec::new();
    let server_response = fixture_bytes("v1/push.response");
    let mut c = Connection::new(
        server_response.as_slice(),
        &mut out,
        Protocol::V1,
        "/foo.git",
        Some(("example.org", None)),
        git::ConnectMode::Process,
        false,
    );

    let mut writer = c.request(client::WriteMode::Binary, client::MessageKind::Flush, false)?;
    let expected = fixture_bytes("v1/push.request");
    writer.write_all(b"7c09ba0c4c3680af369bda4fc8e3c58d3fccdc76 32690d87d3943c7c0dda81246d0cde344ca7e633 refs/heads/main\0 report-status-v2 side-band-64k object-format=sha1 agent=git/2.37.1.(Apple.Git-137.1)").await?;
    writer.write_message(client::MessageKind::Flush).await?;
    {
        let (mut write, mut read) = writer.into_parts();
        write.write_all(&expected[191..]).await?;

        let messages = Arc::new(Mutex::new(Vec::<String>::new()));
        read.set_progress_handler(Some(Box::new({
            let sb = messages.clone();
            move |is_err, data| {
                assert!(!is_err);
                sb.deref()
                    .lock()
                    .push(std::str::from_utf8(data).expect("valid utf8").to_owned());
                std::ops::ControlFlow::Continue(())
            }
        })));
        let mut lines = read.lines();
        let mut info = Vec::new();
        // needed in async version of test
        while let Some(line) = lines.next().await {
            info.push(line?);
        }
        assert_eq!(
            info,
            &["000eunpack ok", "0017ok refs/heads/main", "0000"],
            "this seems to be a packetline encoding within a packetline encoding! Including a flush package. Strange, but it's the real deal."
        );
        let expected_progress = &[
            "Resolving deltas:   0% (0/2)\r",
            "Resolving deltas:  50% (1/2)\r",
            "Resolving deltas: 100% (2/2)\r",
            "Resolving deltas: 100% (2/2), completed with 2 local objects.",
            "\nGitHub found 1 vulnerability on the-lean-crate/criner's default branch (1 high). To find out more, visit:\n     https://github.com/the-lean-crate/criner/security/dependabot/1\n",
        ];
        assert_eq!(
            messages.lock().as_slice(),
            expected_progress,
            "these look like they are created once the whole pack has been received"
        );
    }

    assert_eq!(
        out.as_slice().as_bstr(),
        expected.as_bstr(),
        "we are able to reproduce a typical push request by hand with a little bit of juggling"
    );
    Ok(())
}

#[crate::bisync::bisync]
#[cfg_attr(feature = "blocking-client", test)]
#[cfg_attr(all(feature = "async-client", not(feature = "blocking-client")), async_std::test)]
async fn handshake_v1_process_mode() -> crate::Result {
    let mut out = Vec::new();
    let server_response = fixture_bytes("v1/clone.response");
    let mut c = Connection::new(
        server_response.as_slice(),
        &mut out,
        Protocol::V1,
        "/foo.git",
        Some(("example.org", None)),
        git::ConnectMode::Process,
        false,
    );
    c.handshake(Service::UploadPack, &[]).await?;

    assert_eq!(
        out.as_slice().as_bstr(),
        b"".as_bstr(),
        "it sends no introductory line to help the daemon start the right thing"
    );
    Ok(())
}

#[crate::bisync::bisync]
#[cfg_attr(feature = "blocking-client", test)]
#[cfg_attr(all(feature = "async-client", not(feature = "blocking-client")), async_std::test)]
async fn handshake_v2_downgrade_to_v1() -> crate::Result {
    let mut out = Vec::new();
    let input = fixture_bytes("v1/clone.response");
    let mut c = Connection::new(
        input.as_slice(),
        &mut out,
        Protocol::V2,
        "/bar.git",
        Some(("example.org", None)),
        git::ConnectMode::Daemon,
        false,
    );
    let res = c.handshake(Service::UploadPack, &[]).await?;
    assert_eq!(res.actual_protocol, Protocol::V1);
    assert!(
        res.refs.is_some(),
        "V1 downgrades 'just happen', so we should have refs as part of the handshake"
    );
    drop(res);

    assert_eq!(
        c.supported_protocol_versions(),
        [],
        "it doesn't care and can handle all of them"
    );
    Ok(())
}

#[crate::bisync::bisync]
#[cfg_attr(feature = "blocking-client", test)]
#[cfg_attr(all(feature = "async-client", not(feature = "blocking-client")), async_std::test)]
async fn handshake_v2_and_request() -> crate::Result {
    #[crate::bisync::only_sync]
    fn run() -> crate::Result {
        handshake_v2_and_request_inner()
    }

    #[crate::bisync::only_async]
    async fn run() -> crate::Result {
        // This simulates processing a pack received with async I/O as blocking `BufRead` without blocking the executor.
        blocking::unblock(|| futures_lite::future::block_on(handshake_v2_and_request_inner()).expect("no failure"))
            .await;
        Ok(())
    }

    run().await
}

#[crate::bisync::bisync]
async fn handshake_v2_and_request_inner() -> crate::Result {
    let mut out = Vec::new();
    let input = fixture_bytes("v2/clone.response");
    let mut c = Connection::new(
        input.as_slice(),
        &mut out,
        Protocol::V2,
        "/bar.git",
        Some(("example.org", None)),
        git::ConnectMode::Daemon,
        false,
    );
    assert!(
        c.connection_persists_across_multiple_requests(),
        "tcp connections are stateful despite the protocol version"
    );
    let res = c.handshake(Service::UploadPack, &[]).await?;
    assert_eq!(res.actual_protocol, Protocol::V2);
    assert!(
        res.refs.is_none(),
        "V2 needs a separate trip for getting refs (with additional capabilities)"
    );
    assert_eq!(
        res.capabilities
            .iter()
            .map(|c| (c.name().to_owned(), c.value().map(ToOwned::to_owned)))
            .collect::<Vec<_>>(),
        [
            ("agent", Some("git/2.28.0")),
            ("ls-refs", None),
            ("fetch", Some("shallow")),
            ("server-option", None),
            ("object-format", Some("sha1"))
        ]
        .iter()
        .map(|(k, v)| (k.as_bytes().into(), v.map(|v| v.as_bytes().into())))
        .collect::<Vec<_>>()
    );
    drop(res);

    let reader = c
        .invoke(
            "ls-refs",
            [("agent", Some("git/2.28.0")), ("object-format", Some("sha1"))]
                .iter()
                .copied(),
            Some(
                [
                    "peel",
                    "symrefs",
                    "ref-prefix HEAD",
                    "ref-prefix refs/heads/",
                    "ref-prefix refs/tags",
                ]
                .iter()
                .map(|s| s.as_bytes().as_bstr().to_owned()),
            ),
            false,
        )
        .await?;

    let mut lines = reader.lines();
    let mut refs = Vec::new();
    // needed in async version of test
    while let Some(line) = lines.next().await {
        refs.push(line?);
    }
    assert_eq!(
        refs,
        vec![
            "808e50d724f604f69ab93c6da2919c014667bedb HEAD symref-target:refs/heads/master".to_string(),
            "808e50d724f604f69ab93c6da2919c014667bedb refs/heads/master".into()
        ]
    );
    drop(lines);

    let mut reader = c
        .invoke(
            "fetch",
            [
                ("agent", Some("git/2.28.0")),
                ("something-without-value", None),
                ("object-format", Some("sha1")),
            ]
            .iter()
            .copied(),
            Some(
                [
                    "thin-pack",
                    "ofs-delta",
                    "want 808e50d724f604f69ab93c6da2919c014667bedb",
                    "done",
                ]
                .iter()
                .map(|s| s.as_bytes().as_bstr().to_owned()),
            ),
            false,
        )
        .await?;

    let mut line = String::new();
    reader.read_line(&mut line).await?;
    assert_eq!(line, "packfile\n");

    let messages = Arc::new(Mutex::new(Vec::<String>::new()));
    reader.set_progress_handler(Some(Box::new({
        let sb = messages.clone();
        move |is_err, data| {
            assert!(!is_err);
            sb.deref()
                .lock()
                .push(std::str::from_utf8(data).expect("valid utf8").to_owned());
            std::ops::ControlFlow::Continue(())
        }
    })));

    let expected_entries = 3;
    #[cfg(all(feature = "async-client", not(feature = "blocking-client")))]
    let reader = futures_lite::io::BlockOn::new(reader);

    use gix_pack::data::input;
    let entries = gix_pack::data::input::BytesToEntriesIter::new_from_header(
        reader,
        input::Mode::Verify,
        input::EntryDataMode::Crc32,
        gix_hash::Kind::Sha1,
    )?;
    assert_eq!(entries.count(), expected_entries);

    let messages = Arc::try_unwrap(messages).expect("no other handle").into_inner();
    assert_eq!(messages.len(), 4);

    assert_eq!(
        out.as_slice().as_bstr(),
        b"0039git-upload-pack /bar.git\x00host=example.org\x00\x00version=2\x000014command=ls-refs
0015agent=git/2.28.0
0017object-format=sha1
00010009peel
000csymrefs
0014ref-prefix HEAD
001bref-prefix refs/heads/
0019ref-prefix refs/tags
00000012command=fetch
0015agent=git/2.28.0
001csomething-without-value
0017object-format=sha1
0001000ethin-pack
000eofs-delta
0032want 808e50d724f604f69ab93c6da2919c014667bedb
0009done
0000"
            .as_bstr(),
        "it sends the correct request, including the adjusted version"
    );
    Ok(())
}

#[crate::bisync::bisync]
#[cfg_attr(feature = "blocking-client", test)]
#[cfg_attr(all(feature = "async-client", not(feature = "blocking-client")), async_std::test)]
async fn handshake_v2_with_sha256_object_format() -> crate::Result {
    let mut out = Vec::new();
    let input = fixture_bytes("v2/handshake-sha256.response");
    let mut c = Connection::new(
        input.as_slice(),
        &mut out,
        Protocol::V2,
        "/bar.git",
        Some(("example.org", None)),
        git::ConnectMode::Daemon,
        false,
    );
    let res = c.handshake(Service::UploadPack, &[]).await?;
    assert_eq!(res.actual_protocol, Protocol::V2);
    assert!(
        res.refs.is_none(),
        "V2 needs a separate trip for getting refs (with additional capabilities)"
    );
    assert_eq!(
        res.capabilities
            .iter()
            .map(|c| (c.name().to_owned(), c.value().map(ToOwned::to_owned)))
            .collect::<Vec<_>>(),
        [
            ("agent", Some("git/2.40.0")),
            ("ls-refs", None),
            ("fetch", Some("shallow")),
            ("server-option", None),
            ("object-format", Some("sha256")),
        ]
        .iter()
        .map(|(k, v)| (k.as_bytes().into(), v.map(|v| v.as_bytes().into())))
        .collect::<Vec<_>>(),
        "the sha256 object-format advertised by the server is surfaced from the V2 handshake"
    );
    Ok(())
}

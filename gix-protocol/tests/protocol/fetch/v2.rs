use bstr::ByteSlice;
use gix_features::progress;
use gix_protocol::{fetch, handshake, ls_refs};
use gix_transport::Protocol;

use crate::fetch::{
    _impl::FetchConnection, helper_unused, oid, transport, CloneDelegate, CloneRefInWantDelegate, LsRemoteDelegate,
};

#[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
async fn clone_abort_prep() -> crate::Result {
    let out = Vec::new();
    let mut dlg = CloneDelegate {
        abort_with: Some(std::io::Error::new(std::io::ErrorKind::Other, "hello world")),
        ..Default::default()
    };
    let mut transport = transport(
        out,
        "v2/clone.response",
        Protocol::V2,
        gix_transport::client::git::ConnectMode::Daemon,
    );
    let agent = "agent";
    let err = crate::fetch(
        &mut transport,
        &mut dlg,
        helper_unused,
        progress::Discard,
        FetchConnection::TerminateOnSuccessfulCompletion,
        "agent",
        false,
    )
    .await
    .expect_err("fetch aborted");
    assert_eq!(dlg.pack_bytes, 0, "we aborted before fetching");

    assert_eq!(
        transport.into_inner().1.as_bstr(),
        format!(
            "002fgit-upload-pack does/not/matter\0\0version=2\00014command=ls-refs
0014agent={}
0001000csymrefs
0009peel
00000000",
            gix_protocol::agent(agent)
        )
        .as_bytes()
        .as_bstr()
    );
    match err {
        fetch::Error::Io(err) => {
            assert_eq!(err.kind(), std::io::ErrorKind::Other);
            assert_eq!(err.get_ref().expect("other error").to_string(), "hello world");
        }
        _ => panic!("should not have another error here"),
    }
    Ok(())
}

#[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
async fn ls_remote() -> crate::Result {
    let out = Vec::new();
    let mut delegate = LsRemoteDelegate::default();
    let mut transport = transport(
        out,
        "v2/clone.response",
        Protocol::V2,
        gix_transport::client::git::ConnectMode::Daemon,
    );
    let agent = "agent";
    crate::fetch(
        &mut transport,
        &mut delegate,
        helper_unused,
        progress::Discard,
        FetchConnection::AllowReuse,
        "agent",
        false,
    )
    .await?;

    assert_eq!(
        delegate.refs,
        vec![
            handshake::Ref::Symbolic {
                full_ref_name: "HEAD".into(),
                object: oid("808e50d724f604f69ab93c6da2919c014667bedb"),
                tag: None,
                target: "refs/heads/master".into()
            },
            handshake::Ref::Direct {
                full_ref_name: "refs/heads/master".into(),
                object: oid("808e50d724f604f69ab93c6da2919c014667bedb")
            }
        ]
    );
    assert_eq!(
        transport.into_inner().1.as_bstr(),
        format!(
            "0044git-upload-pack does/not/matter\0\0version=2\0value-only\0key=value\00014command=ls-refs
0014agent={}
0001000csymrefs
0009peel
0000",
            gix_protocol::agent(agent)
        )
        .as_bytes()
        .as_bstr(),
        "the delegate is configured to not emit the final flush message, to potentially run more commands on this connection"
    );
    Ok(())
}

#[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
async fn ls_remote_abort_in_prep_ls_refs() -> crate::Result {
    let out = Vec::new();
    let mut delegate = LsRemoteDelegate {
        abort_with: Some(std::io::Error::new(std::io::ErrorKind::Other, "hello world")),
        ..Default::default()
    };
    let mut transport = transport(
        out,
        "v2/clone.response",
        Protocol::V2,
        gix_transport::client::git::ConnectMode::Daemon,
    );
    let err = crate::fetch(
        &mut transport,
        &mut delegate,
        helper_unused,
        progress::Discard,
        FetchConnection::AllowReuse,
        "agent",
        false,
    )
    .await
    .expect_err("ls-refs preparation is aborted");

    assert!(delegate.refs.is_empty(), "no refs are fetched");
    assert_eq!(
        transport.into_inner().1.as_bstr(),
        b"0044git-upload-pack does/not/matter\x00\x00version=2\x00value-only\x00key=value\x000000".as_bstr()
    );
    match err {
        fetch::Error::LsRefs(ls_refs::Error::Io(err)) => {
            assert_eq!(err.kind(), std::io::ErrorKind::Other);
            assert_eq!(err.get_ref().expect("other error").to_string(), "hello world");
        }
        err => panic!("should not have another error here, got: {err}"),
    }
    Ok(())
}

#[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
async fn ref_in_want() -> crate::Result {
    let out = Vec::new();
    let mut delegate = CloneRefInWantDelegate {
        want_refs: vec!["refs/heads/main".into()],
        ..CloneRefInWantDelegate::default()
    };
    let mut transport = transport(
        out,
        "v2/clone-ref-in-want.response",
        Protocol::V2,
        gix_transport::client::git::ConnectMode::Daemon,
    );

    let agent = "agent";
    crate::fetch(
        &mut transport,
        &mut delegate,
        helper_unused,
        progress::Discard,
        FetchConnection::TerminateOnSuccessfulCompletion,
        "agent",
        false,
    )
    .await?;

    assert!(delegate.refs.is_empty(), "Should not receive any ref advertisement");
    assert_eq!(
        delegate.wanted_refs,
        vec![handshake::Ref::Direct {
            full_ref_name: "refs/heads/main".into(),
            object: oid("9e320b9180e0b5580af68fa3255b7f3d9ecd5af0"),
        }]
    );
    assert_eq!(delegate.pack_bytes, 641, "Should get packfile");
    assert_eq!(
        transport.into_inner().1.as_bstr(),
        format!(
            "002fgit-upload-pack does/not/matter\0\0version=2\00012command=fetch
0014agent={}
0001000ethin-pack
000eofs-delta
001dwant-ref refs/heads/main
0009done
00000000",
            gix_protocol::agent(agent)
        )
        .as_bytes()
        .as_bstr()
    );

    Ok(())
}

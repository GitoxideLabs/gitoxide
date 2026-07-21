use std::io;

use bstr::{BString, ByteSlice};
use gix_protocol::{
    command::Feature,
    fetch::{Arguments, Response},
    handshake,
};
use gix_transport::client::Capabilities;

use crate::fixture_bytes;

pub(super) mod _impl;
use _impl::{Action, DelegateBlocking, RefsAction};

mod ref_map;

mod error {
    use std::io;

    use gix_protocol::{fetch::response, handshake, ls_refs};
    use gix_transport::client;

    /// The error used in [`fetch()`][crate::fetch()].
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        Handshake(handshake::Error),
        Io(io::Error),
        Transport(client::Error),
        LsRefs(ls_refs::Error),
        Response(response::Error),
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Handshake(err) => std::fmt::Display::fmt(err, f),
                Error::Io(_) => f.write_str("Could not access repository or failed to read streaming pack file"),
                Error::Transport(err) => std::fmt::Display::fmt(err, f),
                Error::LsRefs(err) => std::fmt::Display::fmt(err, f),
                Error::Response(err) => std::fmt::Display::fmt(err, f),
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::Handshake(err) => err.source(),
                Error::Io(err) => Some(err),
                Error::Transport(err) => err.source(),
                Error::LsRefs(err) => err.source(),
                Error::Response(err) => err.source(),
            }
        }
    }

    impl From<handshake::Error> for Error {
        fn from(err: handshake::Error) -> Self {
            Error::Handshake(err)
        }
    }

    impl From<io::Error> for Error {
        fn from(err: io::Error) -> Self {
            Error::Io(err)
        }
    }

    impl From<client::Error> for Error {
        fn from(err: client::Error) -> Self {
            Error::Transport(err)
        }
    }

    impl From<ls_refs::Error> for Error {
        fn from(err: ls_refs::Error) -> Self {
            Error::LsRefs(err)
        }
    }

    impl From<response::Error> for Error {
        fn from(err: response::Error) -> Self {
            Error::Response(err)
        }
    }
}
pub use error::Error;

mod arguments;

#[cfg(feature = "blocking-client")]
type Cursor = std::io::Cursor<Vec<u8>>;
#[cfg(all(feature = "async-client", not(feature = "blocking-client")))]
type Cursor = futures_lite::io::Cursor<Vec<u8>>;

#[expect(clippy::result_large_err)]
fn helper_unused(_action: gix_credentials::helper::Action) -> gix_credentials::protocol::Result {
    panic!("Call to credentials helper is unexpected")
}

#[derive(Default)]
pub struct CloneDelegate {
    pack_bytes: usize,
    abort_with: Option<std::io::Error>,
}

impl DelegateBlocking for CloneDelegate {
    fn prepare_fetch(
        &mut self,
        _version: gix_transport::Protocol,
        _server: &Capabilities,
        _features: &mut Vec<Feature>,
        _refs: &[handshake::Ref],
    ) -> io::Result<Action> {
        if _refs.is_empty() {
            return Ok(Action::Cancel);
        }
        match self.abort_with.take() {
            Some(err) => Err(err),
            None => Ok(Action::Continue),
        }
    }
    fn negotiate(
        &mut self,
        refs: &[handshake::Ref],
        arguments: &mut Arguments,
        _previous_response: Option<&Response>,
    ) -> io::Result<Action> {
        for r in refs {
            if let Some(id) = r.unpack().1 {
                arguments.want(id);
            }
        }
        Ok(Action::Cancel)
    }
}

/// A delegate which bypasses refs negotiation entirely via `ref-in-want`.
#[derive(Default)]
pub struct CloneRefInWantDelegate {
    /// Statically-known refs we want.
    want_refs: Vec<BString>,

    /// Number of bytes received of the final packfile.
    pack_bytes: usize,

    /// Refs advertised by `ls-refs` -- should always be empty, as we skip `ls-refs`.
    refs: Vec<handshake::Ref>,

    /// Refs advertised as `wanted-ref` -- should always match `want_refs`
    wanted_refs: Vec<handshake::Ref>,
}

impl DelegateBlocking for CloneRefInWantDelegate {
    fn action(&mut self) -> io::Result<RefsAction> {
        Ok(RefsAction::Skip)
    }

    fn prepare_fetch(
        &mut self,
        _version: gix_transport::Protocol,
        _server: &Capabilities,
        _features: &mut Vec<Feature>,
        refs: &[handshake::Ref],
    ) -> io::Result<Action> {
        refs.clone_into(&mut self.refs);
        Ok(Action::Continue)
    }

    fn negotiate(
        &mut self,
        _refs: &[handshake::Ref],
        arguments: &mut Arguments,
        _prev: Option<&Response>,
    ) -> io::Result<Action> {
        for wanted_ref in &self.want_refs {
            arguments.want_ref(wanted_ref.as_ref());
        }

        Ok(Action::Cancel)
    }
}

#[derive(Default)]
pub struct LsRemoteDelegate {
    refs: Vec<handshake::Ref>,
    abort_with: Option<std::io::Error>,
}

impl DelegateBlocking for LsRemoteDelegate {
    fn handshake_extra_parameters(&self) -> Vec<(String, Option<String>)> {
        vec![("value-only".into(), None), ("key".into(), Some("value".into()))]
    }
    fn action(&mut self) -> std::io::Result<RefsAction> {
        match self.abort_with.take() {
            Some(err) => Err(err),
            None => Ok(RefsAction::Continue),
        }
    }
    fn prepare_fetch(
        &mut self,
        _version: gix_transport::Protocol,
        _server: &Capabilities,
        _features: &mut Vec<Feature>,
        refs: &[handshake::Ref],
    ) -> io::Result<Action> {
        refs.clone_into(&mut self.refs);
        Ok(Action::Cancel)
    }
    fn negotiate(
        &mut self,
        _refs: &[handshake::Ref],
        _arguments: &mut Arguments,
        _previous_response: Option<&Response>,
    ) -> io::Result<Action> {
        unreachable!("this must not be called after closing the connection in `prepare_fetch(…)`")
    }
}

#[cfg(feature = "blocking-client")]
mod blocking_io {
    use std::io;

    use gix_features::progress::NestedProgress;
    use gix_protocol::{fetch::Response, handshake, handshake::Ref};

    use super::_impl::Delegate;
    use crate::fetch::{CloneDelegate, CloneRefInWantDelegate, LsRemoteDelegate};

    impl Delegate for CloneDelegate {
        fn receive_pack(
            &mut self,
            mut input: impl io::BufRead,
            _progress: impl NestedProgress,
            _refs: &[Ref],
            _previous_response: &Response,
        ) -> io::Result<()> {
            self.pack_bytes = io::copy(&mut input, &mut io::sink())? as usize;
            Ok(())
        }
    }

    impl Delegate for CloneRefInWantDelegate {
        fn receive_pack(
            &mut self,
            mut input: impl io::BufRead,
            _progress: impl NestedProgress,
            _refs: &[Ref],
            response: &Response,
        ) -> io::Result<()> {
            for wanted in response.wanted_refs() {
                self.wanted_refs.push(handshake::Ref::Direct {
                    full_ref_name: wanted.path.clone(),
                    object: wanted.id,
                });
            }
            self.pack_bytes = io::copy(&mut input, &mut io::sink())? as usize;
            Ok(())
        }
    }

    impl Delegate for LsRemoteDelegate {
        fn receive_pack(
            &mut self,
            _input: impl io::BufRead,
            _progress: impl NestedProgress,
            _refs: &[Ref],
            _previous_response: &Response,
        ) -> io::Result<()> {
            unreachable!("Should not be called for ls-refs");
        }
    }
}

#[cfg(all(feature = "async-client", not(feature = "blocking-client")))]
mod async_io {
    use std::io;

    use async_trait::async_trait;
    use futures_io::AsyncBufRead;
    use gix_features::progress::NestedProgress;
    use gix_protocol::{fetch::Response, handshake, handshake::Ref};

    use super::_impl::Delegate;
    use crate::fetch::{CloneDelegate, CloneRefInWantDelegate, LsRemoteDelegate};

    #[async_trait(?Send)]
    impl Delegate for CloneDelegate {
        async fn receive_pack(
            &mut self,
            mut input: impl AsyncBufRead + Unpin + 'async_trait,
            _progress: impl NestedProgress,
            _refs: &[Ref],
            _previous_response: &Response,
        ) -> io::Result<()> {
            self.pack_bytes = futures_lite::io::copy(&mut input, &mut futures_lite::io::sink()).await? as usize;
            Ok(())
        }
    }

    #[async_trait(?Send)]
    impl Delegate for CloneRefInWantDelegate {
        async fn receive_pack(
            &mut self,
            mut input: impl AsyncBufRead + Unpin + 'async_trait,
            _progress: impl NestedProgress,
            _refs: &[Ref],
            response: &Response,
        ) -> io::Result<()> {
            for wanted in response.wanted_refs() {
                self.wanted_refs.push(handshake::Ref::Direct {
                    full_ref_name: wanted.path.clone(),
                    object: wanted.id,
                });
            }
            self.pack_bytes = futures_lite::io::copy(&mut input, &mut futures_lite::io::sink()).await? as usize;
            Ok(())
        }
    }

    #[async_trait(?Send)]
    impl Delegate for LsRemoteDelegate {
        async fn receive_pack(
            &mut self,
            _input: impl AsyncBufRead + Unpin + 'async_trait,
            _progress: impl NestedProgress,
            _refs: &[Ref],
            _previous_response: &Response,
        ) -> io::Result<()> {
            unreachable!("Should not be called for ls-refs");
        }
    }
}

pub fn oid(hex_sha: &str) -> gix_hash::ObjectId {
    gix_hash::ObjectId::from_hex(hex_sha.as_bytes()).expect("valid input")
}

#[cfg(all(feature = "async-client", not(feature = "blocking-client")))]
pub fn transport<W: futures_io::AsyncWrite + Unpin>(
    out: W,
    path: &str,
    desired_version: gix_transport::Protocol,
    mode: gix_transport::client::git::ConnectMode,
) -> gix_transport::client::git::async_io::Connection<Cursor, W> {
    let response = fixture_bytes(path);
    gix_transport::client::git::async_io::Connection::new(
        Cursor::new(response),
        out,
        desired_version,
        b"does/not/matter".as_bstr().to_owned(),
        None::<(&str, _)>,
        mode,
        false,
    )
}

#[cfg(feature = "blocking-client")]
pub fn transport<W: std::io::Write>(
    out: W,
    path: &str,
    version: gix_transport::Protocol,
    mode: gix_transport::client::git::ConnectMode,
) -> gix_transport::client::git::blocking_io::Connection<Cursor, W> {
    let response = fixture_bytes(path);
    gix_transport::client::git::blocking_io::Connection::new(
        Cursor::new(response),
        out,
        version,
        b"does/not/matter".as_bstr().to_owned(),
        None::<(&str, _)>,
        mode,
        false,
    )
}

pub mod response;
mod v1;
mod v2;

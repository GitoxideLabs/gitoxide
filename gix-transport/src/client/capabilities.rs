use bstr::{BStr, BString, ByteSlice};

#[cfg(any(feature = "blocking-client", feature = "async-client"))]
use crate::client;
use crate::Protocol;

/// A structure to represent multiple [capabilities](Capability) or features supported by the server.
///
/// ### Deviation
///
/// As a *shortcoming*, we are unable to parse `V1` as emitted from `git-upload-pack` without a `git-daemon` or server,
/// as it will not emit any capabilities for some reason. Only `V2` and `V0` work in that context.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Capabilities {
    data: BString,
    value_sep: u8,
}

/// This implementation yields exactly those minimal capabilities that are required for `gix` to work, nothing more and nothing less.
///
/// This is a bit of a hack just get tests with Protocol V0 to work, which is a good way to enforce stateful transports.
/// Of course, V1 would also do that but when calling `git-upload-pack` directly, it advertises so badly that this is easier to implement.
impl Default for Capabilities {
    fn default() -> Self {
        Capabilities::from_lines("version 2\nmulti_ack_detailed\nside-band-64k\n".into())
            .expect("valid format, known at compile time")
    }
}

/// The name of a single capability.
pub struct Capability<'a>(&'a BStr);

impl<'a> Capability<'a> {
    /// Returns the name of the capability.
    ///
    /// Most capabilities only consist of a name, making them appear like a feature toggle.
    pub fn name(&self) -> &'a BStr {
        self.0
            .splitn(2, |b| *b == b'=')
            .next()
            .expect("there is always a single item")
            .as_bstr()
    }
    /// Returns the value associated with the capability.
    ///
    /// Note that the caller must know whether a single or multiple values are expected, in which
    /// case [`values()`](Capability::values()) should be called.
    pub fn value(&self) -> Option<&'a BStr> {
        self.0.splitn(2, |b| *b == b'=').nth(1).map(ByteSlice::as_bstr)
    }
    /// Returns the values of a capability if its [`value()`](Capability::value()) is space separated.
    pub fn values(&self) -> Option<impl Iterator<Item = &'a BStr>> {
        self.value().map(|v| v.split(|b| *b == b' ').map(ByteSlice::as_bstr))
    }
    /// Returns true if its space-separated [`value()`](Capability::value()) contains the given `want`ed capability.
    pub fn supports(&self, want: impl Into<&'a BStr>) -> Option<bool> {
        let want = want.into();
        self.values().map(|mut iter| iter.any(|v| v == want))
    }
}

impl Capabilities {
    /// Parse capabilities from the given `bytes`.
    ///
    /// Useful in case they are encoded within a `ref` behind a null byte.
    pub fn from_bytes(bytes: &[u8]) -> Result<(Capabilities, usize), crate::Error> {
        let delimiter_pos = bytes.find_byte(0).ok_or(crate::Error::MissingDelimitingNullByte)?;
        if delimiter_pos + 1 == bytes.len() {
            return Err(crate::Error::NoCapabilities);
        }
        let capabilities = &bytes[delimiter_pos + 1..];
        Ok((
            Capabilities {
                data: capabilities.as_bstr().to_owned(),
                value_sep: b' ',
            },
            delimiter_pos,
        ))
    }

    /// Parse capabilities from the given a `lines_buf` which is expected to be all newline separated lines
    /// from the server.
    ///
    /// Useful for parsing capabilities from a data sent from a server, and to avoid having to deal with
    /// blocking and async traits for as long as possible. There is no value in parsing a few bytes
    /// in a non-blocking fashion.
    pub fn from_lines(lines_buf: BString) -> Result<Capabilities, crate::Error> {
        let mut lines = <_ as bstr::ByteSlice>::lines(lines_buf.as_slice().trim());
        let version_line = lines.next().ok_or(crate::Error::MissingVersionLine)?;
        let (name, value) = version_line.split_at(
            version_line
                .find(b" ")
                .ok_or_else(|| crate::Error::MalformattedVersionLine(version_line.to_owned().into()))?,
        );
        if name != b"version" {
            return Err(crate::Error::MalformattedVersionLine(version_line.to_owned().into()));
        }
        if value != b" 2" {
            return Err(crate::Error::UnsupportedVersion {
                desired: Protocol::V2,
                actual: value.to_owned().into(),
            });
        }
        Ok(Capabilities {
            value_sep: b'\n',
            data: lines.as_bytes().into(),
        })
    }

    /// Returns true of the given `feature` is mentioned in this list of capabilities.
    pub fn contains(&self, feature: &str) -> bool {
        self.capability(feature).is_some()
    }

    /// Returns the capability with `name`.
    pub fn capability(&self, name: &str) -> Option<Capability<'_>> {
        self.iter().find(|c| c.name() == name.as_bytes().as_bstr())
    }

    /// Returns an iterator over all capabilities.
    pub fn iter(&self) -> impl Iterator<Item = Capability<'_>> {
        self.data
            .split(move |b| *b == self.value_sep)
            .map(|c| Capability(c.as_bstr()))
    }
}

/// internal use
#[cfg(any(feature = "blocking-client", feature = "async-client"))]
impl Capabilities {
    fn extract_protocol(capabilities_or_version: gix_packetline::TextRef<'_>) -> Result<Protocol, client::Error> {
        let line = capabilities_or_version.as_bstr();
        let version = if line.starts_with_str("version ") {
            if line.len() != "version X".len() {
                return Err(client::Error::UnsupportedProtocolVersion(line.as_bstr().into()));
            }
            match line {
                line if line.ends_with_str("1") => Protocol::V1,
                line if line.ends_with_str("2") => Protocol::V2,
                _ => return Err(client::Error::UnsupportedProtocolVersion(line.as_bstr().into())),
            }
        } else {
            Protocol::V1
        };
        Ok(version)
    }
}

///
#[cfg(feature = "blocking-client")]
pub mod blocking_recv {
    use std::io;

    use bstr::ByteVec;

    use crate::{
        client::{self, blocking_io::ReadlineBufRead, Capabilities},
        packetline::blocking_io::StreamingPeekableIter,
        Protocol,
    };

    /// The information provided by the server upon first connection.
    pub struct Handshake<'a> {
        /// The [`Capabilities`] the remote advertised.
        pub capabilities: Capabilities,
        /// The remote refs as a [`io::BufRead`].
        ///
        /// This is `Some` only when protocol v1 is used. The [`io::BufRead`] must be exhausted by
        /// the caller.
        pub refs: Option<Box<dyn ReadlineBufRead + 'a>>,
        /// The [`Protocol`] the remote advertised.
        pub protocol: Protocol,
    }

    impl Handshake<'_> {
        /// Read the capabilities and version advertisement from the given packetline reader.
        ///
        /// If [`Protocol::V1`] was requested, or the remote decided to downgrade, the remote refs
        /// advertisement will also be included in the [`Handshake`].
        pub fn from_lines_with_version_detection<T: io::Read>(
            rd: &mut StreamingPeekableIter<T>,
        ) -> Result<Handshake<'_>, client::Error> {
            // NOTE that this is vitally important - it is turned on and stays on for all following requests so
            // we automatically abort if the server sends an ERR line anywhere.
            // We are sure this can't clash with binary data when sent due to the way the PACK
            // format looks like, thus there is no binary blob that could ever look like an ERR line by accident.
            rd.fail_on_err_lines(true);

            Ok(match rd.peek_line() {
                Some(line) => {
                    let line = line??.as_text().ok_or(client::Error::ExpectedLine("text"))?;
                    let version = Capabilities::extract_protocol(line)?;
                    match version {
                        Protocol::V0 => unreachable!("already handled in `None` case"),
                        Protocol::V1 => {
                            let (capabilities, delimiter_position) = Capabilities::from_bytes(line.0)?;
                            rd.peek_buffer_replace_and_truncate(delimiter_position, b'\n');
                            Handshake {
                                capabilities,
                                refs: Some(Box::new(rd.as_read())),
                                protocol: Protocol::V1,
                            }
                        }
                        Protocol::V2 => Handshake {
                            capabilities: {
                                let mut rd = rd.as_read();
                                let mut buf = Vec::new();
                                while let Some(line) = rd.read_data_line() {
                                    let line = line??;
                                    match line.as_bstr() {
                                        Some(line) => {
                                            buf.push_str(line);
                                            if buf.last() != Some(&b'\n') {
                                                buf.push(b'\n');
                                            }
                                        }
                                        None => break,
                                    }
                                }
                                Capabilities::from_lines(buf.into())?
                            },
                            refs: None,
                            protocol: Protocol::V2,
                        },
                    }
                }
                None => Handshake {
                    capabilities: Capabilities::default(),
                    refs: Some(Box::new(rd.as_read())),
                    protocol: Protocol::V0,
                },
            })
        }
    }
}

///
#[cfg(feature = "async-client")]
#[allow(missing_docs)]
pub mod async_recv {
    use bstr::ByteVec;
    use futures_io::AsyncRead;

    use crate::{
        client::{self, async_io::ReadlineBufRead, Capabilities},
        packetline::async_io::StreamingPeekableIter,
        Protocol,
    };

    /// The information provided by the server upon first connection.
    pub struct Handshake<'a> {
        /// The [`Capabilities`] the remote advertised.
        pub capabilities: Capabilities,
        /// The remote refs as an [`AsyncBufRead`].
        ///
        /// This is `Some` only when protocol v1 is used. The [`AsyncRead`] must be exhausted by
        /// the caller.
        pub refs: Option<Box<dyn ReadlineBufRead + Unpin + 'a>>,
        /// The [`Protocol`] the remote advertised.
        pub protocol: Protocol,
    }

    impl Handshake<'_> {
        /// Read the capabilities and version advertisement from the given packetline reader.
        ///
        /// If [`Protocol::V1`] was requested, or the remote decided to downgrade, the remote refs
        /// advertisement will also be included in the [`Handshake`].
        pub async fn from_lines_with_version_detection<T: AsyncRead + Unpin>(
            rd: &mut StreamingPeekableIter<T>,
        ) -> Result<Handshake<'_>, client::Error> {
            // NOTE that this is vitally important - it is turned on and stays on for all following requests so
            // we automatically abort if the server sends an ERR line anywhere.
            // We are sure this can't clash with binary data when sent due to the way the PACK
            // format looks like, thus there is no binary blob that could ever look like an ERR line by accident.
            rd.fail_on_err_lines(true);

            Ok(match rd.peek_line().await {
                Some(line) => {
                    let line = line??.as_text().ok_or(client::Error::ExpectedLine("text"))?;
                    let version = Capabilities::extract_protocol(line)?;
                    match version {
                        Protocol::V0 => unreachable!("already handled in `None` case"),
                        Protocol::V1 => {
                            let (capabilities, delimiter_position) = Capabilities::from_bytes(line.0)?;
                            rd.peek_buffer_replace_and_truncate(delimiter_position, b'\n');
                            Handshake {
                                capabilities,
                                refs: Some(Box::new(rd.as_read())),
                                protocol: Protocol::V1,
                            }
                        }
                        Protocol::V2 => Handshake {
                            capabilities: {
                                let mut rd = rd.as_read();
                                let mut buf = Vec::new();
                                while let Some(line) = rd.read_data_line().await {
                                    let line = line??;
                                    match line.as_bstr() {
                                        Some(line) => {
                                            buf.push_str(line);
                                            if buf.last() != Some(&b'\n') {
                                                buf.push(b'\n');
                                            }
                                        }
                                        None => break,
                                    }
                                }
                                Capabilities::from_lines(buf.into())?
                            },
                            refs: None,
                            protocol: Protocol::V2,
                        },
                    }
                }
                None => Handshake {
                    capabilities: Capabilities::default(),
                    refs: Some(Box::new(rd.as_read())),
                    protocol: Protocol::V0,
                },
            })
        }
    }
}

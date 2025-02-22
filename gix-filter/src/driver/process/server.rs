use std::{collections::HashSet, io::Write, str::FromStr};

use bstr::{BString, ByteSlice};

use crate::driver::process::Server;

/// A request to be handled by the server, typically done in a loop.
pub struct Request<'a> {
    parent: &'a mut Server,
    /// The command to execute with this request.
    pub command: String,
    /// A list of key-value pairs of meta-data related to `command`.
    pub meta: Vec<(String, BString)>,
}

///
pub mod next_request {
    use bstr::BString;

    /// The error returned by [Server::next_request()][super::Server::next_request()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("Failed to read from the client")]
        Io(#[from] std::io::Error),
        #[error("{msg} '{actual}'")]
        Protocol { msg: String, actual: BString },
        #[error(transparent)]
        PacketlineDecode(#[from] gix_packetline::decode::Error),
    }
}

///
pub mod handshake {
    /// The error returned by [Server::handshake()][super::Server::handshake()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("Failed to read or write to the client")]
        Io(#[from] std::io::Error),
        #[error("{msg} '{actual}'")]
        Protocol { msg: String, actual: String },
        #[error("Could not select supported version from the one sent by the client: {}", actual.iter().map(ToString::to_string).collect::<Vec<_>>().join(", "))]
        VersionMismatch { actual: Vec<usize> },
    }
}

impl Server {
    /// Perform a handshake with the client sending information to our `stdin` and receiving information through our `stdout`
    /// in packetline format.
    /// `pick_version` is called with all versions supported by the client to pick one from, or `None` to indicate the handshake
    /// should stop.
    /// Use `available_capabilities` to match our capabilities with the ones from the client, so we communicate at most a subset of these.
    ///
    /// ### Note
    ///
    /// The server claims exclusive access over stdout and stdin, so all kinds of other output has to be steered towards stderr or there
    /// will be a deadlock.
    pub fn handshake(
        stdin: std::io::Stdin,
        stdout: std::io::Stdout,
        welcome_prefix: &str,
        pick_version: &mut dyn FnMut(&[usize]) -> Option<usize>,
        available_capabilities: &[&str],
    ) -> Result<Self, handshake::Error> {
        let mut input = gix_packetline::StreamingPeekableIter::new(
            stdin.lock(),
            &[gix_packetline::PacketLineRef::Flush],
            false, /* packet tracing */
        );
        let mut read = input.as_read();
        let mut buf = String::new();
        read.read_line_to_string(&mut buf)?;
        if buf
            .strip_prefix(welcome_prefix)
            .map_or(true, |rest| rest.trim_end() != "-client")
        {
            return Err(handshake::Error::Protocol {
                msg: format!("Expected '{welcome_prefix}-client, got"),
                actual: buf,
            });
        }

        let mut versions = Vec::new();
        loop {
            buf.clear();
            let num_read = read.read_line_to_string(&mut buf)?;
            if num_read == 0 {
                break;
            }
            versions.push(
                match buf
                    .strip_prefix("version=")
                    .and_then(|version| usize::from_str(version.trim_end()).ok())
                {
                    Some(version) => version,
                    None => {
                        return Err(handshake::Error::Protocol {
                            msg: "Expected 'version=<integer>', got".into(),
                            actual: buf,
                        })
                    }
                },
            );
        }
        let version = pick_version(&versions).ok_or(handshake::Error::VersionMismatch { actual: versions })?;
        read.reset_with(&[gix_packetline::PacketLineRef::Flush]);
        let mut out = gix_packetline::Writer::new(stdout.lock());
        out.write_all(format!("{welcome_prefix}-server").as_bytes())?;
        out.write_all(format!("version={version}").as_bytes())?;
        gix_packetline::encode::flush_to_write(out.inner_mut())?;
        out.flush()?;

        let mut capabilities = HashSet::new();
        loop {
            buf.clear();
            let num_read = read.read_line_to_string(&mut buf)?;
            if num_read == 0 {
                break;
            }
            match buf.strip_prefix("capability=") {
                Some(cap) => {
                    let cap = cap.trim_end();
                    if available_capabilities.contains(&cap) {
                        capabilities.insert(cap.to_owned());
                    }
                }
                None => continue,
            }
        }

        for cap in &capabilities {
            out.write_all(format!("capability={cap}").as_bytes())?;
        }
        gix_packetline::encode::flush_to_write(out.inner_mut())?;
        out.flush()?;

        drop(read);
        Ok(Server {
            capabilities,
            version,
            out,
            input,
        })
    }

    /// Read the next request and return it, even if [`command`][Request::command] is *not* supported by us.
    /// If `Ok(None)` is reported, the request loop should end and the process should be shutdown gracefully.
    ///
    /// The reason for allowing any command is that the caller would have to match on the command anyway, and would
    /// have to handle invalid commands that way.
    ///
    /// ### Lifecycle
    ///
    /// Note that the process is supposed to shut-down once there are no more requests, and `git` will wait
    /// until it has finished.
    pub fn next_request(&mut self) -> Result<Option<Request<'_>>, next_request::Error> {
        let mut buf = String::new();
        let mut read = self.input.as_read();

        match read.read_line_to_string(&mut buf) {
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(err) => return Err(err.into()),
        }
        let command = match buf.strip_prefix("command=").map(str::trim_end).map(ToOwned::to_owned) {
            Some(cmd) => cmd,
            None => {
                return Err(next_request::Error::Protocol {
                    msg: "Wanted 'command=<name>', got ".into(),
                    actual: buf.into(),
                })
            }
        };

        let mut meta = Vec::with_capacity(1);
        while let Some(res) = read.read_data_line() {
            let line = res??;
            let line = line
                .as_bstr()
                .ok_or_else(|| next_request::Error::Protocol {
                    msg: "expected data line, got ".into(),
                    actual: format!("{line:?}").into(),
                })?
                .trim();
            let mut tokens = line.splitn(2, |b| *b == b'=');
            let (key, value) = tokens
                .next()
                .zip(tokens.next())
                .ok_or_else(|| next_request::Error::Protocol {
                    msg: "Expected 'key=value' metadata, got".into(),
                    actual: line.into(),
                })?;
            assert!(tokens.next().is_none(), "configured to yield at most two tokens");
            meta.push((key.as_bstr().to_string(), value.into()));
        }

        drop(read);
        self.input.reset_with(&[gix_packetline::PacketLineRef::Flush]);

        Ok(Some(Request {
            parent: self,
            command,
            meta,
        }))
    }
}

mod request {
    use std::io::Write;

    use crate::driver::{
        process,
        process::{server::Request, PacketlineReader},
    };

    impl Request<'_> {
        /// Turn ourselves into a reader that can read until the next flush packet.
        pub fn as_read(&mut self) -> PacketlineReader<'_, std::io::StdinLock<'static>> {
            self.parent.input.as_read()
        }

        /// Provide the write-end of the underlying process.
        pub fn as_write(&mut self) -> impl std::io::Write + '_ {
            WriteAndFlushOnDrop {
                inner: &mut self.parent.out,
            }
        }

        /// Write the `status` message followed by a flush packet.
        pub fn write_status(&mut self, status: process::Status) -> std::io::Result<()> {
            let out = &mut self.parent.out;
            if let Some(message) = status.message() {
                out.write_all(format!("status={message}").as_bytes())?;
            }
            gix_packetline::encode::flush_to_write(out.inner_mut())?;
            out.flush()
        }
    }

    impl std::fmt::Debug for Request<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Request")
                .field("command", &self.command)
                .field("meta", &self.meta)
                .finish()
        }
    }

    struct WriteAndFlushOnDrop<'a> {
        inner: &'a mut gix_packetline::Writer<std::io::StdoutLock<'static>>,
    }

    impl std::io::Write for WriteAndFlushOnDrop<'_> {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.inner.write(buf)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.inner.flush()
        }
    }

    impl Drop for WriteAndFlushOnDrop<'_> {
        fn drop(&mut self) {
            gix_packetline::encode::flush_to_write(self.inner.inner_mut()).ok();
            self.inner.flush().ok();
        }
    }
}

/// Access
impl Server {
    /// Return the list of capabilities we are allowed to use, as negotiated with the client.
    pub fn capabilities(&self) -> &HashSet<String> {
        &self.capabilities
    }

    /// Return the negotiated version of the protocol.
    pub fn version(&self) -> usize {
        self.version
    }
}

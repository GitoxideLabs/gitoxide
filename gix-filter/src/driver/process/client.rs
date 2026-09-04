use std::{collections::HashSet, io::Write, str::FromStr};

use bstr::{BStr, BString, ByteVec};
use gix_packetline::blocking_io::{StreamingPeekableIter, Writer, encode};

use crate::driver::{
    process,
    process::{Capabilities, Client, PacketlineReader},
};

///
pub mod handshake {
    /// The error returned by [Client::handshake()][super::Client::handshake()].
    pub type Error = gix_error::Exn<gix_error::Message>;
}

///
pub mod invoke {
    /// The error returned by [Client::invoke()][super::Client::invoke()].
    pub type Error = gix_error::Exn<gix_error::Message>;

    ///
    pub mod without_content {
        /// The error returned by [Client::invoke_without_content()][super::super::Client::invoke_without_content()].
        pub type Error = gix_error::Exn<gix_error::Message>;
    }
}

/// Protocol implementation
impl Client {
    /// Given a spawned `process` as created from `cmd`, use the 'long-running-process' protocol to send `welcome-prefix` and supported
    /// `versions`, along with the `desired_capabilities`, and perform the handshake to negotiate a version to use along with
    /// obtaining supported capabilities, which may be a sub-set of the desired capabilities.
    pub fn handshake(
        mut process: std::process::Child,
        welcome_prefix: &str,
        versions: &[usize],
        desired_capabilities: &[&str],
    ) -> Result<Self, handshake::Error> {
        use gix_error::{ErrorExt, ResultExt, message};

        let mut out = Writer::new(process.stdin.take().expect("configured stdin when spawning"));
        out.write_all(format!("{welcome_prefix}-client").as_bytes())
            .or_raise(|| message("Failed to read or write to the process"))?;
        for version in versions {
            out.write_all(format!("version={version}").as_bytes())
                .or_raise(|| message("Failed to read or write to the process"))?;
        }
        encode::flush_to_write(out.inner_mut()).or_raise(|| message("Failed to read or write to the process"))?;
        out.flush()
            .or_raise(|| message("Failed to read or write to the process"))?;

        let mut input = StreamingPeekableIter::new(
            process.stdout.take().expect("configured stdout when spawning"),
            &[gix_packetline::PacketLineRef::Flush],
            false, /* packet tracing */
        );
        let mut read = input.as_read();
        let mut buf = String::new();
        read.read_line_to_string(&mut buf)
            .or_raise(|| message("Failed to read or write to the process"))?;
        if buf
            .strip_prefix(welcome_prefix)
            .is_none_or(|rest| rest.trim_end() != "-server")
        {
            return Err(message!("Wanted '{welcome_prefix}-server, got  '{buf}'").raise());
        }

        buf.clear();
        read.read_line_to_string(&mut buf)
            .or_raise(|| message("Failed to read or write to the process"))?;
        let chosen_version = match buf
            .strip_prefix("version=")
            .and_then(|version| usize::from_str(version.trim_end()).ok())
        {
            Some(version) => version,
            None => {
                return Err(message!("Needed 'version=<integer>', got  '{buf}'").raise());
            }
        };

        if !versions.contains(&chosen_version) {
            return Err(message!(
                "Server offered {chosen_version}, we only support  '{}'",
                versions.iter().map(ToString::to_string).collect::<Vec<_>>().join(", ")
            )
            .raise());
        }

        if read
            .read_line_to_string(&mut buf)
            .or_raise(|| message("Failed to read or write to the process"))?
            != 0
        {
            return Err(message!("expected flush packet, got '{buf}'").raise());
        }
        for capability in desired_capabilities {
            out.write_all(format!("capability={capability}").as_bytes())
                .or_raise(|| message("Failed to read or write to the process"))?;
        }
        encode::flush_to_write(out.inner_mut()).or_raise(|| message("Failed to read or write to the process"))?;
        out.flush()
            .or_raise(|| message("Failed to read or write to the process"))?;

        read.reset_with(&[gix_packetline::PacketLineRef::Flush]);
        let mut capabilities = HashSet::new();
        loop {
            buf.clear();
            let num_read = read
                .read_line_to_string(&mut buf)
                .or_raise(|| message("Failed to read or write to the process"))?;
            if num_read == 0 {
                break;
            }
            match buf.strip_prefix("capability=") {
                Some(cap) => {
                    let cap = cap.trim_end();
                    if !desired_capabilities.contains(&cap) {
                        return Err(message!(
                            "The server sent the '{cap}' capability which isn't among the ones we desire can support"
                        )
                        .raise());
                    }
                    capabilities.insert(cap.to_owned());
                }
                None => continue,
            }
        }

        drop(read);
        Ok(Client {
            child: process,
            out: input,
            input: out,
            capabilities,
            version: chosen_version,
        })
    }

    /// Invoke `command` and send all `meta` data before sending all `content` in full.
    pub fn invoke(
        &mut self,
        command: &str,
        meta: &mut dyn Iterator<Item = (&str, BString)>,
        content: &mut dyn std::io::Read,
    ) -> Result<process::Status, invoke::Error> {
        use gix_error::{ResultExt, message};

        self.send_command_and_meta(command, meta)?;
        std::io::copy(content, &mut self.input).or_raise(|| message("Failed to read or write to the process"))?;
        encode::flush_to_write(self.input.inner_mut())
            .or_raise(|| message("Failed to read or write to the process"))?;
        self.input
            .flush()
            .or_raise(|| message("Failed to read or write to the process"))?;
        self.read_status()
            .or_raise(|| message("Failed to read or write to the process"))
    }

    /// Invoke `command` while passing `meta` data, but don't send any content, and return their status.
    /// Call `inspect_line` for each line that we see as command response.
    ///
    /// This is for commands that don't expect a content stream.
    pub fn invoke_without_content<'a>(
        &mut self,
        command: &str,
        meta: &mut dyn Iterator<Item = (&'a str, BString)>,
        inspect_line: &mut dyn FnMut(&BStr),
    ) -> Result<process::Status, invoke::without_content::Error> {
        use gix_error::{ResultExt, message};

        self.send_command_and_meta(command, meta)?;
        while let Some(data) = self.out.read_line() {
            let line = data
                .or_raise(|| message("Failed to read from the process"))?
                .or_raise(|| message("Failed to decode a packet line from the process"))?;
            if let Some(line) = line.as_text() {
                inspect_line(line.as_bstr());
            }
        }
        self.out.reset_with(&[gix_packetline::PacketLineRef::Flush]);
        let status = self
            .read_status()
            .or_raise(|| message("Failed to read or write to the process"))?;
        Ok(status)
    }

    /// Return a `Read` implementation that reads the server process output until the next flush package, and validates
    /// the status. If the status indicates failure, the last read will also fail.
    pub fn as_read(&mut self) -> impl std::io::Read + '_ {
        self.out.reset_with(&[gix_packetline::PacketLineRef::Flush]);
        ReadProcessOutputAndStatus {
            inner: self.out.as_read(),
        }
    }

    /// Read a `status=` line from the process output until it is exhausted.
    /// Note that the last sent status line wins and no status line means that the `Previous` still counts.
    pub fn read_status(&mut self) -> std::io::Result<process::Status> {
        read_status(&mut self.out.as_read())
    }
}

impl Client {
    fn send_command_and_meta(
        &mut self,
        command: &str,
        meta: &mut dyn Iterator<Item = (&str, BString)>,
    ) -> Result<(), invoke::Error> {
        use gix_error::{ResultExt, message};

        self.input
            .write_all(format!("command={command}").as_bytes())
            .or_raise(|| message("Failed to read or write to the process"))?;
        let mut buf = BString::default();
        for (key, value) in meta {
            buf.clear();
            buf.push_str(key);
            buf.push(b'=');
            buf.push_str(&value);
            self.input
                .write_all(&buf)
                .or_raise(|| message("Failed to read or write to the process"))?;
        }
        encode::flush_to_write(self.input.inner_mut())
            .or_raise(|| message("Failed to read or write to the process"))?;
        Ok(())
    }
}

fn read_status(read: &mut PacketlineReader<'_>) -> std::io::Result<process::Status> {
    let mut status = process::Status::Previous;
    let mut buf = String::new();
    let mut count = 0;
    loop {
        buf.clear();
        let num_read = read.read_line_to_string(&mut buf)?;
        if num_read == 0 {
            break;
        }
        if let Some(name) = buf.strip_prefix("status=") {
            status = process::Status::Named(name.trim_end().into());
        }
        count += 1;
    }
    if count > 0 && matches!(status, process::Status::Previous) {
        status = process::Status::Unset;
    }
    read.reset_with(&[gix_packetline::PacketLineRef::Flush]);
    Ok(status)
}

struct ReadProcessOutputAndStatus<'a> {
    inner: PacketlineReader<'a>,
}

impl std::io::Read for ReadProcessOutputAndStatus<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let num_read = self.inner.read(buf)?;
        if num_read == 0 {
            self.inner.reset_with(&[gix_packetline::PacketLineRef::Flush]);
            let status = read_status(&mut self.inner)?;
            if status.is_success() {
                Ok(0)
            } else {
                Err(std::io::Error::other(format!(
                    "Process indicated error after reading: {}",
                    status.message().unwrap_or_default()
                )))
            }
        } else {
            Ok(num_read)
        }
    }
}

/// Access
impl Client {
    /// Return the list of capabilities reported by the serving process.
    pub fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    /// Return the mutable list of capabilities reported by the serving process.
    pub fn capabilities_mut(&mut self) -> &mut Capabilities {
        &mut self.capabilities
    }

    /// Return the negotiated version of the protocol.
    ///
    /// Note that it is the highest one that both the client and the server support.
    pub fn version(&self) -> usize {
        self.version
    }
}

/// Lifecycle
impl Client {
    /// Return the child handle of the running process.
    ///
    /// Note that this will naturally close input and output handles, which is a signal for the child process to shutdown.
    pub fn into_child(self) -> std::process::Child {
        self.child
    }
}

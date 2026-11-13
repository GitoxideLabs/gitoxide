use std::{
    io::{self, Read, Write},
    path::Path,
    process::Stdio,
    time::Duration,
};

use bstr::BStr;
use gix_error::{ErrorExt, Result, ResultExt, bail, message, unsupported};
use gix_packetline::{
    PacketLineRef,
    blocking_io::encode,
    decode::{self, PacketLineOrWantedSize},
};

/// Limits for native filesystem monitor connections and responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// Retries after startup, and retries while a Windows named pipe is busy. Defaults to 100.
    pub connect_retries: usize,
    /// Delay between connection retries. Defaults to 50 milliseconds.
    pub connect_retry_delay: Duration,
    /// Blocking Unix read/write timeout; `None` disables it. Defaults to 30 seconds.
    /// The async client uses this deadline for each connection attempt and the full exchange on all platforms.
    pub socket_timeout: Option<Duration>,
    /// Maximum combined response payload in bytes. Zero accepts only empty payloads. Defaults to 64 MiB.
    pub max_response_size: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            connect_retries: 100,
            connect_retry_delay: Duration::from_millis(50),
            socket_timeout: Some(Duration::from_secs(30)),
            max_response_size: 64 * 1024 * 1024,
        }
    }
}

/// Query a Git-compatible filesystem monitor at the absolute `socket_path` with an opaque `token`.
///
/// If the endpoint is missing or refuses connections, invoke `start` once. Returning `Ok(Ok(()))`
/// indicates custom startup; `Ok(Err(context))` requests Git's default startup on macOS and Windows,
/// which is unsupported elsewhere. The context must provide absolute repository and worktree paths.
/// An outer callback error is returned without falling back to Git.
/// Custom startup must keep the server alive through the query.
/// Windows resolves the endpoint's parent directory before deriving Git's named-pipe name.
/// `options` bound connection retries and response payloads. Unix exchanges support timeouts;
/// Windows response reads are synchronous and may block.
pub fn query(
    socket_path: &Path,
    token: &BStr,
    start: impl FnOnce() -> Result<std::result::Result<(), gix_command::Context>>,
    options: Options,
) -> Result<Vec<u8>> {
    let _span = gix_trace::coarse!("gix_fsmonitor::client::daemon::query", protocol = 2);
    let mut stream = match connect(socket_path, &options) {
        Ok(stream) => stream,
        Err(err) if matches!(err.kind(), io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused) => {
            gix_trace::debug!(error_kind = ?err.kind(), "filesystem monitor daemon is not listening");
            gix_trace::debug!("invoking filesystem monitor daemon startup callback");
            if let Err(context) = start()
                .inspect_err(|_err| {
                    gix_trace::debug!(error = %_err, "filesystem monitor daemon startup callback failed");
                })
                .or_raise(|| message("Could not start the filesystem monitor"))?
            {
                if !cfg!(any(target_os = "macos", windows)) {
                    gix_trace::debug!("filesystem monitor daemon cannot be started on this platform");
                    return Err(err.and_raise(unsupported(
                        "Git's filesystem monitor daemon is unavailable on this platform",
                    )));
                }

                let Some(workdir) = context.worktree_dir.clone() else {
                    bail!(message("Cannot start a filesystem monitor without a worktree"));
                };
                let mut command = std::process::Command::try_from(
                    gix_command::prepare("git")
                        .args(["fsmonitor--daemon", "start"])
                        .with_context(context)
                        .stdout(Stdio::null()),
                )?;
                gix_trace::debug!("starting filesystem monitor daemon");
                let status = command
                    .current_dir(workdir)
                    .status()
                    .inspect_err(|_err| {
                        gix_trace::debug!(error_kind = ?_err.kind(), "filesystem monitor daemon could not be launched");
                    })
                    .or_raise(|| message("Could not start Git's filesystem monitor"))?;
                if !status.success() {
                    gix_trace::debug!(exit_code = ?status.code(), "filesystem monitor daemon startup failed");
                    bail!(message("Git's filesystem monitor could not be started"));
                }
            }
            let mut result = connect(socket_path, &options);
            for _ in 0..options.connect_retries {
                if !result
                    .as_ref()
                    .is_err_and(|err| matches!(err.kind(), io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused))
                {
                    break;
                }
                std::thread::sleep(options.connect_retry_delay);
                result = connect(socket_path, &options);
            }
            result
                .inspect_err(|_err| {
                    gix_trace::debug!(error_kind = ?_err.kind(), "filesystem monitor daemon did not become available after startup");
                })
                .or_raise(|| message("Could not connect to the filesystem monitor after starting it"))?
        }
        Err(err) => {
            gix_trace::debug!(error_kind = ?err.kind(), "filesystem monitor daemon connection failed");
            return Err(err).or_raise(|| message("Could not connect to the filesystem monitor"));
        }
    };
    let response = exchange(&mut stream, token, options.max_response_size)
        .inspect_err(|_err| {
            gix_trace::debug!(error_kind = ?_err.kind(), error = %_err, "filesystem monitor daemon exchange failed");
        })
        .or_raise(|| message("Could not query the filesystem monitor"))?;
    gix_trace::debug!(
        response_bytes = response.len(),
        "filesystem monitor daemon query finished"
    );
    Ok(response)
}

/// Query a native daemon using Tokio I/O and timers, with an asynchronous startup callback.
///
/// Like [`query()`], missing endpoints invoke `start` once; returning a command context requests
/// Git startup on macOS or Windows. `socket_timeout` bounds the entire exchange on all platforms,
/// including Windows. Dropping the future closes its connection; a started daemon is not stopped.
/// Requires a Tokio runtime with I/O, timers, and process support enabled.
#[cfg(feature = "async")]
pub async fn query_async(
    socket_path: &Path,
    token: &BStr,
    start: impl AsyncFnOnce() -> Result<std::result::Result<(), gix_command::Context>>,
    options: Options,
) -> Result<Vec<u8>> {
    async fn connect(path: &Path, options: &Options) -> io::Result<crate::asynchronous::Stream> {
        for attempt in 0..=options.connect_retries {
            let result = match options.socket_timeout {
                Some(timeout) => tokio::time::timeout(timeout, crate::asynchronous::connect(path))
                    .await
                    .unwrap_or_else(|_| {
                        Err(io::Error::new(
                            io::ErrorKind::TimedOut,
                            "filesystem monitor connection timed out",
                        ))
                    }),
                None => crate::asynchronous::connect(path).await,
            };
            if cfg!(windows)
                && result.as_ref().is_err_and(|err| err.raw_os_error() == Some(231))
                && attempt < options.connect_retries
            {
                tokio::time::sleep(options.connect_retry_delay).await;
            } else {
                return result;
            }
        }
        unreachable!("the last connection attempt returns")
    }
    let mut stream = match connect(socket_path, &options).await {
        Ok(stream) => stream,
        Err(error) if matches!(error.kind(), io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused) => {
            if let Err(context) = start()
                .await
                .or_raise(|| message("Could not start the filesystem monitor"))?
            {
                if !cfg!(any(target_os = "macos", windows)) {
                    return Err(error.and_raise(unsupported(
                        "Git's filesystem monitor daemon is unavailable on this platform",
                    )));
                }
                let Some(workdir) = context.worktree_dir.clone() else {
                    bail!(message("Cannot start a filesystem monitor without a worktree"));
                };
                let command = std::process::Command::try_from(
                    gix_command::prepare("git")
                        .args(["fsmonitor--daemon", "start"])
                        .with_context(context)
                        .stdout(Stdio::null()),
                )?;
                let status = tokio::process::Command::from(command)
                    .current_dir(workdir)
                    .status()
                    .await
                    .or_raise(|| message("Could not start Git's filesystem monitor"))?;
                if !status.success() {
                    bail!(message("Git's filesystem monitor could not be started"));
                }
            }
            let mut result = connect(socket_path, &options).await;
            for _ in 0..options.connect_retries {
                if !result
                    .as_ref()
                    .is_err_and(|err| matches!(err.kind(), io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused))
                {
                    break;
                }
                tokio::time::sleep(options.connect_retry_delay).await;
                result = connect(socket_path, &options).await;
            }
            result.or_raise(|| message("Could not connect to the filesystem monitor after starting it"))?
        }
        Err(error) => return Err(error).or_raise(|| message("Could not connect to the filesystem monitor")),
    };
    crate::asynchronous::exchange(&mut stream, token, options.max_response_size, options.socket_timeout)
        .await
        .or_raise(|| message("Could not query the filesystem monitor"))
}

#[cfg(unix)]
fn connect(path: &Path, options: &Options) -> io::Result<std::os::unix::net::UnixStream> {
    use std::os::unix::{fs::PermissionsExt, net::UnixStream};

    let stream = match UnixStream::connect(path) {
        Ok(stream) => stream,
        Err(err) if err.kind() == io::ErrorKind::InvalidInput => {
            gix_trace::debug!("using a short filesystem monitor socket alias");
            // Shorten overlong socket addresses without changing the process-wide working directory.
            let directory = tempfile::Builder::new()
                .prefix("gix-fsm-")
                .permissions(std::fs::Permissions::from_mode(0o700))
                .tempdir_in("/tmp")?;
            let alias = directory.path().join("socket");
            std::os::unix::fs::symlink(path, &alias)?;
            UnixStream::connect(alias)?
        }
        Err(err) => return Err(err),
    };
    stream.set_read_timeout(options.socket_timeout)?;
    stream.set_write_timeout(options.socket_timeout)?;
    Ok(stream)
}

#[cfg(windows)]
fn connect(path: &Path, options: &Options) -> io::Result<std::fs::File> {
    // shortcut: synchronous reads match Git; add cancellable I/O if stalled-daemon deadlines are required.
    let pipe = pipe_name(&crate::normalize_endpoint(path)?)?;
    for attempt in 0..=options.connect_retries {
        match std::fs::OpenOptions::new().read(true).write(true).open(&pipe) {
            Err(err) if err.raw_os_error() == Some(231) && attempt < options.connect_retries => {
                std::thread::sleep(options.connect_retry_delay);
            }
            result => return result,
        }
    }
    unreachable!("the final connection attempt returns its result")
}

#[cfg(windows)]
pub(crate) fn pipe_name(path: &Path) -> io::Result<std::ffi::OsString> {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};

    let path: Vec<_> = path.as_os_str().encode_wide().collect();
    if path.contains(&0) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "NUL in pipe name"));
    }
    let verbatim: Vec<_> = "\\\\?\\".encode_utf16().collect();
    let mut path = path.strip_prefix(verbatim.as_slice()).unwrap_or(&path).to_vec();
    let unc: Vec<_> = "UNC\\".encode_utf16().collect();
    if path.starts_with(&unc) {
        path.splice(..unc.len(), "\\\\".encode_utf16());
    }
    let mut pipe: Vec<_> = "\\\\.\\pipe\\".encode_utf16().collect();
    pipe.extend(path.iter().enumerate().map(|(idx, &ch)| match ch {
        ch if ch == u16::from(b':') && idx == 1 => u16::from(b'_'),
        ch if ch == u16::from(b'/') => u16::from(b'\\'),
        ch => ch,
    }));
    Ok(std::ffi::OsString::from_wide(&pipe))
}

#[cfg(not(any(unix, windows)))]
fn connect(_path: &Path, _options: &Options) -> io::Result<io::Cursor<Vec<u8>>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Filesystem monitor IPC is unavailable",
    ))
}

const MAX_PACKET_SIZE: usize = 65520;

fn exchange(stream: &mut (impl Read + Write), token: &[u8], max_response_size: usize) -> io::Result<Vec<u8>> {
    for chunk in token.chunks(MAX_PACKET_SIZE - 4) {
        encode::data_to_write(chunk, &mut *stream)?;
    }
    encode::flush_to_write(&mut *stream)?;
    stream.flush()?;

    let mut response = Vec::new();
    loop {
        let mut header = [0; 4];
        stream.read_exact(&mut header)?;
        // Decode only the prefix so limits are enforced before reading or allocating payload data.
        match decode::hex_prefix(&header).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))? {
            PacketLineOrWantedSize::Line(PacketLineRef::Flush) => return Ok(response),
            PacketLineOrWantedSize::Line(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Unsupported filesystem monitor packet delimiter",
                ));
            }
            PacketLineOrWantedSize::Wanted(len) => {
                let len = usize::from(len);
                if len > MAX_PACKET_SIZE - 4 || len > max_response_size - response.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Filesystem monitor response is too large",
                    ));
                }
                let offset = response.len();
                response.resize(offset + len, 0);
                stream.read_exact(&mut response[offset..])?;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{self, Cursor, Read, Write};

    use super::*;

    struct Connection {
        input: Cursor<Vec<u8>>,
        written: Vec<u8>,
    }

    impl Connection {
        fn new(input: &[u8]) -> Self {
            Self {
                input: Cursor::new(input.to_owned()),
                written: Vec::new(),
            }
        }
    }

    impl Read for Connection {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let len = buf.len().min(1);
            self.input.read(&mut buf[..len])
        }
    }

    impl Write for Connection {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.written.write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    #[cfg(unix)]
    fn configured_socket_timeouts_are_applied() -> gix_testtools::TestResult {
        let directory = gix_testtools::tempfile::tempdir_in("/tmp")?;
        let path = directory.path().join("socket");
        let _listener = std::os::unix::net::UnixListener::bind(&path)?;
        for timeout in [None, Some(Duration::from_secs(1))] {
            let options = Options {
                connect_retries: 0,
                connect_retry_delay: Duration::ZERO,
                socket_timeout: timeout,
                max_response_size: 0,
            };
            let stream = connect(&path, &options)?;
            assert_eq!(stream.read_timeout()?, timeout, "the configured timeout controls reads");
            assert_eq!(
                stream.write_timeout()?,
                timeout,
                "the configured timeout controls writes"
            );
        }
        Ok(())
    }

    #[test]
    fn response_limits_count_payload_across_packets() -> gix_testtools::TestResult {
        let input = b"0009token0008\x00a\x00b0000";
        assert_eq!(
            exchange(&mut Connection::new(input), b"old-token", 9)?,
            b"token\x00a\x00b"
        );
        for limit in [0, 8] {
            assert_eq!(
                exchange(&mut Connection::new(input), b"old-token", limit)
                    .expect_err("the entire payload must fit the response limit")
                    .kind(),
                io::ErrorKind::InvalidData
            );
        }
        assert!(exchange(&mut Connection::new(b"0000"), b"", 0)?.is_empty());
        assert_eq!(
            exchange(&mut Connection::new(b"000d"), b"old-token", 8)
                .expect_err("response limits must be checked before reading the payload")
                .kind(),
            io::ErrorKind::InvalidData,
            "an oversized payload is rejected even when its data has not arrived"
        );
        Ok(())
    }

    #[test]
    fn fragmented_packets_preserve_binary_response() -> gix_testtools::TestResult {
        let mut connection = Connection::new(b"0009token0008\x00a\x00b0000");
        assert_eq!(
            exchange(&mut connection, b"old-token", Options::default().max_response_size)?,
            b"token\x00a\x00b"
        );
        assert_eq!(
            connection.written, b"000dold-token0000",
            "the request has a flush packet, without appending NUL to the token"
        );
        assert_eq!(
            exchange(
                &mut Connection::new(b"0010ERR \x00path\xff\n\x000000"),
                b"old-token",
                Options::default().max_response_size
            )?,
            b"ERR \x00path\xff\n\x00",
            "Simple IPC payloads are raw bytes, not error lines, UTF-8, or newline-terminated text"
        );
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn unix_stream_exchanges_a_single_query() -> gix_testtools::TestResult {
        let (mut client, mut server) = std::os::unix::net::UnixStream::pair()?;
        let thread = std::thread::spawn(move || -> io::Result<()> {
            let mut request = [0; 17];
            server.read_exact(&mut request)?;
            assert_eq!(&request, b"000dold-token0000");
            for byte in b"000atoken\x000006/\x000000" {
                server.write_all(&[*byte])?;
            }
            Ok(())
        });
        assert_eq!(
            exchange(&mut client, b"old-token", Options::default().max_response_size)?,
            b"token\x00/\x00"
        );
        thread.join().expect("the mock server must finish without panicking")?;
        Ok(())
    }

    #[test]
    #[cfg(windows)]
    fn windows_pipe_names_follow_git() -> gix_testtools::TestResult {
        for path in [
            r"C:\work\.git\fsmonitor--daemon.ipc",
            r"C:/work/.git/fsmonitor--daemon.ipc",
            r"\\?\C:\work\.git\fsmonitor--daemon.ipc",
        ] {
            assert_eq!(
                pipe_name(Path::new(path))?,
                std::ffi::OsStr::new(r"\\.\pipe\C_\work\.git\fsmonitor--daemon.ipc")
            );
        }
        assert_eq!(
            pipe_name(Path::new(r"\\?\UNC\server\share\fsmonitor--daemon.ipc"))?,
            pipe_name(Path::new(r"\\server\share\fsmonitor--daemon.ipc"))?,
            "verbatim paths name the same pipe as regular UNC paths"
        );
        assert_eq!(
            pipe_name(Path::new("C:/pipe\0suffix"))
                .expect_err("native pipe names cannot contain NUL")
                .kind(),
            io::ErrorKind::InvalidInput
        );
        Ok(())
    }

    #[test]
    fn requests_and_responses_obey_packet_size_boundaries() -> gix_testtools::TestResult {
        let mut connection = Connection::new(b"0000");
        assert!(exchange(&mut connection, b"", Options::default().max_response_size)?.is_empty());
        assert_eq!(connection.written, b"0000", "empty requests contain only a flush");

        let token = vec![b'x'; MAX_PACKET_SIZE - 3];
        let mut response = b"fff0".to_vec();
        response.extend_from_slice(&token[..MAX_PACKET_SIZE - 4]);
        response.extend_from_slice(b"000500000");
        let mut connection = Connection::new(&response);
        let output = exchange(&mut connection, &token, Options::default().max_response_size)?;
        assert_eq!(&output[..MAX_PACKET_SIZE - 4], &token[..MAX_PACKET_SIZE - 4]);
        assert_eq!(output[MAX_PACKET_SIZE - 4], b'0');
        let mut expected = b"fff0".to_vec();
        expected.extend_from_slice(&token[..MAX_PACKET_SIZE - 4]);
        expected.extend_from_slice(b"0005x0000");
        assert_eq!(connection.written, expected, "oversized tokens span multiple packets");
        Ok(())
    }

    #[test]
    fn malformed_control_and_truncated_packets_fail() {
        for input in [b"0001", b"0002", b"0003", b"0004", b"fff1", b"ffff", b"zzzz", b"+005"] {
            assert_eq!(
                exchange(
                    &mut Connection::new(input),
                    b"token",
                    Options::default().max_response_size
                )
                .expect_err("invalid packet lengths and controls must fail")
                .kind(),
                io::ErrorKind::InvalidData,
                "invalid framing must cause the caller to use a full status scan"
            );
        }
        for input in [b"".as_slice(), b"00", b"0006x", b"0005x"] {
            assert_eq!(
                exchange(
                    &mut Connection::new(input),
                    b"token",
                    Options::default().max_response_size
                )
                .expect_err("a complete response requires all data and the final flush")
                .kind(),
                io::ErrorKind::UnexpectedEof
            );
        }
    }
}

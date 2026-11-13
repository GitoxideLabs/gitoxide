use std::{io, path::Path, time::Duration};

use gix_macros::keep as maybe_async;
use gix_packetline::{
    PacketLineRef,
    decode::{self, PacketLineOrWantedSize},
};
use tokio::io::{AsyncRead as Read, AsyncReadExt, AsyncWrite as Write, AsyncWriteExt};

const MAX_PACKET_DATA: usize = 65_516;

async fn write_packet(output: &mut (impl Write + Unpin), data: Option<&[u8]>) -> io::Result<()> {
    match data {
        Some(data) => {
            let mut packet = Vec::with_capacity(data.len() + 4);
            gix_packetline::blocking_io::encode::data_to_write(data, &mut packet)?;
            output.write_all(&packet).await
        }
        None => output.write_all(b"0000").await,
    }
}

include!("message.rs");

#[cfg(unix)]
pub(crate) type Stream = tokio::net::UnixStream;
#[cfg(windows)]
pub(crate) type Stream = tokio::net::windows::named_pipe::NamedPipeClient;

#[cfg(unix)]
struct AliasDirectory {
    directory: Option<tempfile::TempDir>,
    runtime: tokio::runtime::Handle,
}

#[cfg(unix)]
impl Drop for AliasDirectory {
    fn drop(&mut self) {
        let directory = self.directory.take();
        self.runtime.spawn_blocking(move || drop(directory));
    }
}

#[cfg(unix)]
pub(crate) async fn connect(path: &Path) -> io::Result<Stream> {
    match Stream::connect(path).await {
        Err(error) if error.kind() == io::ErrorKind::InvalidInput => {
            let path = path.to_owned();
            let runtime = tokio::runtime::Handle::current();
            let (directory, alias) = tokio::task::spawn_blocking(move || -> io::Result<_> {
                use std::os::unix::fs::PermissionsExt;
                let directory = tempfile::Builder::new()
                    .prefix("gix-fsm-")
                    .permissions(std::fs::Permissions::from_mode(0o700))
                    .tempdir_in("/tmp")?;
                let alias = directory.path().join("socket");
                std::os::unix::fs::symlink(path, &alias)?;
                Ok((
                    AliasDirectory {
                        directory: Some(directory),
                        runtime,
                    },
                    alias,
                ))
            })
            .await
            .map_err(io::Error::other)??;
            let result = Stream::connect(alias).await;
            drop(directory);
            result
        }
        result => result,
    }
}

#[cfg(windows)]
pub(crate) async fn connect(path: &Path) -> io::Result<Stream> {
    let path = path.to_owned();
    let path = tokio::task::spawn_blocking(move || crate::normalize_endpoint(&path))
        .await
        .map_err(io::Error::other)??;
    tokio::net::windows::named_pipe::ClientOptions::new().open(crate::client::daemon::pipe_name(&path)?)
}

#[cfg(not(any(unix, windows)))]
pub(crate) type Stream = tokio::io::DuplexStream;
#[cfg(not(any(unix, windows)))]
pub(crate) async fn connect(_path: &Path) -> io::Result<Stream> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "filesystem monitor IPC is unavailable",
    ))
}

pub(crate) async fn exchange(
    stream: &mut Stream,
    token: &[u8],
    limit: usize,
    timeout: Option<Duration>,
) -> io::Result<Vec<u8>> {
    let exchange = async {
        write_message(stream, token).await?;
        read_message(stream, limit).await?.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "filesystem monitor closed without a response",
            )
        })
    };
    match timeout {
        Some(timeout) => tokio::time::timeout(timeout, exchange)
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "filesystem monitor exchange timed out"))?,
        None => exchange.await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "current_thread")]
    async fn fragmented_messages_limits_and_cancelled_reads() -> gix_error::TestResult {
        let (mut reader, mut writer) = tokio::io::duplex(8);
        let write = tokio::spawn(async move {
            for byte in b"0009token0008\0a\0b0000" {
                writer.write_all(&[*byte]).await?;
            }
            Ok::<_, io::Error>(())
        });
        assert_eq!(
            read_message(&mut reader, 9).await?,
            Some(b"token\0a\0b".to_vec()),
            "fragmented packets preserve binary payloads"
        );
        write.await??;
        for mut input in [b"000d".as_slice(), b"0001", b"zzzz", b"0005x"] {
            assert!(
                read_message(&mut input, 8).await.is_err(),
                "malformed, oversized or truncated messages fail before trusting a response"
            );
        }
        let (mut reader, _writer) = tokio::io::duplex(8);
        assert!(
            tokio::time::timeout(Duration::from_millis(10), read_message(&mut reader, 8))
                .await
                .is_err(),
            "a stalled read yields to the runtime timer"
        );
        Ok(())
    }
}

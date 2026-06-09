//! Async transport integration for blocking `receive-pack` server plumbing.
//!
//! This module bridges async byte streams into [`super::serve_v1()`] and
//! [`super::serve_v2()`] for one client/session at a time.

use futures_io::{AsyncRead, AsyncWrite};
use futures_lite::AsyncWriteExt as _;

/// Serve one protocol V1 receive-pack request over async transport streams.
///
/// This adapts async readers/writers to the existing blocking receive-pack plumbing.
pub async fn serve_v1<R, W, D>(input: &mut R, output: &mut W, delegate: &mut D) -> Result<super::Outcome, super::Error>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
    D: super::Delegate,
{
    let outcome = {
        let mut blocking_input = futures_lite::io::BlockOn::new(input);
        let mut blocking_output = futures_lite::io::BlockOn::new(&mut *output);
        super::serve_v1(&mut blocking_input, &mut blocking_output, delegate)?
    };
    output.flush().await?;
    Ok(outcome)
}

/// Serve one protocol V2 receive-pack request over async transport streams.
///
/// This adapts async readers/writers to the existing blocking receive-pack plumbing.
pub async fn serve_v2<R, W, D>(input: &mut R, output: &mut W, delegate: &mut D) -> Result<super::Outcome, super::Error>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
    D: super::Delegate,
{
    let outcome = {
        let mut blocking_input = futures_lite::io::BlockOn::new(input);
        let mut blocking_output = futures_lite::io::BlockOn::new(&mut *output);
        super::serve_v2(&mut blocking_input, &mut blocking_output, delegate)?
    };
    output.flush().await?;
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        future::Future,
        io::Write as _,
        sync::atomic::{AtomicBool, Ordering},
    };

    use bstr::{BString, ByteSlice};
    use futures_lite::io::Cursor;
    use gix_transport::packetline::{
        PacketLineRef,
        blocking_io::{StreamingPeekableIter, Writer, encode},
    };

    use super::*;

    #[derive(Debug, Default, Clone, PartialEq, Eq)]
    struct ListenerOutcome {
        sessions_served: usize,
        updates_received: usize,
        push_options_received: usize,
        ref_statuses_sent: usize,
    }

    #[derive(Default)]
    struct RecordingDelegate {
        response: super::super::Response,
        seen_requests: Vec<super::super::Request>,
        seen_pack_prefixes: Vec<[u8; 4]>,
    }

    impl super::super::Delegate for RecordingDelegate {
        fn receive(
            &mut self,
            request: &super::super::Request,
            pack_data: &mut dyn std::io::Read,
        ) -> Result<super::super::Response, Box<dyn std::error::Error + Send + Sync + 'static>> {
            self.seen_requests.push(request.clone());
            let mut prefix = [0u8; 4];
            pack_data.read_exact(&mut prefix)?;
            self.seen_pack_prefixes.push(prefix);
            Ok(self.response.clone())
        }
    }

    #[async_std::test]
    async fn serve_v1_bridges_async_transport_io() -> Result<(), Box<dyn std::error::Error>> {
        let request = request_bytes(
            &["0000000000000000000000000000000000000000 808e50d724f604f69ab93c6da2919c014667bedb refs/heads/main"],
            &["report-status"],
            &[],
            b"PACK\0\0\0\x02",
        )?;
        let mut input = Cursor::new(request);
        let mut output = Cursor::new(Vec::<u8>::new());
        let mut delegate = RecordingDelegate {
            response: super::super::Response {
                unpack_status: super::super::UnpackStatus::Ok,
                ref_statuses: vec![super::super::RefStatus::Ok {
                    ref_name: "refs/heads/main".into(),
                }],
                sideband_messages: Vec::new(),
            },
            ..Default::default()
        };

        let outcome = serve_v1(&mut input, &mut output, &mut delegate).await?;
        assert_eq!(outcome.updates_received, 1);
        assert_eq!(outcome.push_options_received, 0);
        assert_eq!(outcome.ref_statuses_sent, 1);
        assert_eq!(delegate.seen_pack_prefixes, vec![*b"PACK"]);

        let mut reader = StreamingPeekableIter::new(output.get_ref().as_slice(), &[PacketLineRef::Flush], false);
        assert_eq!(next_text_line(&mut reader)?.as_bstr(), "unpack ok".as_bytes().as_bstr());
        assert_eq!(
            next_text_line(&mut reader)?.as_bstr(),
            "ok refs/heads/main".as_bytes().as_bstr()
        );
        assert!(reader.read_line().is_none());
        assert_eq!(reader.stopped_at(), Some(PacketLineRef::Flush));
        Ok(())
    }

    #[async_std::test]
    async fn serve_v2_bridges_async_transport_io() -> Result<(), Box<dyn std::error::Error>> {
        let request = request_bytes_v2(
            &["report-status-v2", "push-options"],
            &["0000000000000000000000000000000000000000 808e50d724f604f69ab93c6da2919c014667bedb refs/heads/main"],
            &["trace=1"],
            b"PACK\0\0\0\x02",
        )?;
        let mut input = Cursor::new(request);
        let mut output = Cursor::new(Vec::<u8>::new());
        let mut delegate = RecordingDelegate {
            response: super::super::Response {
                unpack_status: super::super::UnpackStatus::Ok,
                ref_statuses: vec![super::super::RefStatus::Ok {
                    ref_name: "refs/heads/main".into(),
                }],
                sideband_messages: Vec::new(),
            },
            ..Default::default()
        };

        let outcome = serve_v2(&mut input, &mut output, &mut delegate).await?;
        assert_eq!(outcome.updates_received, 1);
        assert_eq!(outcome.push_options_received, 1);
        assert_eq!(outcome.ref_statuses_sent, 1);
        assert_eq!(delegate.seen_pack_prefixes, vec![*b"PACK"]);
        assert!(
            delegate.seen_requests[0].has_capability("report-status-v2"),
            "V2 features should be lowered to capabilities"
        );
        assert_eq!(delegate.seen_requests[0].push_options, vec![BString::from("trace=1")]);

        let mut reader = StreamingPeekableIter::new(output.get_ref().as_slice(), &[PacketLineRef::Flush], false);
        assert_eq!(
            next_text_line(&mut reader)?.as_bstr(),
            "report-status".as_bytes().as_bstr()
        );
        assert_eq!(next_text_line(&mut reader)?.as_bstr(), "unpack ok".as_bytes().as_bstr());
        assert_eq!(
            next_text_line(&mut reader)?.as_bstr(),
            "ok refs/heads/main".as_bytes().as_bstr()
        );
        assert!(reader.read_line().is_none());
        assert_eq!(reader.stopped_at(), Some(PacketLineRef::Flush));
        Ok(())
    }

    #[async_std::test]
    async fn listen_v1_serves_until_connection_source_is_exhausted() -> Result<(), Box<dyn std::error::Error>> {
        let request_one = request_bytes(
            &["0000000000000000000000000000000000000000 808e50d724f604f69ab93c6da2919c014667bedb refs/heads/main"],
            &["report-status"],
            &[],
            b"PACK\0\0\0\x02",
        )?;
        let request_two = request_bytes(
            &["808e50d724f604f69ab93c6da2919c014667bedb 0000000000000000000000000000000000000000 refs/heads/main"],
            &["report-status", "push-options"],
            &["trace=1"],
            b"PACK\0\0\0\x02",
        )?;

        let mut incoming = VecDeque::from(vec![
            Ok((Cursor::new(request_one), Cursor::new(Vec::<u8>::new()))),
            Ok((Cursor::new(request_two), Cursor::new(Vec::<u8>::new()))),
        ]);
        let mut delegate = RecordingDelegate {
            response: super::super::Response {
                unpack_status: super::super::UnpackStatus::Ok,
                ref_statuses: vec![super::super::RefStatus::Ok {
                    ref_name: "refs/heads/main".into(),
                }],
                sideband_messages: Vec::new(),
            },
            ..Default::default()
        };
        let should_stop = AtomicBool::new(false);

        let outcome = listen_v1_for_test(
            || {
                std::future::ready(match incoming.pop_front() {
                    Some(connection) => connection.map(Some),
                    None => Ok(None),
                })
            },
            &mut delegate,
            &should_stop,
        )
        .await?;

        assert_eq!(
            outcome,
            ListenerOutcome {
                sessions_served: 2,
                updates_received: 2,
                push_options_received: 1,
                ref_statuses_sent: 2,
            }
        );
        assert_eq!(delegate.seen_requests.len(), 2);
        assert_eq!(delegate.seen_pack_prefixes, vec![*b"PACK", *b"PACK"]);
        Ok(())
    }

    #[async_std::test]
    async fn listen_v2_serves_until_connection_source_is_exhausted() -> Result<(), Box<dyn std::error::Error>> {
        let request_one = request_bytes_v2(
            &["report-status-v2"],
            &["0000000000000000000000000000000000000000 808e50d724f604f69ab93c6da2919c014667bedb refs/heads/main"],
            &[],
            b"PACK\0\0\0\x02",
        )?;
        let request_two = request_bytes_v2(
            &["report-status-v2", "push-options"],
            &["808e50d724f604f69ab93c6da2919c014667bedb 0000000000000000000000000000000000000000 refs/heads/main"],
            &["trace=1"],
            b"PACK\0\0\0\x02",
        )?;

        let mut incoming = VecDeque::from(vec![
            Ok((Cursor::new(request_one), Cursor::new(Vec::<u8>::new()))),
            Ok((Cursor::new(request_two), Cursor::new(Vec::<u8>::new()))),
        ]);
        let mut delegate = RecordingDelegate {
            response: super::super::Response {
                unpack_status: super::super::UnpackStatus::Ok,
                ref_statuses: vec![super::super::RefStatus::Ok {
                    ref_name: "refs/heads/main".into(),
                }],
                sideband_messages: Vec::new(),
            },
            ..Default::default()
        };
        let should_stop = AtomicBool::new(false);

        let outcome = listen_v2_for_test(
            || {
                std::future::ready(match incoming.pop_front() {
                    Some(connection) => connection.map(Some),
                    None => Ok(None),
                })
            },
            &mut delegate,
            &should_stop,
        )
        .await?;

        assert_eq!(
            outcome,
            ListenerOutcome {
                sessions_served: 2,
                updates_received: 2,
                push_options_received: 1,
                ref_statuses_sent: 2,
            }
        );
        assert_eq!(delegate.seen_requests.len(), 2);
        assert_eq!(delegate.seen_pack_prefixes, vec![*b"PACK", *b"PACK"]);
        Ok(())
    }

    async fn listen_v1_for_test<R, W, D, Next, NextFuture>(
        mut next_connection: Next,
        delegate: &mut D,
        should_stop: &AtomicBool,
    ) -> Result<ListenerOutcome, Box<dyn std::error::Error>>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
        D: super::super::Delegate,
        Next: FnMut() -> NextFuture,
        NextFuture: Future<Output = std::io::Result<Option<(R, W)>>>,
    {
        let mut aggregated = ListenerOutcome::default();
        while !should_stop.load(Ordering::Relaxed) {
            let Some((mut input, mut output)) = next_connection().await? else {
                break;
            };
            let outcome = serve_v1(&mut input, &mut output, delegate).await?;
            aggregated.sessions_served += 1;
            aggregated.updates_received += outcome.updates_received;
            aggregated.push_options_received += outcome.push_options_received;
            aggregated.ref_statuses_sent += outcome.ref_statuses_sent;
        }
        Ok(aggregated)
    }

    async fn listen_v2_for_test<R, W, D, Next, NextFuture>(
        mut next_connection: Next,
        delegate: &mut D,
        should_stop: &AtomicBool,
    ) -> Result<ListenerOutcome, Box<dyn std::error::Error>>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
        D: super::super::Delegate,
        Next: FnMut() -> NextFuture,
        NextFuture: Future<Output = std::io::Result<Option<(R, W)>>>,
    {
        let mut aggregated = ListenerOutcome::default();
        while !should_stop.load(Ordering::Relaxed) {
            let Some((mut input, mut output)) = next_connection().await? else {
                break;
            };
            let outcome = serve_v2(&mut input, &mut output, delegate).await?;
            aggregated.sessions_served += 1;
            aggregated.updates_received += outcome.updates_received;
            aggregated.push_options_received += outcome.push_options_received;
            aggregated.ref_statuses_sent += outcome.ref_statuses_sent;
        }
        Ok(aggregated)
    }

    fn request_bytes(
        updates: &[&str],
        capabilities: &[&str],
        push_options: &[&str],
        pack_data: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        assert!(!updates.is_empty(), "at least one update command is required");
        let mut out = Vec::new();
        {
            let mut writer = Writer::new(&mut out);
            writer.enable_text_mode();
            let first = if capabilities.is_empty() {
                updates[0].to_owned()
            } else {
                format!("{}\0 {}", updates[0], capabilities.join(" "))
            };
            writer.write_all(first.as_bytes())?;
            for update in &updates[1..] {
                writer.write_all(update.as_bytes())?;
            }
            encode::flush_to_write(writer.inner_mut())?;

            if !push_options.is_empty() {
                for option in push_options {
                    writer.write_all(option.as_bytes())?;
                }
                encode::flush_to_write(writer.inner_mut())?;
            }
        }
        out.extend_from_slice(pack_data);
        Ok(out)
    }

    fn request_bytes_v2(
        features: &[&str],
        updates: &[&str],
        push_options: &[&str],
        pack_data: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        assert!(!updates.is_empty(), "at least one update command is required");
        let mut out = Vec::new();
        {
            let mut writer = Writer::new(&mut out);
            writer.enable_text_mode();
            writer.write_all(b"command=push")?;
            for feature in features {
                writer.write_all(feature.as_bytes())?;
            }
            encode::delim_to_write(writer.inner_mut())?;
            writer.write_all(b"section=ref-updates")?;
            for update in updates {
                writer.write_all(update.as_bytes())?;
            }
            if push_options.is_empty() {
                encode::flush_to_write(writer.inner_mut())?;
            } else {
                encode::delim_to_write(writer.inner_mut())?;
                writer.write_all(b"section=push-options")?;
                for option in push_options {
                    writer.write_all(option.as_bytes())?;
                }
                encode::flush_to_write(writer.inner_mut())?;
            }
        }
        out.extend_from_slice(pack_data);
        Ok(out)
    }

    fn next_text_line(reader: &mut StreamingPeekableIter<&[u8]>) -> Result<BString, Box<dyn std::error::Error>> {
        let line = reader
            .read_line()
            .expect("expected packetline")
            .expect("read should succeed")
            .expect("decode should succeed");
        Ok(line.as_text().expect("expected text packetline").as_bstr().to_owned())
    }
}

use std::{collections::VecDeque, io::Write as _};

use bstr::BString;
use gix_protocol::{
    futures_io::{AsyncRead, AsyncWrite},
    futures_lite,
    receive_pack::{self, RefStatus, Response, UnpackStatus},
    transport::packetline::{
        PacketLineRef,
        blocking_io::{StreamingPeekableIter, Writer, encode},
    },
};

#[derive(Default)]
struct DemoDelegate {
    sessions_processed: usize,
}

impl receive_pack::Delegate for DemoDelegate {
    fn receive(
        &mut self,
        request: &receive_pack::Request,
        pack_data: &mut dyn std::io::Read,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let mut header = [0u8; 4];
        pack_data.read_exact(&mut header)?;
        if header != *b"PACK" {
            return Err(std::io::Error::other(format!("expected pack header \"PACK\", got {header:?}")).into());
        }

        let first_update = request
            .updates
            .first()
            .ok_or_else(|| std::io::Error::other("expected at least one reference update"))?;

        self.sessions_processed += 1;

        Ok(Response {
            unpack_status: UnpackStatus::Ok,
            ref_statuses: vec![RefStatus::Ok {
                ref_name: first_update.ref_name.clone(),
            }],
            sideband_messages: Vec::new(),
        })
    }
}

async fn handle_client_session<R, W>(
    delegate: &mut impl receive_pack::Delegate,
    input: &mut R,
    output: &mut W,
) -> Result<receive_pack::Outcome, receive_pack::Error>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    receive_pack::async_io::serve_v2(input, output, delegate).await
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    futures_lite::future::block_on(async {
        let mut sessions = build_demo_sessions()?;
        let mut delegate = DemoDelegate::default();

        let mut session_id = 0usize;
        while let Some((mut input, mut output)) = sessions.pop_front() {
            session_id += 1;
            let outcome = handle_client_session(&mut delegate, &mut input, &mut output).await?;
            let report_status_lines = decode_report_status_lines(output.get_ref().as_slice())?;
            println!(
                "session #{session_id}: updates={}, push-options={}, report-status={report_status_lines:?}",
                outcome.updates_received, outcome.push_options_received
            );
        }

        println!("processed {} per-client sessions", delegate.sessions_processed);
        Ok::<(), Box<dyn std::error::Error>>(())
    })
}

fn build_demo_sessions()
-> Result<VecDeque<(futures_lite::io::Cursor<Vec<u8>>, futures_lite::io::Cursor<Vec<u8>>)>, Box<dyn std::error::Error>>
{
    let session_one = request_bytes_v2(
        &["report-status-v2", "agent=gitoxide-example"],
        &["0000000000000000000000000000000000000000 808e50d724f604f69ab93c6da2919c014667bedb refs/heads/main"],
        &[],
        b"PACK\0\0\0\x02",
    )?;
    let session_two = request_bytes_v2(
        &["report-status-v2", "push-options", "agent=gitoxide-example"],
        &["808e50d724f604f69ab93c6da2919c014667bedb 0000000000000000000000000000000000000000 refs/heads/main"],
        &["trace=1"],
        b"PACK\0\0\0\x02",
    )?;

    Ok(VecDeque::from(vec![
        (
            futures_lite::io::Cursor::new(session_one),
            futures_lite::io::Cursor::new(Vec::new()),
        ),
        (
            futures_lite::io::Cursor::new(session_two),
            futures_lite::io::Cursor::new(Vec::new()),
        ),
    ]))
}

fn request_bytes_v2(
    features: &[&str],
    updates: &[&str],
    push_options: &[&str],
    pack_data: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    if updates.is_empty() {
        return Err(std::io::Error::other("at least one update command is required").into());
    }

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

        if !push_options.is_empty() {
            encode::delim_to_write(writer.inner_mut())?;
            writer.write_all(b"section=push-options")?;
            for option in push_options {
                writer.write_all(option.as_bytes())?;
            }
        }
        encode::flush_to_write(writer.inner_mut())?;
    }

    out.extend_from_slice(pack_data);
    Ok(out)
}

fn decode_report_status_lines(output: &[u8]) -> Result<Vec<BString>, Box<dyn std::error::Error>> {
    let mut reader = StreamingPeekableIter::new(output, &[PacketLineRef::Flush], false);
    let mut lines = Vec::new();
    while let Some(line) = reader.read_line() {
        let line = line??;
        let text = line
            .as_text()
            .ok_or_else(|| std::io::Error::other("expected text packetline in report-status response"))?;
        lines.push(text.as_bstr().to_owned());
    }

    if reader.stopped_at() != Some(PacketLineRef::Flush) {
        return Err(std::io::Error::other("expected report-status response to terminate with flush").into());
    }
    Ok(lines)
}

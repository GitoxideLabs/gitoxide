//! Blocking server-side plumbing for `receive-pack` protocol interactions.
//!
//! This module parses incoming push command sections (including negotiated capabilities and
//! optional push-options), exposes the remaining input as pack data to a delegate, and writes
//! report-status responses in plain packet-line or sideband mode.

use std::io::{self, Write as _};

use bstr::{BStr, BString, ByteSlice, ByteVec};
use gix_transport::packetline::{
    Channel, PacketLineRef,
    blocking_io::{Writer, encode},
    decode,
};

type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

const MAX_SIDEBAND_DATA_BYTES: usize = 65_515;
const V2_SECTION_REF_UPDATES: &str = "section=ref-updates";
const V2_SECTION_PUSH_OPTIONS: &str = "section=push-options";
const V2_SECTION_REPORT_STATUS: &str = "report-status";
const V2_SECTION_MESSAGES: &str = "messages";

/// Async transport integration for receive-pack server plumbing.
#[cfg(feature = "async-client")]
pub mod async_io;

/// A parsed receive-pack capability from the first update command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capability {
    /// Capability name, like `report-status-v2` or `side-band-64k`.
    pub name: BString,
    /// Optional capability value for key-value capabilities.
    pub value: Option<BString>,
}

/// A parsed feature line from the V2 command header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feature {
    /// The feature name, like `agent` or `report-status-v2`.
    pub name: BString,
    /// Optional feature value for key-value features.
    pub value: Option<BString>,
}

/// A capability line to advertise in protocol V2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct V2Capability {
    /// Capability name, like `push` or `server-option`.
    pub name: BString,
    /// Optional capability values associated with `name`.
    pub values: Vec<BString>,
}

/// A single requested ref update in a push command list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    /// Expected old object id currently at `ref_name`.
    pub old_id: gix_hash::ObjectId,
    /// New object id to update `ref_name` to.
    pub new_id: gix_hash::ObjectId,
    /// Fully qualified reference name to update.
    pub ref_name: BString,
}

/// Parsed `receive-pack` request metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Parsed capabilities advertised by the client in the first update command.
    pub capabilities: Vec<Capability>,
    /// Parsed update commands from the command section.
    pub updates: Vec<Update>,
    /// Optional push-options section entries (if negotiated and provided).
    pub push_options: Vec<BString>,
}

/// Parsed receive-pack protocol V2 request metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct V2Request {
    /// Header features associated with the command request.
    pub features: Vec<Feature>,
    /// Parsed receive-pack request payload.
    pub request: Request,
    /// If true, additional bytes are present after the argument section and represent pack data.
    pub has_pack: bool,
}

impl Request {
    /// Returns true if the request contains a capability with `name`.
    pub fn has_capability(&self, name: &str) -> bool {
        let name = name.as_bytes().as_bstr();
        self.capabilities
            .iter()
            .any(|capability| capability.name.as_bstr() == name)
    }

    fn uses_sideband(&self) -> bool {
        self.has_capability("side-band") || self.has_capability("side-band-64k")
    }

    fn wants_report_status(&self) -> bool {
        self.has_capability("report-status") || self.has_capability("report-status-v2")
    }
}

/// Status of unpacking the received pack data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnpackStatus {
    /// Pack unpacking succeeded.
    Ok,
    /// Pack unpacking failed with a message.
    Error(BString),
}

/// Per-reference report-status entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefStatus {
    /// Reference update succeeded.
    Ok {
        /// Updated reference name.
        ref_name: BString,
    },
    /// Reference update failed with a message.
    Rejected {
        /// Rejected reference name.
        ref_name: BString,
        /// Rejection reason.
        message: BString,
    },
}

/// Kind of sideband message to send before/after report-status data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidebandMessageKind {
    /// Sideband progress channel (`2`).
    Progress,
    /// Sideband error channel (`3`).
    Error,
}

/// A sideband message emitted during push processing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebandMessage {
    /// Target sideband channel.
    pub kind: SidebandMessageKind,
    /// Message payload bytes.
    pub text: BString,
}

/// Delegate-provided receive-pack response data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// Overall unpack result.
    pub unpack_status: UnpackStatus,
    /// Per-reference results.
    pub ref_statuses: Vec<RefStatus>,
    /// Optional sideband progress/error messages.
    pub sideband_messages: Vec<SidebandMessage>,
}

impl Default for Response {
    fn default() -> Self {
        Response {
            unpack_status: UnpackStatus::Ok,
            ref_statuses: Vec::new(),
            sideband_messages: Vec::new(),
        }
    }
}

/// Outcome of serving one receive-pack push request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Number of update commands parsed from the request.
    pub updates_received: usize,
    /// Number of push-options parsed from the optional push-options section.
    pub push_options_received: usize,
    /// Number of per-ref statuses sent.
    pub ref_statuses_sent: usize,
    /// Whether a report-status payload was written.
    pub report_status_sent: bool,
    /// Number of bytes written onto sideband channels.
    pub sideband_bytes_sent: u64,
}

/// Delegate implementation used by [`serve_v1()`] to process received pushes.
pub trait Delegate {
    /// Process a parsed `receive-pack` request and consume pack data from `pack_data`.
    fn receive(&mut self, request: &Request, pack_data: &mut dyn io::Read) -> Result<Response, BoxError>;
}

/// Errors returned while parsing receive-pack requests and writing responses.
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Decode(#[from] decode::Error),
    #[error("Expected at least one update command before the command-section flush")]
    MissingUpdateCommands,
    #[error("Expected `command=<name>` in receive-pack V2 request header")]
    MissingV2Command,
    #[error("Unsupported receive-pack V2 command {command:?}")]
    UnsupportedV2Command { command: BString },
    #[error("Malformed receive-pack V2 header line {line:?}")]
    MalformedV2HeaderLine { line: BString },
    #[error("Expected a V2 argument section after the header delimiter")]
    MissingV2ArgumentSection,
    #[error("Expected at least one section in receive-pack V2 request arguments")]
    MissingV2SectionHeader,
    #[error("Expected a `section=ref-updates` section in receive-pack V2 request arguments")]
    MissingV2RefUpdatesSection,
    #[error("Unknown receive-pack V2 section header {section:?}")]
    UnknownV2Section { section: BString },
    #[error("Duplicate receive-pack V2 section header {section:?}")]
    DuplicateV2Section { section: BString },
    #[error("Unexpected packet line type {line_type} in receive-pack request section")]
    UnexpectedPacketLineType { line_type: &'static str },
    #[error("Malformed receive-pack update command line {line:?}")]
    MalformedCommandLine { line: BString },
    #[error("Could not parse object id in command line {line:?}")]
    InvalidObjectId {
        line: BString,
        #[source]
        source: gix_hash::decode::Error,
    },
    #[error("Delegate failed")]
    Delegate(#[source] BoxError),
}

/// Parse a protocol V1 receive-pack request from `input`, leaving `input` positioned at pack data.
///
/// The parser consumes:
/// - command section (`old new ref` lines) until flush
/// - optional push-options section if negotiated and present
///
/// Remaining bytes in `input` can be interpreted as pack data by the caller.
pub fn parse_v1_request(input: &mut impl io::BufRead) -> Result<Request, Error> {
    let mut command_lines = read_text_packet_lines_until_flush(input)?;
    if command_lines.is_empty() {
        return Err(Error::MissingUpdateCommands);
    }

    let first_line = command_lines.remove(0);
    let (first_command, capabilities) = split_first_command_and_capabilities(first_line.as_bstr());
    let mut updates = Vec::with_capacity(command_lines.len() + 1);
    updates.push(parse_update_command(first_command.as_bstr())?);
    for line in command_lines {
        updates.push(parse_update_command(line.as_bstr())?);
    }

    let push_options = if capabilities
        .iter()
        .any(|capability| capability.name.as_bstr() == "push-options".as_bytes().as_bstr())
    {
        read_optional_push_options(input)?
    } else {
        Vec::new()
    };

    Ok(Request {
        capabilities,
        updates,
        push_options,
    })
}

/// Parse a protocol V2 receive-pack request from `input`, leaving `input` positioned at optional pack data.
///
/// The parser consumes:
/// - command request header lines through delimiter (expects `command=push`)
/// - argument sections encoded as text packet lines:
///   - `section=ref-updates` (required)
///   - `section=push-options` (optional)
///   - each section terminated by delimiter or final flush
///
/// Remaining bytes in `input` can be interpreted as pack data by the caller.
pub fn parse_v2_request(input: &mut impl io::BufRead) -> Result<V2Request, Error> {
    let (header_lines, header_terminator) = read_text_packet_lines_until_delimiter_or_flush(input)?;
    if header_terminator != SectionTerminator::Delimiter {
        return Err(Error::MissingV2ArgumentSection);
    }

    let (command, features) = parse_v2_header_lines(header_lines)?;
    if command.as_bstr() != "push".as_bytes().as_bstr() {
        return Err(Error::UnsupportedV2Command { command });
    }

    let mut updates = None::<Vec<Update>>;
    let mut push_options = None::<Vec<BString>>;
    loop {
        let (section_lines, section_terminator) = read_text_packet_lines_until_delimiter_or_flush(input)?;
        if section_lines.is_empty() {
            return Err(Error::MissingV2SectionHeader);
        }
        let section = section_lines[0].clone();
        let mut payload = section_lines;
        payload.remove(0);

        match section.as_bstr() {
            section_name if section_name == V2_SECTION_REF_UPDATES.as_bytes().as_bstr() => {
                if updates.is_some() {
                    return Err(Error::DuplicateV2Section { section });
                }
                if payload.is_empty() {
                    return Err(Error::MissingUpdateCommands);
                }
                let mut parsed = Vec::with_capacity(payload.len());
                for line in payload {
                    parsed.push(parse_update_command(line.as_bstr())?);
                }
                updates = Some(parsed);
            }
            section_name if section_name == V2_SECTION_PUSH_OPTIONS.as_bytes().as_bstr() => {
                if push_options.is_some() {
                    return Err(Error::DuplicateV2Section { section });
                }
                push_options = Some(payload);
            }
            _ => return Err(Error::UnknownV2Section { section }),
        }

        if section_terminator == SectionTerminator::Flush {
            break;
        }
    }

    let updates = updates.ok_or(Error::MissingV2RefUpdatesSection)?;
    let capabilities = features
        .iter()
        .map(|feature| Capability {
            name: feature.name.clone(),
            value: feature.value.clone(),
        })
        .collect::<Vec<_>>();
    let has_pack = !input.fill_buf()?.is_empty();

    Ok(V2Request {
        features,
        request: Request {
            capabilities,
            updates,
            push_options: push_options.unwrap_or_default(),
        },
        has_pack,
    })
}

/// Serve one protocol V1 receive-pack push request end-to-end.
pub fn serve_v1(
    input: impl io::Read,
    mut output: impl io::Write,
    delegate: &mut impl Delegate,
) -> Result<Outcome, Error> {
    let mut input = io::BufReader::new(input);
    let request = parse_v1_request(&mut input)?;
    let response = delegate.receive(&request, &mut input).map_err(Error::Delegate)?;
    let report_status_sent = request.wants_report_status();
    let sideband_bytes_sent = write_v1_response(&mut output, &request, &response)?;

    Ok(Outcome {
        updates_received: request.updates.len(),
        push_options_received: request.push_options.len(),
        ref_statuses_sent: response.ref_statuses.len(),
        report_status_sent,
        sideband_bytes_sent,
    })
}

/// Write a receive-pack response matching `request` capabilities.
///
/// Returns the number of payload bytes written through sideband channels.
pub fn write_v1_response(mut output: impl io::Write, request: &Request, response: &Response) -> Result<u64, Error> {
    let mut sideband_bytes_sent = 0u64;
    let report_status_payload = request
        .wants_report_status()
        .then(|| encode_report_status_payload(response))
        .transpose()?;

    if request.uses_sideband() {
        for message in &response.sideband_messages {
            let channel = match message.kind {
                SidebandMessageKind::Progress => Channel::Progress,
                SidebandMessageKind::Error => Channel::Error,
            };
            let payload: &[u8] = message.text.as_ref();
            sideband_bytes_sent += payload.len() as u64;
            encode::band_to_write(channel, payload, &mut output)?;
        }
        if let Some(payload) = report_status_payload.as_ref() {
            for chunk in payload.chunks(MAX_SIDEBAND_DATA_BYTES) {
                sideband_bytes_sent += chunk.len() as u64;
                encode::band_to_write(Channel::Data, chunk, &mut output)?;
            }
        }
        encode::flush_to_write(&mut output)?;
        return Ok(sideband_bytes_sent);
    }

    if let Some(payload) = report_status_payload {
        output.write_all(&payload)?;
    } else {
        encode::flush_to_write(&mut output)?;
    }
    Ok(0)
}

/// Serve one protocol V2 receive-pack push request end-to-end.
pub fn serve_v2(
    input: impl io::Read,
    mut output: impl io::Write,
    delegate: &mut impl Delegate,
) -> Result<Outcome, Error> {
    let mut input = io::BufReader::new(input);
    let request = parse_v2_request(&mut input)?;
    let response = if request.has_pack {
        delegate
            .receive(&request.request, &mut input)
            .map_err(Error::Delegate)?
    } else {
        let mut empty = io::empty();
        delegate
            .receive(&request.request, &mut empty)
            .map_err(Error::Delegate)?
    };

    let report_status_sent = request.request.wants_report_status();
    let sideband_bytes_sent = write_v2_response(&mut output, &request.request, &response)?;

    Ok(Outcome {
        updates_received: request.request.updates.len(),
        push_options_received: request.request.push_options.len(),
        ref_statuses_sent: response.ref_statuses.len(),
        report_status_sent,
        sideband_bytes_sent,
    })
}

/// Write a protocol V2 capability advertisement, including the `version 2` line.
pub fn write_v2_capability_advertisement(
    mut output: impl io::Write,
    capabilities: &[V2Capability],
) -> Result<(), Error> {
    let mut writer = Writer::new(&mut output);
    writer.enable_text_mode();
    writer.write_all(b"version 2")?;
    for capability in capabilities {
        let mut line = capability.name.clone();
        if !capability.values.is_empty() {
            line.push_byte(b'=');
            for (idx, value) in capability.values.iter().enumerate() {
                if idx != 0 {
                    line.push_byte(b' ');
                }
                line.push_str(value);
            }
        }
        writer.write_all(line.as_ref())?;
    }
    encode::flush_to_write(writer.inner_mut())?;
    Ok(())
}

/// Write a receive-pack V2 response with sectioned report-status and optional message sections.
pub fn write_v2_response(mut output: impl io::Write, request: &Request, response: &Response) -> Result<u64, Error> {
    let mut writer = Writer::new(&mut output);
    writer.enable_text_mode();
    let mut wrote_section = false;

    if request.wants_report_status() {
        writer.write_all(V2_SECTION_REPORT_STATUS.as_bytes())?;
        writer.write_all(format_unpack_status_line(&response.unpack_status).as_ref())?;
        for status in &response.ref_statuses {
            writer.write_all(format_ref_status_line(status).as_ref())?;
        }
        wrote_section = true;
    }

    if !response.sideband_messages.is_empty() {
        if wrote_section {
            encode::delim_to_write(writer.inner_mut())?;
        }
        writer.write_all(V2_SECTION_MESSAGES.as_bytes())?;
        for message in &response.sideband_messages {
            writer.write_all(format_v2_message_line(message).as_ref())?;
        }
    }

    encode::flush_to_write(writer.inner_mut())?;
    Ok(0)
}

fn read_optional_push_options(input: &mut impl io::BufRead) -> Result<Vec<BString>, Error> {
    let Some(first_byte) = input.fill_buf()?.first().copied() else {
        return Ok(Vec::new());
    };
    if !first_byte.is_ascii_hexdigit() {
        return Ok(Vec::new());
    }
    read_text_packet_lines_until_flush(input)
}

fn read_text_packet_lines_until_flush(input: &mut impl io::BufRead) -> Result<Vec<BString>, Error> {
    let mut lines = Vec::new();
    loop {
        let mut hex_bytes = [0u8; 4];
        input.read_exact(&mut hex_bytes)?;
        match decode::hex_prefix(&hex_bytes)? {
            decode::PacketLineOrWantedSize::Line(PacketLineRef::Flush) => break,
            decode::PacketLineOrWantedSize::Line(other) => {
                return Err(Error::UnexpectedPacketLineType {
                    line_type: packet_line_kind(&other),
                });
            }
            decode::PacketLineOrWantedSize::Wanted(data_len) => {
                let mut data = vec![0u8; data_len as usize];
                input.read_exact(&mut data)?;
                if data.last() == Some(&b'\n') {
                    data.pop();
                    if data.last() == Some(&b'\r') {
                        data.pop();
                    }
                }
                lines.push(data.into());
            }
        }
    }
    Ok(lines)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SectionTerminator {
    Delimiter,
    Flush,
}

fn read_text_packet_lines_until_delimiter_or_flush(
    input: &mut impl io::BufRead,
) -> Result<(Vec<BString>, SectionTerminator), Error> {
    let mut lines = Vec::new();
    loop {
        let mut hex_bytes = [0u8; 4];
        input.read_exact(&mut hex_bytes)?;
        match decode::hex_prefix(&hex_bytes)? {
            decode::PacketLineOrWantedSize::Line(PacketLineRef::Delimiter) => {
                return Ok((lines, SectionTerminator::Delimiter));
            }
            decode::PacketLineOrWantedSize::Line(PacketLineRef::Flush) => return Ok((lines, SectionTerminator::Flush)),
            decode::PacketLineOrWantedSize::Line(other) => {
                return Err(Error::UnexpectedPacketLineType {
                    line_type: packet_line_kind(&other),
                });
            }
            decode::PacketLineOrWantedSize::Wanted(data_len) => {
                let mut data = vec![0u8; data_len as usize];
                input.read_exact(&mut data)?;
                if data.last() == Some(&b'\n') {
                    data.pop();
                    if data.last() == Some(&b'\r') {
                        data.pop();
                    }
                }
                lines.push(data.into());
            }
        }
    }
}

fn parse_v2_header_lines(lines: Vec<BString>) -> Result<(BString, Vec<Feature>), Error> {
    let mut command = None::<BString>;
    let mut features = Vec::new();
    for line in lines {
        let bytes: &[u8] = line.as_ref();
        if let Some(command_name) = bytes.strip_prefix(b"command=") {
            if command.is_some() || command_name.is_empty() {
                return Err(Error::MalformedV2HeaderLine { line });
            }
            command = Some(command_name.into());
            continue;
        }
        features.push(parse_v2_feature_line(line.as_bstr())?);
    }
    let command = command.ok_or(Error::MissingV2Command)?;
    Ok((command, features))
}

fn parse_v2_feature_line(line: &BStr) -> Result<Feature, Error> {
    if let Some((name, value)) = split_once(line, b'=') {
        if name.is_empty() {
            return Err(Error::MalformedV2HeaderLine { line: line.to_owned() });
        }
        return Ok(Feature {
            name: name.to_owned(),
            value: Some(value.to_owned()),
        });
    }
    if line.is_empty() {
        return Err(Error::MalformedV2HeaderLine { line: line.to_owned() });
    }
    Ok(Feature {
        name: line.to_owned(),
        value: None,
    })
}

fn split_first_command_and_capabilities(line: &BStr) -> (BString, Vec<Capability>) {
    match line.find_byte(0) {
        Some(nul_pos) => {
            let command = line[..nul_pos].as_bstr().to_owned();
            let capabilities = parse_capabilities(line[nul_pos + 1..].as_bstr());
            (command, capabilities)
        }
        None => (line.to_owned(), Vec::new()),
    }
}

fn parse_capabilities(raw: &BStr) -> Vec<Capability> {
    raw.split(|byte| *byte == b' ')
        .filter(|token| !token.is_empty())
        .map(|token| {
            let token = token.as_bstr();
            if let Some((name, value)) = split_once(token, b'=') {
                Capability {
                    name: name.to_owned(),
                    value: Some(value.to_owned()),
                }
            } else {
                Capability {
                    name: token.to_owned(),
                    value: None,
                }
            }
        })
        .collect()
}

fn parse_update_command(line: &BStr) -> Result<Update, Error> {
    if line.find_byte(0).is_some() {
        return Err(Error::MalformedCommandLine { line: line.to_owned() });
    }

    let mut tokens = line.splitn(3, |byte| *byte == b' ');
    let old_hex = tokens
        .next()
        .ok_or_else(|| Error::MalformedCommandLine { line: line.to_owned() })?;
    let new_hex = tokens
        .next()
        .ok_or_else(|| Error::MalformedCommandLine { line: line.to_owned() })?;
    let ref_name = tokens
        .next()
        .ok_or_else(|| Error::MalformedCommandLine { line: line.to_owned() })?;
    if old_hex.is_empty() || new_hex.is_empty() || ref_name.is_empty() {
        return Err(Error::MalformedCommandLine { line: line.to_owned() });
    }

    let old_id = gix_hash::ObjectId::from_hex(old_hex).map_err(|source| Error::InvalidObjectId {
        line: line.to_owned(),
        source,
    })?;
    let new_id = gix_hash::ObjectId::from_hex(new_hex).map_err(|source| Error::InvalidObjectId {
        line: line.to_owned(),
        source,
    })?;

    Ok(Update {
        old_id,
        new_id,
        ref_name: ref_name.as_bstr().to_owned(),
    })
}

fn encode_report_status_payload(response: &Response) -> Result<Vec<u8>, Error> {
    let mut payload = Vec::new();
    let mut writer = Writer::new(&mut payload);
    writer.enable_text_mode();
    writer.write_all(format_unpack_status_line(&response.unpack_status).as_ref())?;
    for status in &response.ref_statuses {
        writer.write_all(format_ref_status_line(status).as_ref())?;
    }
    encode::flush_to_write(writer.inner_mut())?;
    Ok(payload)
}

fn format_unpack_status_line(status: &UnpackStatus) -> BString {
    match status {
        UnpackStatus::Ok => "unpack ok".into(),
        UnpackStatus::Error(message) => {
            let mut line = BString::from("unpack ");
            line.push_str(message);
            line
        }
    }
}

fn format_ref_status_line(status: &RefStatus) -> BString {
    match status {
        RefStatus::Ok { ref_name } => {
            let mut line = BString::from("ok ");
            line.push_str(ref_name);
            line
        }
        RefStatus::Rejected { ref_name, message } => {
            let mut line = BString::from("ng ");
            line.push_str(ref_name);
            line.push_byte(b' ');
            line.push_str(message);
            line
        }
    }
}

fn format_v2_message_line(message: &SidebandMessage) -> BString {
    match message.kind {
        SidebandMessageKind::Progress => {
            let mut line = BString::from("progress ");
            line.push_str(&message.text);
            line
        }
        SidebandMessageKind::Error => {
            let mut line = BString::from("error ");
            line.push_str(&message.text);
            line
        }
    }
}

fn split_once(line: &BStr, separator: u8) -> Option<(&BStr, &BStr)> {
    let idx = line.find_byte(separator)?;
    Some((line[..idx].as_bstr(), line[idx + 1..].as_bstr()))
}

fn packet_line_kind(line: &PacketLineRef<'_>) -> &'static str {
    match line {
        PacketLineRef::Data(_) => "data",
        PacketLineRef::Flush => "flush",
        PacketLineRef::Delimiter => "delimiter",
        PacketLineRef::ResponseEnd => "response-end",
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use gix_transport::packetline::{BandRef, PacketLineRef, blocking_io::StreamingPeekableIter};

    use super::*;

    #[derive(Default)]
    struct MockDelegate {
        response: Response,
        seen_request: Option<Request>,
        seen_pack_prefix: Option<[u8; 4]>,
    }

    impl Delegate for MockDelegate {
        fn receive(&mut self, request: &Request, pack_data: &mut dyn io::Read) -> Result<Response, BoxError> {
            self.seen_request = Some(request.clone());
            let mut prefix = [0u8; 4];
            pack_data.read_exact(&mut prefix)?;
            self.seen_pack_prefix = Some(prefix);
            Ok(self.response.clone())
        }
    }

    #[test]
    fn serve_v1_parses_commands_capabilities_and_writes_sideband_report_status()
    -> Result<(), Box<dyn std::error::Error>> {
        let request = request_bytes(
            &["0000000000000000000000000000000000000000 808e50d724f604f69ab93c6da2919c014667bedb refs/heads/main"],
            &[
                "report-status-v2",
                "side-band-64k",
                "object-format=sha1",
                "agent=git/gitplane",
            ],
            &[],
            b"PACK\0\0\0\x02",
        )?;
        let mut output = Vec::new();
        let mut delegate = MockDelegate {
            response: Response {
                unpack_status: UnpackStatus::Ok,
                ref_statuses: vec![RefStatus::Ok {
                    ref_name: "refs/heads/main".into(),
                }],
                sideband_messages: Vec::new(),
            },
            ..Default::default()
        };

        let outcome = serve_v1(request.as_slice(), &mut output, &mut delegate)?;
        assert_eq!(
            outcome,
            Outcome {
                updates_received: 1,
                push_options_received: 0,
                ref_statuses_sent: 1,
                report_status_sent: true,
                sideband_bytes_sent: 41,
            }
        );

        let seen = delegate
            .seen_request
            .as_ref()
            .expect("request should be visible to delegate");
        assert_eq!(seen.updates.len(), 1);
        assert_eq!(
            seen.updates[0].ref_name.as_bstr(),
            "refs/heads/main".as_bytes().as_bstr()
        );
        assert!(seen.has_capability("report-status-v2"));
        assert!(seen.has_capability("side-band-64k"));
        assert_eq!(
            delegate.seen_pack_prefix,
            Some(*b"PACK"),
            "delegate should receive pack data at the current read position"
        );

        let mut outer_reader = StreamingPeekableIter::new(output.as_slice(), &[PacketLineRef::Flush], false);
        let mut report_payload = Vec::<u8>::new();
        while let Some(line) = outer_reader.read_line() {
            let line = line??;
            match line.decode_band()? {
                BandRef::Data(data) => report_payload.extend_from_slice(data),
                BandRef::Progress(_) | BandRef::Error(_) => {}
            }
        }
        assert_eq!(outer_reader.stopped_at(), Some(PacketLineRef::Flush));

        let mut inner_reader = StreamingPeekableIter::new(report_payload.as_slice(), &[PacketLineRef::Flush], false);
        assert_eq!(
            next_text_line(&mut inner_reader)?.as_bstr(),
            "unpack ok".as_bytes().as_bstr()
        );
        assert_eq!(
            next_text_line(&mut inner_reader)?.as_bstr(),
            "ok refs/heads/main".as_bytes().as_bstr()
        );
        assert!(inner_reader.read_line().is_none());
        assert_eq!(inner_reader.stopped_at(), Some(PacketLineRef::Flush));
        Ok(())
    }

    #[test]
    fn serve_v1_parses_push_options_section_when_negotiated() -> Result<(), Box<dyn std::error::Error>> {
        let request = request_bytes(
            &["0000000000000000000000000000000000000000 808e50d724f604f69ab93c6da2919c014667bedb refs/heads/main"],
            &["report-status", "push-options"],
            &["ci.skip", "trace=1"],
            b"PACK\0\0\0\x02",
        )?;
        let mut output = Vec::new();
        let mut delegate = MockDelegate {
            response: Response {
                unpack_status: UnpackStatus::Ok,
                ref_statuses: vec![RefStatus::Ok {
                    ref_name: "refs/heads/main".into(),
                }],
                sideband_messages: Vec::new(),
            },
            ..Default::default()
        };

        let outcome = serve_v1(request.as_slice(), &mut output, &mut delegate)?;
        assert_eq!(outcome.push_options_received, 2);

        let seen = delegate
            .seen_request
            .as_ref()
            .expect("request should be visible to delegate");
        assert_eq!(
            seen.push_options,
            vec![BString::from("ci.skip"), BString::from("trace=1")]
        );
        assert_eq!(delegate.seen_pack_prefix, Some(*b"PACK"));

        let mut reader = StreamingPeekableIter::new(output.as_slice(), &[PacketLineRef::Flush], false);
        assert_eq!(next_text_line(&mut reader)?.as_bstr(), "unpack ok".as_bytes().as_bstr());
        assert_eq!(
            next_text_line(&mut reader)?.as_bstr(),
            "ok refs/heads/main".as_bytes().as_bstr()
        );
        assert!(reader.read_line().is_none());
        assert_eq!(reader.stopped_at(), Some(PacketLineRef::Flush));
        Ok(())
    }

    #[test]
    fn parse_v1_request_rejects_malformed_update_line() -> Result<(), Box<dyn std::error::Error>> {
        let request = request_bytes(&["not-an-update-line"], &[], &[], b"PACK\0\0\0\x02")?;
        let mut input = std::io::BufReader::new(Cursor::new(request));
        let err = parse_v1_request(&mut input).expect_err("malformed command line should fail");
        assert!(matches!(err, Error::MalformedCommandLine { .. }));
        Ok(())
    }

    #[test]
    fn parse_v2_request_parses_sections_and_leaves_pack_data() -> Result<(), Box<dyn std::error::Error>> {
        let request = request_bytes_v2(
            &["report-status-v2", "push-options", "agent=git/gitplane"],
            &["0000000000000000000000000000000000000000 808e50d724f604f69ab93c6da2919c014667bedb refs/heads/main"],
            &["ci.skip", "trace=1"],
            b"PACK\0\0\0\x02",
        )?;
        let mut input = std::io::BufReader::new(Cursor::new(request));
        let parsed = parse_v2_request(&mut input)?;

        assert_eq!(
            parsed.features,
            vec![
                Feature {
                    name: "report-status-v2".into(),
                    value: None,
                },
                Feature {
                    name: "push-options".into(),
                    value: None,
                },
                Feature {
                    name: "agent".into(),
                    value: Some("git/gitplane".into()),
                },
            ]
        );
        assert!(parsed.has_pack);
        assert_eq!(parsed.request.updates.len(), 1);
        assert_eq!(
            parsed.request.push_options,
            vec![BString::from("ci.skip"), BString::from("trace=1")]
        );
        assert!(parsed.request.has_capability("report-status-v2"));
        assert!(parsed.request.has_capability("push-options"));
        assert_eq!(
            parsed.request.updates[0].ref_name.as_bstr(),
            "refs/heads/main".as_bytes().as_bstr()
        );

        let mut prefix = [0u8; 4];
        std::io::Read::read_exact(&mut input, &mut prefix)?;
        assert_eq!(prefix, *b"PACK");
        Ok(())
    }

    #[test]
    fn parse_v2_request_rejects_unknown_section() -> Result<(), Box<dyn std::error::Error>> {
        let mut out = Vec::new();
        {
            let mut writer = Writer::new(&mut out);
            writer.enable_text_mode();
            writer.write_all(b"command=push")?;
            encode::delim_to_write(writer.inner_mut())?;
            writer.write_all(b"section=unknown")?;
            writer.write_all(
                b"0000000000000000000000000000000000000000 808e50d724f604f69ab93c6da2919c014667bedb refs/heads/main",
            )?;
            encode::flush_to_write(writer.inner_mut())?;
        }
        let mut input = std::io::BufReader::new(Cursor::new(out));
        let err = parse_v2_request(&mut input).expect_err("unknown V2 section should fail");
        assert!(matches!(err, Error::UnknownV2Section { .. }));
        Ok(())
    }

    #[test]
    fn serve_v2_parses_sections_and_writes_report_status() -> Result<(), Box<dyn std::error::Error>> {
        let request = request_bytes_v2(
            &["report-status-v2", "push-options"],
            &["0000000000000000000000000000000000000000 808e50d724f604f69ab93c6da2919c014667bedb refs/heads/main"],
            &["trace=1"],
            b"PACK\0\0\0\x02",
        )?;
        let mut output = Vec::new();
        let mut delegate = MockDelegate {
            response: Response {
                unpack_status: UnpackStatus::Ok,
                ref_statuses: vec![RefStatus::Ok {
                    ref_name: "refs/heads/main".into(),
                }],
                sideband_messages: Vec::new(),
            },
            ..Default::default()
        };

        let outcome = serve_v2(request.as_slice(), &mut output, &mut delegate)?;
        assert_eq!(
            outcome,
            Outcome {
                updates_received: 1,
                push_options_received: 1,
                ref_statuses_sent: 1,
                report_status_sent: true,
                sideband_bytes_sent: 0,
            }
        );
        assert_eq!(delegate.seen_pack_prefix, Some(*b"PACK"));
        let seen = delegate
            .seen_request
            .as_ref()
            .expect("request should be visible to delegate");
        assert_eq!(seen.push_options, vec![BString::from("trace=1")]);
        assert!(seen.has_capability("report-status-v2"));

        let mut reader = StreamingPeekableIter::new(output.as_slice(), &[PacketLineRef::Flush], false);
        assert_eq!(
            next_text_line(&mut reader)?.as_bstr(),
            V2_SECTION_REPORT_STATUS.as_bytes().as_bstr()
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

    #[test]
    fn write_v2_capability_advertisement_includes_version_and_values() -> Result<(), Box<dyn std::error::Error>> {
        let mut output = Vec::new();
        write_v2_capability_advertisement(
            &mut output,
            &[
                V2Capability {
                    name: "push".into(),
                    values: vec!["report-status-v2".into(), "push-options".into()],
                },
                V2Capability {
                    name: "object-format".into(),
                    values: vec!["sha1".into()],
                },
            ],
        )?;

        let mut reader = StreamingPeekableIter::new(output.as_slice(), &[PacketLineRef::Flush], false);
        assert_eq!(next_text_line(&mut reader)?.as_bstr(), "version 2".as_bytes().as_bstr());
        assert_eq!(
            next_text_line(&mut reader)?.as_bstr(),
            "push=report-status-v2 push-options".as_bytes().as_bstr()
        );
        assert_eq!(
            next_text_line(&mut reader)?.as_bstr(),
            "object-format=sha1".as_bytes().as_bstr()
        );
        assert!(reader.read_line().is_none());
        assert_eq!(reader.stopped_at(), Some(PacketLineRef::Flush));
        Ok(())
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
            writer.write_all(V2_SECTION_REF_UPDATES.as_bytes())?;
            for update in updates {
                writer.write_all(update.as_bytes())?;
            }
            if push_options.is_empty() {
                encode::flush_to_write(writer.inner_mut())?;
            } else {
                encode::delim_to_write(writer.inner_mut())?;
                writer.write_all(V2_SECTION_PUSH_OPTIONS.as_bytes())?;
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

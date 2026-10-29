use bstr::{BStr, BString, ByteSlice};
use gix_error::{OptionExt, Result, ResultExt, bail, message};
use gix_hash::{Kind, ObjectId};
use gix_transport::client::Capabilities;

/// A reference update sent to receive-pack.
#[derive(Debug, Clone)]
pub struct Command {
    /// The fully qualified destination reference.
    pub name: gix_ref::FullName,
    /// The advertised value, or a null ID when creating the reference.
    ///
    /// The server compares this value under its reference lock, protecting against
    /// updates made after the advertisement. Callers apply any additional lease
    /// constraints before constructing the command.
    pub old_id: ObjectId,
    /// The desired value, or a null ID when deleting the reference.
    pub new_id: ObjectId,
}

/// Features a caller can request in addition to reliable status reports.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options<'a> {
    /// Require all reference updates to succeed or fail together.
    pub atomic: bool,
    /// Strings passed to server-side receive hooks. NUL and LF are prohibited.
    pub push_options: &'a [BString],
    /// An optional client identification, without whitespace or NUL.
    pub agent: Option<&'a BStr>,
    /// Ask the server to suppress pack progress if it supports doing so.
    pub quiet: bool,
}

/// The negotiated receive-pack status report format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportStatus {
    /// One status per requested reference.
    V1,
    /// Statuses can describe references rewritten by a proc-receive hook.
    V2,
}

/// Negotiated features that affect pack generation and response decoding.
#[derive(Debug, Clone, Copy)]
pub struct Features {
    /// The status report format that will be sent by the server.
    pub report_status: ReportStatus,
    /// The response multiplexes pack status and progress using sidebands.
    pub side_band_64k: bool,
    /// The generated pack may refer to a delta base by its pack offset.
    pub ofs_delta: bool,
    /// The generated pack may omit delta bases already present on the server.
    pub thin_pack: bool,
}

/// A validated request, ready to send after a receive-pack handshake.
///
/// Servers must support `report-status` or `report-status-v2`, so callers can
/// distinguish confirmed reference updates from an interrupted connection.
#[derive(Debug, Clone)]
pub struct Request {
    commands: Vec<Command>,
    object_hash: Kind,
    features: Features,
    capability_line: BString,
    push_options: Vec<BString>,
}

impl Request {
    /// Validate `commands` and negotiate features against the server advertisement.
    /// Unsupported mandatory options fail before any update is sent.
    pub fn new(
        commands: Vec<Command>,
        object_hash: Kind,
        capabilities: &Capabilities,
        options: Options<'_>,
    ) -> Result<Self> {
        let mut selected = Vec::<BString>::new();
        let report_status = if capabilities.contains("report-status-v2") {
            selected.push("report-status-v2".into());
            ReportStatus::V2
        } else if capabilities.contains("report-status") {
            selected.push("report-status".into());
            ReportStatus::V1
        } else {
            bail!(message("The remote does not support receive-pack status reports").unsupported());
        };
        let side_band_64k = capabilities.contains("side-band-64k");
        if side_band_64k {
            selected.push("side-band-64k".into());
        }
        if options.quiet && capabilities.contains("quiet") {
            selected.push("quiet".into());
        }
        for (requested, capability) in [
            (options.atomic, "atomic"),
            (!options.push_options.is_empty(), "push-options"),
        ] {
            if requested {
                if !capabilities.contains(capability) {
                    bail!("The remote does not support {capability}".unsupported());
                }
                selected.push(capability.into());
            }
        }
        let hash_name = object_hash.to_string();
        if let Some(format) = capabilities.capability("object-format") {
            if format.value() != Some(hash_name.as_bytes().as_bstr()) {
                bail!("The remote does not support object format {hash_name}".unsupported());
            }
            selected.push(format!("object-format={hash_name}").into());
        } else if hash_name != "sha1" {
            bail!(message("A remote without object-format support requires SHA-1").unsupported());
        }
        if let Some(agent) = options.agent {
            if agent.is_empty() || agent.iter().any(|byte| *byte == 0 || byte.is_ascii_whitespace()) {
                bail!(message("The push agent must be a nonempty token without whitespace or NUL").validation());
            }
            if capabilities.contains("agent") {
                let mut feature = BString::from("agent=");
                feature.extend_from_slice(agent);
                selected.push(feature);
            }
        }
        for option in options.push_options {
            if option.find_byteset(b"\0\n").is_some() || option.len() > 65516 {
                bail!(message("Push options must fit in a packet line and cannot contain NUL or LF").validation());
            }
        }
        let mut names = std::collections::HashSet::new();
        for command in &commands {
            let name = command.name.as_bstr();
            if !name.starts_with(b"refs/") || (!command.new_id.is_null() && !name[5..].contains(&b'/')) {
                bail!("The remote cannot update reference {name:?}".validation());
            }
            if !names.insert(name) {
                bail!("More than one push command updates {name:?}".validation());
            }
            if command.old_id.kind() != object_hash || command.new_id.kind() != object_hash {
                bail!(message("Push command object IDs must use the repository's object format").validation());
            }
            if command.new_id.is_null() && !capabilities.contains("delete-refs") {
                bail!(message("The remote does not support deleting references").unsupported());
            }
        }
        let request = Request {
            commands,
            object_hash,
            features: Features {
                report_status,
                side_band_64k,
                ofs_delta: capabilities.contains("ofs-delta"),
                thin_pack: !capabilities.contains("no-thin"),
            },
            capability_line: selected.join(&b' ').into(),
            push_options: options.push_options.to_vec(),
        };
        if request.command_lines().any(|line| line.len() > 65516) {
            bail!(message("A push command exceeds the packet line size limit").validation());
        }
        Ok(request)
    }

    /// The commands that will be sent, in the caller's order.
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }

    /// Features to respect when generating a pack and decoding the response.
    pub fn features(&self) -> Features {
        self.features
    }

    /// The options to send as individual packet lines after the commands' flush.
    pub fn push_options(&self) -> &[BString] {
        &self.push_options
    }

    /// Produce command packet payloads, with capabilities following a NUL in the first.
    /// A flush packet must follow the final command, and again after any push options.
    pub fn command_lines(&self) -> impl Iterator<Item = BString> + '_ {
        self.commands.iter().enumerate().map(|(index, command)| {
            let mut line = BString::from(format!("{} {} ", command.old_id, command.new_id));
            line.extend_from_slice(command.name.as_bstr());
            if index == 0 {
                line.push(0);
                line.extend_from_slice(&self.capability_line);
            }
            line
        })
    }

    /// Decode the complete status report's packet payloads, after removing any sidebands.
    ///
    /// Reject malformed, missing, or unsolicited statuses rather than reporting an
    /// unconfirmed update as successful. Unknown v2 options are ignored for extensibility.
    pub fn parse_response(&self, lines: impl IntoIterator<Item = impl AsRef<[u8]>>) -> Result<Response> {
        let mut lines = lines.into_iter();
        let first = lines
            .next()
            .ok_or_raise(|| message("The remote omitted the unpack status").corrupted())?;
        let first = report_line(first.as_ref())?;
        let unpack = first
            .strip_prefix(b"unpack ")
            .filter(|status| !status.is_empty())
            .ok_or_raise(|| message("The first push status must report the unpack result").corrupted())?;
        let mut response = Response {
            unpack_error: (unpack != b"ok").then(|| unpack.into()),
            refs: Vec::new(),
        };
        for line in lines {
            let line = report_line(line.as_ref())?;
            if let Some(option) = line.strip_prefix(b"option ") {
                if self.features.report_status != ReportStatus::V2 {
                    bail!(message("The remote sent v2 options in a v1 status report").corrupted());
                }
                let status = response
                    .refs
                    .last_mut()
                    .filter(|status| status.error.is_none())
                    .ok_or_raise(|| {
                        message("A push status option must follow a successful reference status").corrupted()
                    })?;
                status.report.parse_option(option, self.object_hash)?;
                continue;
            }
            let (name, error) = if let Some(name) = line.strip_prefix(b"ok ") {
                (name, None)
            } else if let Some(status) = line.strip_prefix(b"ng ") {
                let (name, reason) = status
                    .split_once_str(b" ")
                    .filter(|(_, reason)| !reason.is_empty())
                    .ok_or_raise(|| message("A rejected push status must include a reason").corrupted())?;
                (name, Some(BString::from(reason)))
            } else {
                bail!("Invalid push status {line:?}".corrupted());
            };
            if !self.commands.iter().any(|command| command.name.as_bstr() == name) {
                bail!("The remote reported an unrequested reference {name:?}".corrupted());
            }
            if let Some(previous) = response.refs.iter().find(|status| status.name.as_bstr() == name)
                && (self.features.report_status == ReportStatus::V1 || previous.error.is_some() || error.is_some())
            {
                bail!("The remote reported conflicting or duplicate statuses for {name:?}".corrupted());
            }
            response.refs.push(RefStatus {
                name: gix_ref::FullName::try_from(name.as_bstr())
                    .or_raise(|| message("Invalid reference name in push status"))?,
                error,
                report: Report::default(),
            });
        }
        for command in &self.commands {
            if !response.refs.iter().any(|status| status.name == command.name) {
                bail!(
                    "The remote omitted the status of {:?}".corrupted(),
                    command.name.as_bstr()
                );
            }
        }
        Ok(response)
    }
}

/// Confirmed results returned by receive-pack.
#[derive(Debug, Clone)]
pub struct Response {
    /// The unpack failure, or `None` if the server accepted the pack.
    pub unpack_error: Option<BString>,
    /// Statuses in the server's order. V2 may report several destinations for one command.
    pub refs: Vec<RefStatus>,
}

/// A server's status for one command, with optional proc-receive information.
#[derive(Debug, Clone)]
pub struct RefStatus {
    /// The destination in the original command.
    pub name: gix_ref::FullName,
    /// The rejection reason, or `None` for a confirmed successful update.
    pub error: Option<BString>,
    /// Details of a destination rewritten by a proc-receive hook.
    pub report: Report,
}

/// Additional information in a v2 status report.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// The actual reference updated by the server.
    pub ref_name: Option<gix_ref::FullName>,
    /// The value before the server's update.
    pub old_id: Option<ObjectId>,
    /// The value after the server's update.
    pub new_id: Option<ObjectId>,
    /// The server reports that this update required force.
    pub forced_update: bool,
}

impl Report {
    fn parse_option(&mut self, option: &[u8], object_hash: Kind) -> Result<()> {
        let (key, value) = option
            .split_once_str(b" ")
            .map_or((option, None), |(key, value)| (key, Some(value)));
        match key {
            b"refname" => {
                self.ref_name = Some(
                    gix_ref::FullName::try_from(
                        value
                            .ok_or_raise(|| message("Missing reported reference name").corrupted())?
                            .as_bstr(),
                    )
                    .or_raise(|| message("Invalid reported reference name").corrupted())?,
                );
            }
            b"old-oid" | b"new-oid" => {
                let id = ObjectId::from_hex(value.ok_or_raise(|| message("Missing reported object ID").corrupted())?)
                    .or_raise(|| message("Invalid reported object ID").corrupted())?;
                if id.kind() != object_hash {
                    bail!(message("Reported object IDs have the wrong object format").corrupted());
                }
                if key == b"old-oid" {
                    self.old_id = Some(id);
                } else {
                    self.new_id = Some(id);
                }
            }
            b"forced-update" => {
                if value.is_some() {
                    bail!(message("The forced-update option does not take a value").corrupted());
                }
                self.forced_update = true;
            }
            _ => {}
        }
        Ok(())
    }
}

fn report_line(line: &[u8]) -> Result<&[u8]> {
    let line = line.strip_suffix(b"\n").unwrap_or(line);
    if line.find_byteset(b"\0\n").is_some() {
        bail!(message("Invalid NUL or embedded newline in push status").corrupted());
    }
    Ok(line)
}

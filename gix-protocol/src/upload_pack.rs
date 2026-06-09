//! Blocking server-side plumbing for `upload-pack` protocol V2 interactions.
//!
//! This module provides in-process request/response handling primitives intended for
//! server integrations that own connection handling and authentication.
//! It focuses on protocol framing and command parsing/writing while delegating repository
//! access and pack generation to caller-provided implementations.

/// Async transport integration for upload-pack server plumbing.
#[cfg(feature = "async-server")]
pub mod async_io;

use std::{
    collections::BTreeSet,
    io,
    sync::atomic::AtomicBool,
};
#[cfg(any(feature = "blocking-server", feature = "async-server"))]
use std::io::Write as _;

use bstr::{BString, ByteSlice};
#[cfg(any(feature = "blocking-server", feature = "async-server"))]
use bstr::{BStr, ByteVec};
use gix_ref::file::ReferenceExt as _;
#[cfg(any(feature = "blocking-server", feature = "async-server"))]
use gix_transport::packetline::blocking_io::{StreamingPeekableIter, Writer, encode};
#[cfg(any(feature = "blocking-server", feature = "async-server"))]
use gix_transport::packetline::Channel;
#[cfg(any(feature = "blocking-server", feature = "async-server"))]
use gix_transport::packetline::PacketLineRef;

use crate::{
    fetch::response::{Acknowledgement, ShallowUpdate, WantedRef},
    handshake::Ref,
};

type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[cfg(any(feature = "blocking-server", feature = "async-server"))]
const HEADER_DELIMITERS: &[PacketLineRef<'static>] = &[PacketLineRef::Delimiter, PacketLineRef::Flush];
#[cfg(any(feature = "blocking-server", feature = "async-server"))]
const ARGUMENT_DELIMITERS: &[PacketLineRef<'static>] = &[PacketLineRef::Flush];
#[allow(dead_code)] // Used by async_io submodule in later tasks.
pub(crate) const MAX_SIDEBAND_DATA_BYTES: usize = 65_515;

/// A parsed feature line from a protocol V2 request header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feature {
    /// The feature name, e.g. `agent`.
    pub name: BString,
    /// An optional feature value, e.g. `git/2.48.0`.
    pub value: Option<BString>,
}

/// A capability line to advertise in protocol V2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capability {
    /// The capability name, like `ls-refs` or `fetch`.
    pub name: BString,
    /// Optional values associated with `name`, separated by spaces when rendered.
    pub values: Vec<BString>,
}

/// Server-side configuration for upload-pack capability validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerConfig {
    /// The object hash algorithm this server supports.
    pub object_hash: gix_hash::Kind,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            object_hash: gix_hash::Kind::Sha1,
        }
    }
}

/// A parsed protocol V2 request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Request header features that accompany the command.
    pub features: Vec<Feature>,
    /// The upload-pack command payload.
    pub command: Command,
}

/// Parsed upload-pack command variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// A `ls-refs` command.
    LsRefs(LsRefs),
    /// A `fetch` command.
    Fetch(Fetch),
}

/// Parsed `ls-refs` command arguments.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LsRefs {
    /// If true, include symbolic reference targets in output.
    pub symrefs: bool,
    /// If true, include peeled object IDs where available.
    pub peel: bool,
    /// If true, include unborn refs in output.
    pub unborn: bool,
    /// Prefix filters to apply to advertised refs.
    pub ref_prefixes: Vec<BString>,
    /// Unknown arguments preserved for higher-level handling.
    pub extra_arguments: Vec<BString>,
}

/// Parsed `fetch` command arguments.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Fetch {
    /// Requested object IDs.
    pub wants: Vec<gix_hash::ObjectId>,
    /// Object IDs already present on the client.
    pub haves: Vec<gix_hash::ObjectId>,
    /// Requested refs through `want-ref`.
    pub want_refs: Vec<BString>,
    /// Shallow boundary commits sent by the client.
    pub shallow: Vec<gix_hash::ObjectId>,
    /// Optional depth requested by the client through `deepen <depth>`.
    pub deepen: Option<u32>,
    /// Optional depth timestamp requested by the client through `deepen-since <timestamp>`.
    pub deepen_since: Option<gix_date::SecondsSinceUnixEpoch>,
    /// Ref exclusions requested by the client through `deepen-not <ref>`.
    pub deepen_not: Vec<BString>,
    /// If true, client requests `deepen-relative`.
    pub deepen_relative: bool,
    /// Filter specifications requested by the client through `filter <spec>`.
    pub filters: Vec<BString>,
    /// Protocols requested by the client through `packfile-uris <protocols>`.
    pub packfile_uris: Vec<BString>,
    /// If true, client requests thin-pack behavior.
    pub thin_pack: bool,
    /// If true, client requests `no-progress`.
    pub no_progress: bool,
    /// If true, client requests `ofs-delta`.
    pub ofs_delta: bool,
    /// If true, client requests `include-tag`.
    pub include_tag: bool,
    /// If true, client requests `sideband-all`.
    pub sideband_all: bool,
    /// If true, client requests `wait-for-done`.
    pub wait_for_done: bool,
    /// If true, client completed negotiation with `done`.
    pub done: bool,
    /// Unknown arguments preserved for higher-level handling.
    pub extra_arguments: Vec<BString>,
}
/// The result of negotiating an upload-pack `fetch` request against repository data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchNegotiation {
    /// Acknowledgements to send in the `acknowledgments` section.
    pub acknowledgements: Vec<Acknowledgement>,
    /// Requested refs that could be resolved to object IDs and returned in `wanted-refs`.
    pub wanted_refs: Vec<WantedRef>,
    /// `want` object IDs that are present in the repository.
    pub known_wants: Vec<gix_hash::ObjectId>,
    /// `want` object IDs that are absent in the repository.
    pub missing_wants: Vec<gix_hash::ObjectId>,
    /// `have` object IDs that are present in the repository.
    pub common_haves: Vec<gix_hash::ObjectId>,
    /// `want-ref` names that could not be resolved to a reference.
    pub unresolved_want_refs: Vec<BString>,
}

impl FetchNegotiation {
    /// Convert this negotiation result into a [`FetchOutput`] without pack data.
    pub fn into_output(self) -> FetchOutput {
        let mut output = FetchOutput::without_pack();
        output.acknowledgements = self.acknowledgements;
        output.wanted_refs = self.wanted_refs;
        output
    }

    /// Convert this negotiation result into a [`FetchOutput`] and populate repository-backed pack data.
    ///
    /// Pack generation traverses all commits reachable from negotiated wants and from peeled `want-ref`
    /// targets while excluding commits reachable from acknowledged `have` lines.
    pub fn into_output_with_repository_pack<Find>(
        self,
        request: &Fetch,
        object_database: Find,
        object_hash: gix_hash::Kind,
    ) -> Result<FetchOutput, FetchPackGenerationError>
    where
        Find: gix_object::Find + gix_pack::Find + Clone,
    {
        let pack_data = generate_fetch_pack_data_with_repository(request, &self, object_database, object_hash)?;
        let mut output = self.into_output();
        output.pack_data = pack_data.map(|pack| Box::new(io::Cursor::new(pack)) as Box<dyn io::Read + Send + 'static>);
        Ok(output)
    }
}

/// Errors returned while negotiating `fetch` requests against repository state.
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum FetchNegotiationError {
    #[error(transparent)]
    OpenPackedRefs(#[from] gix_ref::packed::buffer::open::Error),
    #[error("Could not lookup wanted ref {ref_name:?}")]
    FindWantedRef {
        ref_name: BString,
        #[source]
        source: gix_ref::file::find::existing::Error,
    },
    #[error("Could not resolve wanted ref {ref_name:?} to an object id")]
    ResolveWantedRef {
        ref_name: BString,
        #[source]
        source: gix_ref::peel::to_object::Error,
    },
}

/// Errors returned while building repository-backed pack data for negotiated fetches.
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum FetchPackGenerationError {
    #[error(transparent)]
    FindObject(#[from] gix_object::find::Error),
    #[error("Object {id} disappeared while preparing pack data")]
    MissingObject { id: gix_hash::ObjectId },
    #[error(transparent)]
    DecodeTag(#[from] gix_object::decode::Error),
    #[error("Tag cycle detected at {id}")]
    TagCycle { id: gix_hash::ObjectId },
    #[error(transparent)]
    TraverseCommits(#[from] gix_traverse::commit::simple::Error),
    #[error(transparent)]
    CountObjects(#[from] gix_pack::data::output::count::objects::Error),
    #[error(transparent)]
    BuildPackEntry(#[from] gix_pack::data::output::entry::Error),
    #[error(transparent)]
    EncodePack(#[from] gix_pack::data::output::bytes::Error<gix_pack::data::output::entry::Error>),
    #[error("Cannot encode more than u32::MAX objects in a single pack, got {object_count}")]
    TooManyObjects { object_count: usize },
}

fn generate_fetch_pack_data_with_repository<Find>(
    _request: &Fetch,
    negotiation: &FetchNegotiation,
    object_database: Find,
    object_hash: gix_hash::Kind,
) -> Result<Option<Vec<u8>>, FetchPackGenerationError>
where
    Find: gix_object::Find + gix_pack::Find + Clone,
{
    let mut requested_ids = Vec::new();
    let mut seen_requested_ids = BTreeSet::new();
    for object_id in negotiation
        .known_wants
        .iter()
        .chain(negotiation.wanted_refs.iter().map(|wanted| &wanted.id))
    {
        if seen_requested_ids.insert(*object_id) {
            requested_ids.push(*object_id);
        }
    }
    if requested_ids.is_empty() {
        return Ok(None);
    }

    let mut object_buf = Vec::new();
    let wanted_commit_tips = collect_commit_tips(&requested_ids, &object_database, &mut object_buf)?;
    let hidden_commit_tips = collect_commit_tips(&negotiation.common_haves, &object_database, &mut object_buf)?;

    let mut objects_to_pack = Vec::new();
    if !wanted_commit_tips.is_empty() {
        let mut walk = gix_traverse::commit::Simple::new(wanted_commit_tips, object_database.clone());
        if !hidden_commit_tips.is_empty() {
            walk = walk.hide(hidden_commit_tips)?;
        }
        for commit in walk {
            objects_to_pack.push(commit?.id);
        }
    }
    objects_to_pack.extend(requested_ids);
    if objects_to_pack.is_empty() {
        return Ok(None);
    }

    let mut object_ids = objects_to_pack.into_iter().map(Ok::<_, BoxError>);
    let should_interrupt = AtomicBool::new(false);
    let (counts, _) = gix_pack::data::output::count::objects_unthreaded(
        &object_database,
        &mut object_ids,
        &gix_features::progress::Discard,
        &should_interrupt,
        gix_pack::data::output::count::objects::ObjectExpansion::TreeContents,
    )?;
    if counts.is_empty() {
        return Ok(None);
    }

    let object_count = counts.len();
    let num_entries =
        u32::try_from(object_count).map_err(|_| FetchPackGenerationError::TooManyObjects { object_count })?;
    let mut object_buf = Vec::new();
    let mut entries = Vec::with_capacity(object_count);
    for count in &counts {
        let object = gix_pack::Find::try_find(&object_database, count.id.as_ref(), &mut object_buf)?
            .ok_or_else(|| FetchPackGenerationError::MissingObject { id: count.id })?
            .0;
        entries.push(gix_pack::data::output::Entry::from_data(
            count,
            &object,
            gix_zlib::Compression::default(),
        )?);
    }
    let mut writer = gix_pack::data::output::bytes::FromEntriesIter::new(
        std::iter::once(Ok::<_, gix_pack::data::output::entry::Error>(entries)),
        Vec::new(),
        num_entries,
        gix_pack::data::Version::V2,
        object_hash,
    );
    for written in &mut writer {
        written?;
    }
    Ok(Some(writer.into_write()))
}

fn collect_commit_tips<Find>(
    object_ids: &[gix_hash::ObjectId],
    object_database: &Find,
    object_buf: &mut Vec<u8>,
) -> Result<Vec<gix_hash::ObjectId>, FetchPackGenerationError>
where
    Find: gix_object::Find,
{
    let mut tips = Vec::new();
    let mut seen_tips = BTreeSet::new();
    for object_id in object_ids {
        if let Some(commit_id) = peel_to_commit_tip(object_id, object_database, object_buf)? {
            if seen_tips.insert(commit_id) {
                tips.push(commit_id);
            }
        }
    }
    Ok(tips)
}

fn peel_to_commit_tip<Find>(
    object_id: &gix_hash::ObjectId,
    object_database: &Find,
    object_buf: &mut Vec<u8>,
) -> Result<Option<gix_hash::ObjectId>, FetchPackGenerationError>
where
    Find: gix_object::Find,
{
    let mut id = *object_id;
    let mut seen_tags = BTreeSet::new();
    loop {
        let object = object_database
            .try_find(id.as_ref(), object_buf)?
            .ok_or_else(|| FetchPackGenerationError::MissingObject { id })?;
        match object.kind {
            gix_object::Kind::Commit => return Ok(Some(id)),
            gix_object::Kind::Tag => {
                if !seen_tags.insert(id) {
                    return Err(FetchPackGenerationError::TagCycle { id });
                }
                id = gix_object::TagRefIter::from_bytes(object.data, object.object_hash).target_id()?;
            }
            gix_object::Kind::Tree | gix_object::Kind::Blob => return Ok(None),
        }
    }
}

/// Negotiate a `fetch` request using repository refs and object existence checks.
///
/// This resolves:
/// - `have` lines into `ACK` responses for object IDs known by the repository
/// - `want` lines into known/missing object sets
/// - `want-ref` lines into `wanted-refs` response entries when refs can be resolved
///
/// When `request.done` is true (client signals negotiation is complete), the acknowledgements
/// list ends with [`Acknowledgement::Ready`] if common haves exist, or is left empty for
/// fresh clones so that `write_fetch_response` omits the `acknowledgments` section entirely.
///
/// Pack construction is intentionally out of scope of this helper.
pub fn negotiate_fetch_with_repository(
    request: &Fetch,
    refs: &gix_ref::file::Store,
    mut object_exists: impl FnMut(&gix_hash::oid) -> bool,
) -> Result<FetchNegotiation, FetchNegotiationError> {
    let mut common_haves = Vec::new();
    let mut seen_haves = BTreeSet::new();
    for have in &request.haves {
        if object_exists(have) && seen_haves.insert(*have) {
            common_haves.push(*have);
        }
    }

    let mut known_wants = Vec::new();
    let mut missing_wants = Vec::new();
    let mut seen_known_wants = BTreeSet::new();
    let mut seen_missing_wants = BTreeSet::new();
    for want in &request.wants {
        if object_exists(want) {
            if seen_known_wants.insert(*want) {
                known_wants.push(*want);
            }
        } else if seen_missing_wants.insert(*want) {
            missing_wants.push(*want);
        }
    }

    let packed = refs.cached_packed_buffer()?;
    let packed = packed.as_ref().map(|buffer| &***buffer);

    let mut wanted_refs = Vec::new();
    let mut unresolved_want_refs = Vec::new();
    let mut seen_resolved_wants = BTreeSet::new();
    let mut seen_unresolved_wants = BTreeSet::new();
    for requested_ref in &request.want_refs {
        if seen_resolved_wants.contains(requested_ref) || seen_unresolved_wants.contains(requested_ref) {
            continue;
        }

        let partial_name: &gix_ref::PartialNameRef = match requested_ref.as_bstr().try_into() {
            Ok(name) => name,
            Err(_) => {
                if seen_unresolved_wants.insert(requested_ref.clone()) {
                    unresolved_want_refs.push(requested_ref.clone());
                }
                continue;
            }
        };

        match refs.find_packed(partial_name, packed) {
            Ok(mut reference) => {
                let id = reference.follow_to_object_packed(refs, packed).map_err(|source| {
                    FetchNegotiationError::ResolveWantedRef {
                        ref_name: requested_ref.clone(),
                        source,
                    }
                })?;
                if seen_resolved_wants.insert(requested_ref.clone()) {
                    wanted_refs.push(WantedRef {
                        id,
                        path: requested_ref.clone(),
                    });
                }
            }
            Err(gix_ref::file::find::existing::Error::NotFound { .. }) => {
                if seen_unresolved_wants.insert(requested_ref.clone()) {
                    unresolved_want_refs.push(requested_ref.clone());
                }
            }
            Err(source) => {
                return Err(FetchNegotiationError::FindWantedRef {
                    ref_name: requested_ref.clone(),
                    source,
                });
            }
        }
    }

    let acknowledgements = if request.done {
        if common_haves.is_empty() {
            Vec::new()
        } else {
            let mut acks: Vec<Acknowledgement> = common_haves
                .iter()
                .copied()
                .map(Acknowledgement::Common)
                .collect();
            acks.push(Acknowledgement::Ready);
            acks
        }
    } else if common_haves.is_empty() {
        vec![Acknowledgement::Nak]
    } else {
        common_haves.iter().copied().map(Acknowledgement::Common).collect()
    };

    Ok(FetchNegotiation {
        acknowledgements,
        wanted_refs,
        known_wants,
        missing_wants,
        common_haves,
        unresolved_want_refs,
    })
}

/// Output payload for a `fetch` response.
pub struct FetchOutput {
    /// Negotiation acknowledgements to return in the `acknowledgments` section.
    pub acknowledgements: Vec<Acknowledgement>,
    /// Optional shallow boundary updates to return in the `shallow-info` section.
    pub shallow_updates: Vec<ShallowUpdate>,
    /// Optional `wanted-refs` section entries.
    pub wanted_refs: Vec<WantedRef>,
    /// If present, pack data streamed as sideband channel 1 in the `packfile` section.
    pub pack_data: Option<Box<dyn io::Read + Send + 'static>>,
}

impl FetchOutput {
    /// Create a response output with `pack_data` and no additional sections.
    pub fn new(pack_data: impl io::Read + Send + 'static) -> Self {
        Self {
            acknowledgements: Vec::new(),
            shallow_updates: Vec::new(),
            wanted_refs: Vec::new(),
            pack_data: Some(Box::new(pack_data)),
        }
    }

    /// Create a response output without pack data.
    pub fn without_pack() -> Self {
        Self {
            acknowledgements: Vec::new(),
            shallow_updates: Vec::new(),
            wanted_refs: Vec::new(),
            pack_data: None,
        }
    }
}

/// The outcome of serving a single upload-pack protocol V2 command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// `ls-refs` output was produced.
    LsRefs {
        /// Number of refs sent to the client after applying filters.
        refs_sent: usize,
    },
    /// `fetch` output was produced.
    Fetch {
        /// Number of acknowledgement lines sent.
        acknowledgements_sent: usize,
        /// Number of shallow updates sent.
        shallow_updates_sent: usize,
        /// Number of wanted refs sent.
        wanted_refs_sent: usize,
        /// Number of raw pack bytes sent on sideband channel 1.
        pack_bytes_sent: u64,
    },
}

/// Delegate implementation used by [`serve_v2()`] to obtain repository data.
pub trait Delegate {
    /// Return refs to advertise for the incoming `ls-refs` request.
    fn ls_refs(&mut self, request: &LsRefs) -> Result<Vec<Ref>, BoxError>;
    /// Produce a fetch response for the incoming `fetch` request.
    ///
    /// [`negotiate_fetch_with_repository()`] can be used to obtain repository-backed
    /// acknowledgement and `wanted-refs` data before pack generation is applied.
    fn fetch(&mut self, request: &Fetch) -> Result<FetchOutput, BoxError>;
}

/// Errors returned by upload-pack request parsing and response writing.
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Decode(#[from] gix_transport::packetline::decode::Error),
    #[error("Expected text packetline, got {line_type}")]
    NonTextPacketLine { line_type: &'static str },
    #[error("Expected `command=<name>` in request header")]
    MissingCommand,
    #[error("Unsupported upload-pack V2 command {command:?}")]
    UnsupportedCommand { command: BString },
    #[error("Malformed request header line {line:?}")]
    MalformedHeaderLine { line: BString },
    #[error("Malformed {command} argument line {line:?}")]
    MalformedArgument { command: &'static str, line: BString },
    #[error("Could not parse object id in line {line:?}")]
    InvalidObjectId {
        line: BString,
        #[source]
        source: gix_hash::decode::Error,
    },
    #[error("Delegate failed")]
    Delegate(#[source] BoxError),
    #[error("Client requested object-format \"{requested}\" but server supports \"{supported}\"")]
    UnsupportedObjectFormat { requested: BString, supported: BString },
    #[error("Invalid object-format value \"{value}\" (expected \"sha1\" or \"sha256\")")]
    InvalidObjectFormat { value: BString },
    #[error("Object ID hex length {actual} does not match expected {expected} for {hash_kind}")]
    ObjectIdLengthMismatch {
        actual: usize,
        expected: usize,
        hash_kind: gix_hash::Kind,
    },
}

#[cfg(any(feature = "blocking-server", feature = "async-server"))]
/// Parse a single protocol V2 upload-pack request from `input`.
///
/// The `config` parameter controls capability validation — the client's `object-format`
/// feature (if present) is checked against `config.object_hash`, and OID hex lengths in
/// fetch arguments are enforced to match the configured hash kind.
pub fn parse_v2_request(input: impl io::Read, config: &ServerConfig) -> Result<Request, Error> {
    let mut reader = StreamingPeekableIter::new(input, HEADER_DELIMITERS, false);
    let header_lines = read_text_lines(&mut reader)?;
    let has_argument_section = reader.stopped_at() == Some(PacketLineRef::Delimiter);
    let argument_lines = if has_argument_section {
        reader.reset_with(ARGUMENT_DELIMITERS);
        read_text_lines(&mut reader)?
    } else {
        Vec::new()
    };

    let (command, features) = parse_header_lines(header_lines)?;
    validate_object_format(&features, config)?;
    let command: &[u8] = command.as_ref();
    let command = match command {
        b"ls-refs" => Command::LsRefs(parse_ls_refs_arguments(argument_lines)),
        b"fetch" => Command::Fetch(parse_fetch_arguments(argument_lines, config.object_hash)?),
        other => {
            return Err(Error::UnsupportedCommand { command: other.into() });
        }
    };
    Ok(Request { features, command })
}

/// Serve one protocol V2 upload-pack request end-to-end.
///
/// The caller owns transport setup/teardown and invokes this function with one complete request payload.
/// The `config` parameter controls capability validation — the client's `object-format` feature
/// (if present) is checked against `config.object_hash`, and OID hex lengths in fetch arguments
/// are enforced to match the configured hash kind.
#[cfg(any(feature = "blocking-server", feature = "async-server"))]
pub fn serve_v2(
    input: impl io::Read,
    mut output: impl io::Write,
    delegate: &mut impl Delegate,
    config: &ServerConfig,
) -> Result<Outcome, Error> {
    match parse_v2_request(input, config)? {
        Request {
            command: Command::LsRefs(request),
            ..
        } => {
            let refs = delegate.ls_refs(&request).map_err(Error::Delegate)?;
            let refs_sent = write_ls_refs_response(&mut output, &request, &refs)?;
            Ok(Outcome::LsRefs { refs_sent })
        }
        Request {
            command: Command::Fetch(request),
            ..
        } => {
            let mut response = delegate.fetch(&request).map_err(Error::Delegate)?;
            let pack_bytes_sent = write_fetch_response(&mut output, &mut response)?;
            Ok(Outcome::Fetch {
                acknowledgements_sent: response.acknowledgements.len(),
                shallow_updates_sent: response.shallow_updates.len(),
                wanted_refs_sent: response.wanted_refs.len(),
                pack_bytes_sent,
            })
        }
    }
}

/// Write a protocol V2 capability advertisement, including the `version 2` line.
#[cfg(any(feature = "blocking-server", feature = "async-server"))]
pub fn write_v2_capability_advertisement(mut output: impl io::Write, capabilities: &[Capability]) -> Result<(), Error> {
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

/// Write a `ls-refs` response body according to `request`.
///
/// Returns the number of refs written.
#[cfg(any(feature = "blocking-server", feature = "async-server"))]
pub fn write_ls_refs_response(mut output: impl io::Write, request: &LsRefs, refs: &[Ref]) -> Result<usize, Error> {
    let mut writer = Writer::new(&mut output);
    writer.enable_text_mode();
    let mut refs_sent = 0usize;

    for line in refs
        .iter()
        .filter(|reference| matches_ref_prefixes(reference, &request.ref_prefixes))
        .filter_map(|reference| format_ls_ref_line(reference, request))
    {
        writer.write_all(line.as_ref())?;
        refs_sent += 1;
    }
    encode::flush_to_write(writer.inner_mut())?;
    Ok(refs_sent)
}

/// Write the non-pack metadata sections (acknowledgments, shallow-info, wanted-refs) of a fetch response.
///
/// This is shared between the blocking [`write_fetch_response`] and the async variant
/// in [`async_io`] to avoid duplicating the section-framing logic.
///
/// When `has_pack_data` is true, each section is terminated with a delimiter packet (`0001`)
/// to signal that another section follows. When false, the last section omits the trailing
/// delimiter — the caller's final flush packet (`0000`) terminates the response instead.
/// This matches the V2 protocol framing expected by clients.
#[cfg(any(feature = "blocking-server", feature = "async-server"))]
pub(crate) fn write_fetch_metadata_sections(
    mut output: impl io::Write,
    acknowledgements: &[Acknowledgement],
    shallow_updates: &[ShallowUpdate],
    wanted_refs: &[WantedRef],
    has_pack_data: bool,
) -> Result<(), Error> {
    let mut writer = Writer::new(&mut output);
    writer.enable_text_mode();

    let sections: [(&[u8], bool); 3] = [
        (b"acknowledgments" as &[u8], !acknowledgements.is_empty()),
        (b"shallow-info", !shallow_updates.is_empty()),
        (b"wanted-refs", !wanted_refs.is_empty()),
    ];
    let last_active_idx = sections.iter().rposition(|(_, active)| *active);

    if !acknowledgements.is_empty() {
        writer.write_all(b"acknowledgments")?;
        for ack in acknowledgements {
            writer.write_all(format_acknowledgement_line(*ack).as_ref())?;
        }
        let is_last = last_active_idx == Some(0);
        if has_pack_data || !is_last {
            encode::delim_to_write(writer.inner_mut())?;
        }
    }

    if !shallow_updates.is_empty() {
        writer.write_all(b"shallow-info")?;
        for update in shallow_updates {
            writer.write_all(format_shallow_update_line(update).as_ref())?;
        }
        let is_last = last_active_idx == Some(1);
        if has_pack_data || !is_last {
            encode::delim_to_write(writer.inner_mut())?;
        }
    }

    if !wanted_refs.is_empty() {
        writer.write_all(b"wanted-refs")?;
        for wanted in wanted_refs {
            writer.write_all(format_wanted_ref_line(wanted).as_ref())?;
        }
        let is_last = last_active_idx == Some(2);
        if has_pack_data || !is_last {
            encode::delim_to_write(writer.inner_mut())?;
        }
    }

    Ok(())
}

/// Write a V2 `fetch` response, including optional sections and optional pack stream.
///
/// Returns the number of raw pack bytes sent on sideband channel `1`.
#[cfg(any(feature = "blocking-server", feature = "async-server"))]
pub fn write_fetch_response(mut output: impl io::Write, response: &mut FetchOutput) -> Result<u64, Error> {
    write_fetch_metadata_sections(
        &mut output,
        &response.acknowledgements,
        &response.shallow_updates,
        &response.wanted_refs,
        response.pack_data.is_some(),
    )?;

    let mut writer = Writer::new(&mut output);
    writer.enable_text_mode();

    let mut pack_bytes_sent = 0u64;
    if let Some(pack_data) = response.pack_data.as_mut() {
        writer.write_all(b"packfile")?;
        let mut buffer = [0u8; MAX_SIDEBAND_DATA_BYTES];
        loop {
            let bytes_read = pack_data.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            pack_bytes_sent += bytes_read as u64;
            encode::band_to_write(Channel::Data, &buffer[..bytes_read], writer.inner_mut())?;
        }
    }
    encode::flush_to_write(writer.inner_mut())?;
    Ok(pack_bytes_sent)
}

#[cfg(any(feature = "blocking-server", feature = "async-server"))]
fn parse_header_lines(lines: Vec<BString>) -> Result<(BString, Vec<Feature>), Error> {
    let mut command = None::<BString>;
    let mut features = Vec::new();

    for line in lines {
        let bytes: &[u8] = line.as_ref();
        if let Some(command_name) = bytes.strip_prefix(b"command=") {
            if command.is_some() || command_name.is_empty() {
                return Err(Error::MalformedHeaderLine { line });
            }
            command = Some(command_name.into());
            continue;
        }
        features.push(parse_feature_line(line.as_bstr())?);
    }

    let command = command.ok_or(Error::MissingCommand)?;
    Ok((command, features))
}

#[cfg(any(feature = "blocking-server", feature = "async-server"))]
fn parse_feature_line(line: &BStr) -> Result<Feature, Error> {
    if let Some((name, value)) = split_once(line, b'=') {
        if name.is_empty() {
            return Err(Error::MalformedHeaderLine { line: line.to_owned() });
        }
        return Ok(Feature {
            name: name.to_owned(),
            value: Some(value.to_owned()),
        });
    }
    if line.is_empty() {
        return Err(Error::MalformedHeaderLine { line: line.to_owned() });
    }
    Ok(Feature {
        name: line.to_owned(),
        value: None,
    })
}

#[cfg(any(feature = "blocking-server", feature = "async-server"))]
fn parse_ls_refs_arguments(arguments: Vec<BString>) -> LsRefs {
    let mut parsed = LsRefs::default();
    for line in arguments {
        let bytes: &[u8] = line.as_ref();
        match bytes {
            b"symrefs" => parsed.symrefs = true,
            b"peel" => parsed.peel = true,
            b"unborn" => parsed.unborn = true,
            _ => {
                if let Some(prefix) = bytes.strip_prefix(b"ref-prefix ") {
                    if !prefix.is_empty() {
                        parsed.ref_prefixes.push(prefix.into());
                    } else {
                        parsed.extra_arguments.push(line);
                    }
                } else {
                    parsed.extra_arguments.push(line);
                }
            }
        }
    }
    parsed
}

#[cfg(any(feature = "blocking-server", feature = "async-server"))]
fn parse_fetch_arguments(arguments: Vec<BString>, object_hash: gix_hash::Kind) -> Result<Fetch, Error> {
    let mut parsed = Fetch::default();
    for line in arguments {
        let bytes: &[u8] = line.as_ref();
        match bytes {
            b"thin-pack" => parsed.thin_pack = true,
            b"no-progress" => parsed.no_progress = true,
            b"ofs-delta" => parsed.ofs_delta = true,
            b"include-tag" => parsed.include_tag = true,
            b"sideband-all" => parsed.sideband_all = true,
            b"deepen-relative" => parsed.deepen_relative = true,
            b"wait-for-done" => parsed.wait_for_done = true,
            b"done" => parsed.done = true,
            _ => {
                if bytes.starts_with(b"want ") {
                    parsed
                        .wants
                        .push(parse_object_id(line.as_bstr(), b"want ", "fetch", object_hash)?);
                } else if bytes.starts_with(b"have ") {
                    parsed
                        .haves
                        .push(parse_object_id(line.as_bstr(), b"have ", "fetch", object_hash)?);
                } else if bytes.starts_with(b"shallow ") {
                    parsed
                        .shallow
                        .push(parse_object_id(line.as_bstr(), b"shallow ", "fetch", object_hash)?);
                } else if let Some(value) = bytes.strip_prefix(b"deepen ") {
                    parsed.deepen = Some(parse_u32_argument(line.as_bstr(), value, "fetch", false)?);
                } else if let Some(value) = bytes.strip_prefix(b"deepen-since ") {
                    parsed.deepen_since = Some(parse_i64_argument(line.as_bstr(), value, "fetch")?);
                } else if let Some(value) = bytes.strip_prefix(b"deepen-not ") {
                    if value.is_empty() {
                        return Err(Error::MalformedArgument { command: "fetch", line });
                    }
                    parsed.deepen_not.push(value.into());
                } else if let Some(value) = bytes.strip_prefix(b"filter ") {
                    if value.is_empty() {
                        return Err(Error::MalformedArgument { command: "fetch", line });
                    }
                    parsed.filters.push(value.into());
                } else if let Some(value) = bytes.strip_prefix(b"want-ref ") {
                    if value.is_empty() {
                        return Err(Error::MalformedArgument { command: "fetch", line });
                    }
                    parsed.want_refs.push(value.into());
                } else if let Some(value) = bytes.strip_prefix(b"packfile-uris ") {
                    parsed
                        .packfile_uris
                        .extend(parse_comma_separated_values(line.as_bstr(), value, "fetch")?);
                } else {
                    parsed.extra_arguments.push(line);
                }
            }
        }
    }
    Ok(parsed)
}

/// Known `object-format` values — recognized regardless of compile-time hash features.
/// This ensures a sha256 request against a sha1-only build reports "unsupported" not "invalid".
#[cfg(any(feature = "blocking-server", feature = "async-server"))]
const KNOWN_OBJECT_FORMATS: &[&str] = &["sha1", "sha256"];

#[cfg(any(feature = "blocking-server", feature = "async-server"))]
fn validate_object_format(features: &[Feature], config: &ServerConfig) -> Result<(), Error> {
    for feature in features {
        if feature.name == "object-format" {
            let value: &BStr = feature
                .value
                .as_ref()
                .map_or(b"".as_bstr(), |v| v.as_bstr());
            let value_str = match value.to_str() {
                Ok(s) => s,
                Err(_) => return Err(Error::InvalidObjectFormat { value: value.to_owned() }),
            };
            // Check if the value is a recognized hash name (independent of compile-time features)
            if !KNOWN_OBJECT_FORMATS.contains(&value_str) {
                return Err(Error::InvalidObjectFormat { value: value.to_owned() });
            }
            // Check if it matches the server's configured hash
            if value_str == config.object_hash.to_string().as_str() {
                return Ok(());
            }
            return Err(Error::UnsupportedObjectFormat {
                requested: value.to_owned(),
                supported: config.object_hash.to_string().into(),
            });
        }
    }
    // No object-format feature: assume server's hash — OK
    Ok(())
}

#[cfg(any(feature = "blocking-server", feature = "async-server"))]
fn parse_object_id(
    line: &BStr,
    prefix: &[u8],
    command: &'static str,
    object_hash: gix_hash::Kind,
) -> Result<gix_hash::ObjectId, Error> {
    let hex = line
        .as_bytes()
        .strip_prefix(prefix)
        .ok_or_else(|| Error::MalformedArgument {
            command,
            line: line.to_owned(),
        })?;

    let expected_len = object_hash.len_in_hex();
    if hex.len() != expected_len {
        return Err(Error::ObjectIdLengthMismatch {
            actual: hex.len(),
            expected: expected_len,
            hash_kind: object_hash,
        });
    }

    gix_hash::ObjectId::from_hex(hex).map_err(|source| Error::InvalidObjectId {
        line: line.to_owned(),
        source,
    })
}

#[cfg(any(feature = "blocking-server", feature = "async-server"))]
fn parse_u32_argument(line: &BStr, value: &[u8], command: &'static str, allow_zero: bool) -> Result<u32, Error> {
    std::str::from_utf8(value)
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|parsed| allow_zero || *parsed != 0)
        .ok_or_else(|| Error::MalformedArgument {
            command,
            line: line.to_owned(),
        })
}

#[cfg(any(feature = "blocking-server", feature = "async-server"))]
fn parse_i64_argument(line: &BStr, value: &[u8], command: &'static str) -> Result<i64, Error> {
    std::str::from_utf8(value)
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .ok_or_else(|| Error::MalformedArgument {
            command,
            line: line.to_owned(),
        })
}

#[cfg(any(feature = "blocking-server", feature = "async-server"))]
fn parse_comma_separated_values(line: &BStr, value: &[u8], command: &'static str) -> Result<Vec<BString>, Error> {
    if value.is_empty() {
        return Err(Error::MalformedArgument {
            command,
            line: line.to_owned(),
        });
    }

    let values = value
        .split(|byte| *byte == b',')
        .map(|value| value.as_bstr().to_owned())
        .collect::<Vec<_>>();
    if values.iter().any(|value| value.is_empty()) {
        return Err(Error::MalformedArgument {
            command,
            line: line.to_owned(),
        });
    }
    Ok(values)
}

#[cfg(any(feature = "blocking-server", feature = "async-server"))]
fn read_text_lines<T: io::Read>(reader: &mut StreamingPeekableIter<T>) -> Result<Vec<BString>, Error> {
    let mut out = Vec::new();
    while let Some(line) = reader.read_line() {
        let line = line?;
        let line = line?;
        let text = line.as_text().ok_or_else(|| Error::NonTextPacketLine {
            line_type: packet_line_kind(&line),
        })?;
        out.push(text.as_bstr().to_owned());
    }
    Ok(out)
}

#[cfg(any(feature = "blocking-server", feature = "async-server"))]
fn packet_line_kind(line: &PacketLineRef<'_>) -> &'static str {
    match line {
        PacketLineRef::Data(_) => "data",
        PacketLineRef::Flush => "flush",
        PacketLineRef::Delimiter => "delimiter",
        PacketLineRef::ResponseEnd => "response-end",
    }
}

#[cfg(any(feature = "blocking-server", feature = "async-server"))]
fn split_once(line: &BStr, separator: u8) -> Option<(&BStr, &BStr)> {
    let idx = line.find_byte(separator)?;
    Some((line[..idx].as_bstr(), line[idx + 1..].as_bstr()))
}

#[cfg(any(feature = "blocking-server", feature = "async-server"))]
fn matches_ref_prefixes(reference: &Ref, prefixes: &[BString]) -> bool {
    if prefixes.is_empty() {
        return true;
    }
    let full_ref_name = match reference {
        Ref::Peeled { full_ref_name, .. }
        | Ref::Direct { full_ref_name, .. }
        | Ref::Symbolic { full_ref_name, .. }
        | Ref::Unborn { full_ref_name, .. } => full_ref_name,
    };
    prefixes.iter().any(|prefix| {
        let full_ref_name: &[u8] = full_ref_name.as_ref();
        let prefix: &[u8] = prefix.as_ref();
        full_ref_name.starts_with(prefix)
    })
}

#[cfg(any(feature = "blocking-server", feature = "async-server"))]
fn format_ls_ref_line(reference: &Ref, request: &LsRefs) -> Option<BString> {
    let mut line = BString::default();
    match reference {
        Ref::Direct { full_ref_name, object } => {
            line.push_str(object.to_string());
            line.push_byte(b' ');
            line.push_str(full_ref_name);
        }
        Ref::Peeled {
            full_ref_name,
            tag,
            object,
        } => {
            line.push_str(tag.to_string());
            line.push_byte(b' ');
            line.push_str(full_ref_name);
            if request.peel {
                line.push_str(" peeled:");
                line.push_str(object.to_string());
            }
        }
        Ref::Symbolic {
            full_ref_name,
            target,
            tag,
            object,
        } => {
            let advertised_id = tag.as_ref().unwrap_or(object);
            line.push_str(advertised_id.to_string());
            line.push_byte(b' ');
            line.push_str(full_ref_name);

            if request.symrefs {
                line.push_str(" symref-target:");
                line.push_str(target);
            }
            if request.peel {
                if let Some(tag) = tag {
                    line.push_str(" peeled:");
                    line.push_str(object.to_string());
                    if tag == object {
                        return Some(line);
                    }
                }
            }
        }
        Ref::Unborn { full_ref_name, target } => {
            if !request.unborn {
                return None;
            }
            line.push_str("unborn ");
            line.push_str(full_ref_name);
            line.push_str(" symref-target:");
            line.push_str(target);
        }
    }
    Some(line)
}

#[cfg(any(feature = "blocking-server", feature = "async-server"))]
fn format_acknowledgement_line(ack: Acknowledgement) -> BString {
    match ack {
        Acknowledgement::Common(id) => format!("ACK {id} common").into(),
        Acknowledgement::Ready => "ready".into(),
        Acknowledgement::Nak => "NAK".into(),
    }
}

#[cfg(any(feature = "blocking-server", feature = "async-server"))]
fn format_shallow_update_line(update: &ShallowUpdate) -> BString {
    match update {
        ShallowUpdate::Shallow(id) => format!("shallow {id}").into(),
        ShallowUpdate::Unshallow(id) => format!("unshallow {id}").into(),
    }
}

#[cfg(any(feature = "blocking-server", feature = "async-server"))]
fn format_wanted_ref_line(wanted: &WantedRef) -> BString {
    let mut line = BString::default();
    line.push_str(wanted.id.to_string());
    line.push_byte(b' ');
    let path: &[u8] = wanted.path.as_ref();
    line.push_str(path);
    line
}

#[cfg(test)]
mod tests {
    use gix_object::Write as _;
    use std::{
        collections::BTreeSet,
        fs,
        io::Write as _,
        io::{BufReader, Cursor, Read as _},
        path::PathBuf,
        sync::atomic::{AtomicBool, AtomicU64, Ordering},
    };

    use gix_transport::packetline::{BandRef, PacketLineRef, blocking_io::StreamingPeekableIter};

    use super::*;

    #[derive(Default)]
    struct MockDelegate {
        refs: Vec<Ref>,
        fetch_output: Option<FetchOutput>,
        seen_ls_refs: Option<LsRefs>,
        seen_fetch: Option<Fetch>,
    }

    impl Delegate for MockDelegate {
        fn ls_refs(&mut self, request: &LsRefs) -> Result<Vec<Ref>, BoxError> {
            self.seen_ls_refs = Some(request.clone());
            Ok(self.refs.clone())
        }

        fn fetch(&mut self, request: &Fetch) -> Result<FetchOutput, BoxError> {
            self.seen_fetch = Some(request.clone());
            self.fetch_output
                .take()
                .ok_or_else(|| std::io::Error::other("fetch output should be configured").into())
        }
    }

    #[test]
    fn parse_ls_refs_request() -> Result<(), Box<dyn std::error::Error>> {
        let input = request_bytes(
            "ls-refs",
            &["agent=git/gitplane", "object-format=sha1"],
            &["symrefs", "peel", "ref-prefix refs/heads/"],
        )?;

        let request = parse_v2_request(input.as_slice(), &ServerConfig::default())?;
        assert_eq!(
            request.features,
            vec![
                Feature {
                    name: "agent".into(),
                    value: Some("git/gitplane".into()),
                },
                Feature {
                    name: "object-format".into(),
                    value: Some("sha1".into()),
                },
            ]
        );
        match request.command {
            Command::LsRefs(arguments) => {
                assert!(arguments.symrefs);
                assert!(arguments.peel);
                assert_eq!(arguments.ref_prefixes, vec![BString::from("refs/heads/")]);
            }
            Command::Fetch(_) => panic!("expected ls-refs command"),
        }
        Ok(())
    }

    #[test]
    fn parse_fetch_request() -> Result<(), Box<dyn std::error::Error>> {
        let id_one = "808e50d724f604f69ab93c6da2919c014667bedb";
        let id_two = "9e320b9180e0b5580af68fa3255b7f3d9ecd5af0";
        let input = request_bytes(
            "fetch",
            &["agent=git/gitplane"],
            &[
                "thin-pack",
                "ofs-delta",
                &format!("want {id_one}"),
                &format!("have {id_two}"),
                "want-ref refs/heads/main",
                "done",
            ],
        )?;

        let request = parse_v2_request(input.as_slice(), &ServerConfig::default())?;
        match request.command {
            Command::Fetch(arguments) => {
                assert!(arguments.thin_pack);
                assert!(arguments.ofs_delta);
                assert!(arguments.done);
                assert_eq!(arguments.wants, vec![gix_hash::ObjectId::from_hex(id_one.as_bytes())?]);
                assert_eq!(arguments.haves, vec![gix_hash::ObjectId::from_hex(id_two.as_bytes())?]);
                assert_eq!(arguments.want_refs, vec![BString::from("refs/heads/main")]);
            }
            Command::LsRefs(_) => panic!("expected fetch command"),
        }
        Ok(())
    }

    #[test]
    fn parse_fetch_request_with_negotiation_arguments() -> Result<(), Box<dyn std::error::Error>> {
        let id = "808e50d724f604f69ab93c6da2919c014667bedb";
        let input = request_bytes(
            "fetch",
            &[],
            &[
                "no-progress",
                "deepen 16",
                "deepen-since 12345",
                "deepen-not refs/tags/v1.0.0",
                "deepen-relative",
                "filter blob:none",
                "packfile-uris https,ssh",
                "wait-for-done",
                &format!("want {id}"),
                "done",
            ],
        )?;

        let request = parse_v2_request(input.as_slice(), &ServerConfig::default())?;
        match request.command {
            Command::Fetch(arguments) => {
                assert!(arguments.no_progress);
                assert_eq!(arguments.deepen, Some(16));
                assert_eq!(arguments.deepen_since, Some(12_345));
                assert_eq!(arguments.deepen_not, vec![BString::from("refs/tags/v1.0.0")]);
                assert!(arguments.deepen_relative);
                assert_eq!(arguments.filters, vec![BString::from("blob:none")]);
                assert_eq!(
                    arguments.packfile_uris,
                    vec![BString::from("https"), BString::from("ssh")]
                );
                assert!(arguments.wait_for_done);
                assert!(arguments.done);
            }
            Command::LsRefs(_) => panic!("expected fetch command"),
        }
        Ok(())
    }

    #[test]
    fn parse_fetch_request_with_invalid_deepen_value() -> Result<(), Box<dyn std::error::Error>> {
        let input = request_bytes("fetch", &[], &["deepen nope"])?;
        let err = parse_v2_request(input.as_slice(), &ServerConfig::default())
            .expect_err("invalid deepen value should fail parsing");
        assert!(
            matches!(err, Error::MalformedArgument { command: "fetch", line } if line.as_bstr() == "deepen nope".as_bytes().as_bstr())
        );
        Ok(())
    }

    /// Informational features like `agent` pass through without validation,
    /// and even `object-format` is preserved in the parsed features list.
    #[test]
    fn feature_pass_through() -> Result<(), Box<dyn std::error::Error>> {
        let input = request_bytes(
            "ls-refs",
            &["agent=git/test", "object-format=sha1"],
            &[],
        )?;

        let request = parse_v2_request(input.as_slice(), &ServerConfig::default())?;
        assert_eq!(
            request.features,
            vec![
                Feature {
                    name: "agent".into(),
                    value: Some("git/test".into()),
                },
                Feature {
                    name: "object-format".into(),
                    value: Some("sha1".into()),
                },
            ],
            "both agent and object-format features should be present in parsed result"
        );
        Ok(())
    }

    #[test]
    fn negotiate_fetch_with_repository_tracks_wants_and_common_haves() -> Result<(), Box<dyn std::error::Error>> {
        let (_tmp, refs) = temporary_ref_store(&Vec::<(&str, String)>::new())?;
        let known_want = object_id("808e50d724f604f69ab93c6da2919c014667bedb");
        let missing_want = object_id("9e320b9180e0b5580af68fa3255b7f3d9ecd5af0");
        let common_have = object_id("f99771fe6a1b535783af3163eba95a927aae21d5");
        let unknown_have = object_id("2d9d136fb0765f2e24c44a0f91984318d580d03b");

        let request = Fetch {
            wants: vec![known_want.clone(), missing_want.clone(), known_want.clone()],
            haves: vec![common_have.clone(), unknown_have, common_have.clone()],
            ..Default::default()
        };
        let known_objects = [known_want.clone(), common_have.clone()]
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();

        let negotiation = negotiate_fetch_with_repository(&request, &refs, |id| known_objects.contains(id))?;

        assert_eq!(negotiation.known_wants, vec![known_want]);
        assert_eq!(negotiation.missing_wants, vec![missing_want]);
        assert_eq!(negotiation.common_haves, vec![common_have]);
        assert_eq!(negotiation.acknowledgements, vec![Acknowledgement::Common(common_have)]);
        assert!(negotiation.wanted_refs.is_empty());
        assert!(negotiation.unresolved_want_refs.is_empty());

        let output = negotiation.into_output();
        assert_eq!(output.acknowledgements, vec![Acknowledgement::Common(common_have)]);
        assert!(output.wanted_refs.is_empty());
        assert!(output.pack_data.is_none());
        Ok(())
    }

    #[test]
    fn negotiate_fetch_with_repository_sends_nak_without_common_haves() -> Result<(), Box<dyn std::error::Error>> {
        let (_tmp, refs) = temporary_ref_store(&Vec::<(&str, String)>::new())?;
        let request = Fetch {
            haves: vec![object_id("f99771fe6a1b535783af3163eba95a927aae21d5")],
            ..Default::default()
        };

        let negotiation = negotiate_fetch_with_repository(&request, &refs, |_| false)?;
        assert_eq!(negotiation.acknowledgements, vec![Acknowledgement::Nak]);
        Ok(())
    }

    #[test]
    fn negotiate_fetch_with_repository_resolves_want_refs() -> Result<(), Box<dyn std::error::Error>> {
        let main = object_id("808e50d724f604f69ab93c6da2919c014667bedb");
        let (_tmp, refs) = temporary_ref_store(&[
            ("HEAD", "ref: refs/heads/main\n".to_string()),
            ("refs/heads/main", format!("{main}\n")),
        ])?;

        let request = Fetch {
            want_refs: vec![
                "HEAD".into(),
                "refs/heads/main".into(),
                "HEAD".into(),
                "refs/heads/missing".into(),
                "not a ref".into(),
            ],
            ..Default::default()
        };

        let negotiation = negotiate_fetch_with_repository(&request, &refs, |_| false)?;
        assert_eq!(
            negotiation.wanted_refs,
            vec![
                WantedRef {
                    id: main.clone(),
                    path: "HEAD".into(),
                },
                WantedRef {
                    id: main,
                    path: "refs/heads/main".into(),
                },
            ]
        );
        assert_eq!(
            negotiation.unresolved_want_refs,
            vec![BString::from("refs/heads/missing"), BString::from("not a ref")]
        );
        Ok(())
    }

    #[test]
    fn into_output_with_repository_pack_omits_pack_without_wants() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = temporary_object_store_with_linear_history()?;
        let (_tmp, refs) = temporary_ref_store(&Vec::<(&str, String)>::new())?;
        let request = Fetch::default();

        let negotiation =
            negotiate_fetch_with_repository(&request, &refs, |id| gix_pack::Find::contains(&fixture.odb, id))?;
        let output =
            negotiation.into_output_with_repository_pack(&request, fixture.odb.clone(), gix_hash::Kind::Sha1)?;

        assert_eq!(output.acknowledgements, vec![Acknowledgement::Nak]);
        assert!(output.pack_data.is_none(), "no wants should not produce a pack");
        Ok(())
    }

    #[test]
    fn into_output_with_repository_pack_excludes_common_have_history() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = temporary_object_store_with_linear_history()?;
        let (_tmp, refs) = temporary_ref_store(&Vec::<(&str, String)>::new())?;
        let request = Fetch {
            wants: vec![fixture.commit_three.clone()],
            haves: vec![fixture.commit_one.clone()],
            ..Default::default()
        };

        let negotiation =
            negotiate_fetch_with_repository(&request, &refs, |id| gix_pack::Find::contains(&fixture.odb, id))?;
        let mut output =
            negotiation.into_output_with_repository_pack(&request, fixture.odb.clone(), gix_hash::Kind::Sha1)?;
        let mut pack_bytes = Vec::new();
        output
            .pack_data
            .as_mut()
            .expect("known wants should produce pack data")
            .read_to_end(&mut pack_bytes)?;

        let packed_ids = pack_object_ids(pack_bytes, gix_hash::Kind::Sha1)?;
        assert!(packed_ids.contains(&fixture.commit_three));
        assert!(packed_ids.contains(&fixture.commit_two));
        assert!(
            !packed_ids.contains(&fixture.commit_one),
            "commits acknowledged as common should not be resent"
        );
        Ok(())
    }

    #[test]
    fn into_output_with_repository_pack_peels_tag_wants_to_commits() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = temporary_object_store_with_linear_history()?;
        let (_tmp, refs) = temporary_ref_store(&Vec::<(&str, String)>::new())?;
        let request = Fetch {
            wants: vec![fixture.tag_three.clone()],
            haves: vec![fixture.commit_one.clone()],
            ..Default::default()
        };

        let negotiation =
            negotiate_fetch_with_repository(&request, &refs, |id| gix_pack::Find::contains(&fixture.odb, id))?;
        let mut output =
            negotiation.into_output_with_repository_pack(&request, fixture.odb.clone(), gix_hash::Kind::Sha1)?;
        let mut pack_bytes = Vec::new();
        output
            .pack_data
            .as_mut()
            .expect("tag wants should produce pack data")
            .read_to_end(&mut pack_bytes)?;

        let packed_ids = pack_object_ids(pack_bytes, gix_hash::Kind::Sha1)?;
        assert!(packed_ids.contains(&fixture.tag_three));
        assert!(packed_ids.contains(&fixture.commit_three));
        assert!(packed_ids.contains(&fixture.commit_two));
        assert!(
            !packed_ids.contains(&fixture.commit_one),
            "common history should stay excluded even for tag wants"
        );
        Ok(())
    }

    #[test]
    fn serve_ls_refs_with_prefix_filter() -> Result<(), Box<dyn std::error::Error>> {
        let request = request_bytes(
            "ls-refs",
            &["agent=git/gitplane"],
            &["symrefs", "peel", "ref-prefix refs/heads/"],
        )?;
        let mut output = Vec::new();
        let mut delegate = MockDelegate {
            refs: vec![
                Ref::Symbolic {
                    full_ref_name: "HEAD".into(),
                    target: "refs/heads/main".into(),
                    tag: None,
                    object: gix_hash::ObjectId::from_hex(b"808e50d724f604f69ab93c6da2919c014667bedb")?,
                },
                Ref::Direct {
                    full_ref_name: "refs/heads/main".into(),
                    object: gix_hash::ObjectId::from_hex(b"808e50d724f604f69ab93c6da2919c014667bedb")?,
                },
                Ref::Direct {
                    full_ref_name: "refs/tags/v1.0.0".into(),
                    object: gix_hash::ObjectId::from_hex(b"9e320b9180e0b5580af68fa3255b7f3d9ecd5af0")?,
                },
            ],
            ..Default::default()
        };

        let outcome = serve_v2(request.as_slice(), &mut output, &mut delegate, &ServerConfig::default())?;
        assert_eq!(outcome, Outcome::LsRefs { refs_sent: 1 });
        assert!(
            delegate
                .seen_ls_refs
                .as_ref()
                .expect("request should be captured")
                .symrefs
        );

        let mut reader = StreamingPeekableIter::new(output.as_slice(), &[PacketLineRef::Flush], false);
        let advertised = next_text_line(&mut reader)?;
        assert_eq!(
            advertised.as_bstr(),
            "808e50d724f604f69ab93c6da2919c014667bedb refs/heads/main"
                .as_bytes()
                .as_bstr()
        );
        assert!(reader.read_line().is_none(), "flush should terminate response");
        assert_eq!(reader.stopped_at(), Some(PacketLineRef::Flush));
        Ok(())
    }

    #[test]
    fn serve_fetch_with_pack_sideband() -> Result<(), Box<dyn std::error::Error>> {
        let common_id = gix_hash::ObjectId::from_hex(b"808e50d724f604f69ab93c6da2919c014667bedb")?;
        let wanted_id = gix_hash::ObjectId::from_hex(b"9e320b9180e0b5580af68fa3255b7f3d9ecd5af0")?;
        let request = request_bytes(
            "fetch",
            &["agent=git/gitplane"],
            &[&format!("want {common_id}"), "done"],
        )?;
        let mut output = Vec::new();
        let mut fetch_output = FetchOutput::new(Cursor::new(b"PACK\0\0\0\0".to_vec()));
        fetch_output.acknowledgements.push(Acknowledgement::Common(common_id));
        fetch_output.wanted_refs.push(WantedRef {
            id: wanted_id,
            path: "refs/heads/main".into(),
        });
        let mut delegate = MockDelegate {
            fetch_output: Some(fetch_output),
            ..Default::default()
        };

        let outcome = serve_v2(request.as_slice(), &mut output, &mut delegate, &ServerConfig::default())?;
        assert_eq!(
            outcome,
            Outcome::Fetch {
                acknowledgements_sent: 1,
                shallow_updates_sent: 0,
                wanted_refs_sent: 1,
                pack_bytes_sent: 8,
            }
        );
        assert!(delegate.seen_fetch.as_ref().expect("request should be captured").done);

        let mut reader = StreamingPeekableIter::new(output.as_slice(), &[PacketLineRef::Flush], false);
        assert_eq!(
            next_text_line(&mut reader)?.as_bstr(),
            "acknowledgments".as_bytes().as_bstr()
        );
        assert_eq!(
            next_text_line(&mut reader)?.as_bstr(),
            format!("ACK {common_id} common").as_bytes().as_bstr()
        );
        expect_delimiter(&mut reader)?;
        assert_eq!(
            next_text_line(&mut reader)?.as_bstr(),
            "wanted-refs".as_bytes().as_bstr()
        );
        assert_eq!(
            next_text_line(&mut reader)?.as_bstr(),
            format!("{wanted_id} refs/heads/main").as_bytes().as_bstr()
        );
        expect_delimiter(&mut reader)?;
        assert_eq!(next_text_line(&mut reader)?.as_bstr(), "packfile".as_bytes().as_bstr());
        assert_eq!(next_band_data(&mut reader)?, b"PACK\0\0\0\0");
        assert!(reader.read_line().is_none(), "flush should terminate response");
        assert_eq!(reader.stopped_at(), Some(PacketLineRef::Flush));
        Ok(())
    }

    fn next_text_line(reader: &mut StreamingPeekableIter<&[u8]>) -> Result<BString, Box<dyn std::error::Error>> {
        let line = reader
            .read_line()
            .expect("expected packetline")
            .expect("read should succeed")
            .expect("decode should succeed");
        Ok(line.as_text().expect("expected text packetline").as_bstr().to_owned())
    }

    fn expect_delimiter(reader: &mut StreamingPeekableIter<&[u8]>) -> Result<(), Box<dyn std::error::Error>> {
        let line = reader
            .read_line()
            .expect("expected packetline")
            .expect("read should succeed")
            .expect("decode should succeed");
        match line {
            PacketLineRef::Delimiter => Ok(()),
            other => Err(format!("expected delimiter, got {other:?}").into()),
        }
    }

    fn next_band_data(reader: &mut StreamingPeekableIter<&[u8]>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let line = reader
            .read_line()
            .expect("expected packetline")
            .expect("read should succeed")
            .expect("decode should succeed");
        match line.decode_band()? {
            BandRef::Data(data) => Ok(data.to_vec()),
            other => Err(format!("expected data band, got {other:?}").into()),
        }
    }

    fn request_bytes(
        command: &str,
        features: &[&str],
        arguments: &[&str],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut out = Vec::new();
        let mut writer = Writer::new(&mut out);
        writer.enable_text_mode();
        writer.write_all(format!("command={command}").as_bytes())?;
        for feature in features {
            writer.write_all(feature.as_bytes())?;
        }
        if arguments.is_empty() {
            encode::flush_to_write(writer.inner_mut())?;
            return Ok(out);
        }

        encode::delim_to_write(writer.inner_mut())?;
        for argument in arguments {
            writer.write_all(argument.as_bytes())?;
        }
        encode::flush_to_write(writer.inner_mut())?;
        Ok(out)
    }

    struct ObjectStoreFixture {
        _temp: TempDir,
        odb: gix_odb::Handle,
        commit_one: gix_hash::ObjectId,
        commit_two: gix_hash::ObjectId,
        commit_three: gix_hash::ObjectId,
        tag_three: gix_hash::ObjectId,
    }

    fn temporary_object_store_with_linear_history() -> Result<ObjectStoreFixture, Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        let objects_path = temp.path.join("objects");
        fs::create_dir_all(&objects_path)?;
        let odb = gix_odb::at(objects_path)?;

        let blob_one = odb
            .write_buf(gix_object::Kind::Blob, b"one\n")
            .map_err(std::io::Error::other)?;
        let tree_one = write_single_file_tree(&odb, "file.txt", &blob_one)?;
        let commit_one = write_commit_object(&odb, &tree_one, None, "commit one")?;
        let blob_two = odb
            .write_buf(gix_object::Kind::Blob, b"two\n")
            .map_err(std::io::Error::other)?;
        let tree_two = write_single_file_tree(&odb, "file.txt", &blob_two)?;
        let commit_two = write_commit_object(&odb, &tree_two, Some(&commit_one), "commit two")?;
        let blob_three = odb
            .write_buf(gix_object::Kind::Blob, b"three\n")
            .map_err(std::io::Error::other)?;
        let tree_three = write_single_file_tree(&odb, "file.txt", &blob_three)?;
        let commit_three = write_commit_object(&odb, &tree_three, Some(&commit_two), "commit three")?;
        let tag_three = write_tag_object(&odb, &commit_three, "v1.0.0", "release tag")?;

        Ok(ObjectStoreFixture {
            _temp: temp,
            odb,
            commit_one,
            commit_two,
            commit_three,
            tag_three,
        })
    }

    fn write_single_file_tree(
        odb: &gix_odb::Handle,
        filename: &str,
        blob_id: &gix_hash::ObjectId,
    ) -> Result<gix_hash::ObjectId, std::io::Error> {
        let tree = gix_object::Tree {
            entries: vec![gix_object::tree::Entry {
                mode: gix_object::tree::EntryKind::Blob.into(),
                filename: BString::from(filename),
                oid: blob_id.clone(),
            }],
        };
        odb.write(&tree).map_err(std::io::Error::other)
    }

    fn write_commit_object(
        odb: &gix_odb::Handle,
        tree_id: &gix_hash::ObjectId,
        parent: Option<&gix_hash::ObjectId>,
        message: &str,
    ) -> Result<gix_hash::ObjectId, std::io::Error> {
        let mut bytes = format!("tree {tree_id}\n").into_bytes();
        if let Some(parent) = parent {
            bytes.extend_from_slice(format!("parent {parent}\n").as_bytes());
        }
        bytes.extend_from_slice(b"author Example <example@example.com> 0 +0000\n");
        bytes.extend_from_slice(b"committer Example <example@example.com> 0 +0000\n\n");
        bytes.extend_from_slice(message.as_bytes());
        bytes.push(b'\n');
        odb.write_buf(gix_object::Kind::Commit, &bytes)
            .map_err(std::io::Error::other)
    }

    fn write_tag_object(
        odb: &gix_odb::Handle,
        target: &gix_hash::ObjectId,
        name: &str,
        message: &str,
    ) -> Result<gix_hash::ObjectId, std::io::Error> {
        let mut bytes = format!("object {target}\n").into_bytes();
        bytes.extend_from_slice(b"type commit\n");
        bytes.extend_from_slice(format!("tag {name}\n").as_bytes());
        bytes.extend_from_slice(b"tagger Example <example@example.com> 0 +0000\n\n");
        bytes.extend_from_slice(message.as_bytes());
        bytes.push(b'\n');
        odb.write_buf(gix_object::Kind::Tag, &bytes)
            .map_err(std::io::Error::other)
    }

    fn pack_object_ids(
        pack_data: Vec<u8>,
        object_hash: gix_hash::Kind,
    ) -> Result<BTreeSet<gix_hash::ObjectId>, Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        let mut reader = BufReader::new(Cursor::new(pack_data));
        let outcome = gix_pack::Bundle::write_to_directory(
            &mut reader,
            Some(temp.path.as_path()),
            &mut gix_features::progress::Discard,
            &AtomicBool::new(false),
            None::<gix_odb::Handle>,
            gix_pack::bundle::write::Options {
                object_hash,
                ..Default::default()
            },
        )?;
        let bundle = outcome
            .to_bundle()
            .ok_or_else(|| std::io::Error::other("a bundle path should be available"))??;
        Ok(bundle.index.iter().map(|entry| entry.oid).collect())
    }

    fn object_id(hex: &str) -> gix_hash::ObjectId {
        gix_hash::ObjectId::from_hex(hex.as_bytes()).expect("valid object id in test")
    }

    // Bug condition exploration tests: these encode the EXPECTED behavior per protocol v2.
    // They are expected to FAIL on unfixed code, confirming the bug exists (done flag is ignored).
    // **Validates: Requirements 1.1, 1.2, 2.1, 2.2**

    #[test]
    fn negotiate_fetch_done_with_common_haves_should_end_with_ready() -> Result<(), Box<dyn std::error::Error>> {
        let (_tmp, refs) = temporary_ref_store(&Vec::<(&str, String)>::new())?;
        let known_have = object_id("808e50d724f604f69ab93c6da2919c014667bedb");

        let request = Fetch {
            haves: vec![known_have.clone()],
            done: true,
            ..Default::default()
        };
        let known_objects = [known_have.clone()].into_iter().collect::<BTreeSet<_>>();

        let negotiation = negotiate_fetch_with_repository(&request, &refs, |id| known_objects.contains(id))?;

        assert_eq!(
            negotiation.acknowledgements,
            vec![Acknowledgement::Common(known_have), Acknowledgement::Ready],
            "when done=true and common haves exist, acknowledgements must end with Ready to signal packfile follows"
        );
        Ok(())
    }

    #[test]
    fn negotiate_fetch_done_with_no_haves_should_produce_empty_acknowledgements() -> Result<(), Box<dyn std::error::Error>> {
        let (_tmp, refs) = temporary_ref_store(&Vec::<(&str, String)>::new())?;

        let request = Fetch {
            done: true,
            ..Default::default()
        };

        let negotiation = negotiate_fetch_with_repository(&request, &refs, |_| false)?;

        assert!(
            negotiation.acknowledgements.is_empty(),
            "when done=true and no haves exist (fresh clone), acknowledgements must be empty so the section is omitted"
        );
        Ok(())
    }

    #[test]
    fn negotiate_fetch_done_with_all_unknown_haves_should_produce_empty_acknowledgements() -> Result<(), Box<dyn std::error::Error>> {
        let (_tmp, refs) = temporary_ref_store(&Vec::<(&str, String)>::new())?;
        let unknown_have = object_id("f99771fe6a1b535783af3163eba95a927aae21d5");

        let request = Fetch {
            haves: vec![unknown_have],
            done: true,
            ..Default::default()
        };

        let negotiation = negotiate_fetch_with_repository(&request, &refs, |_| false)?;

        assert!(
            negotiation.acknowledgements.is_empty(),
            "when done=true and no haves are known (all unknown), acknowledgements must be empty so the section is omitted"
        );
        Ok(())
    }

    #[test]
    fn negotiate_fetch_done_with_mixed_known_unknown_haves_should_have_common_then_ready() -> Result<(), Box<dyn std::error::Error>> {
        let (_tmp, refs) = temporary_ref_store(&Vec::<(&str, String)>::new())?;
        let known_have = object_id("808e50d724f604f69ab93c6da2919c014667bedb");
        let unknown_have = object_id("9e320b9180e0b5580af68fa3255b7f3d9ecd5af0");

        let request = Fetch {
            haves: vec![known_have.clone(), unknown_have],
            done: true,
            ..Default::default()
        };
        let known_objects = [known_have.clone()].into_iter().collect::<BTreeSet<_>>();

        let negotiation = negotiate_fetch_with_repository(&request, &refs, |id| known_objects.contains(id))?;

        assert_eq!(
            negotiation.acknowledgements,
            vec![Acknowledgement::Common(known_have), Acknowledgement::Ready],
            "when done=true with mix of known/unknown haves, acknowledgements must be [Common(known), Ready]"
        );
        Ok(())
    }

    // Preservation property tests: verify that `done == false` behavior is unchanged.
    // These tests must PASS on unfixed code, confirming baseline behavior to preserve.
    // **Validates: Requirements 3.1, 3.2**

    #[test]
    fn preservation_done_false_single_known_have_produces_common_only() -> Result<(), Box<dyn std::error::Error>> {
        let (_tmp, refs) = temporary_ref_store(&Vec::<(&str, String)>::new())?;
        let known_have = object_id("808e50d724f604f69ab93c6da2919c014667bedb");

        let request = Fetch {
            haves: vec![known_have.clone()],
            done: false,
            ..Default::default()
        };
        let known_objects = [known_have.clone()].into_iter().collect::<BTreeSet<_>>();

        let negotiation = negotiate_fetch_with_repository(&request, &refs, |id| known_objects.contains(id))?;

        assert_eq!(
            negotiation.acknowledgements,
            vec![Acknowledgement::Common(known_have)],
            "when done=false with a known have, acknowledgements must be [Common(id)] without Ready"
        );
        assert!(
            !negotiation.acknowledgements.contains(&Acknowledgement::Ready),
            "done=false must never produce Ready"
        );
        assert!(
            !negotiation.acknowledgements.is_empty(),
            "done=false with known haves must never produce empty acknowledgements"
        );
        Ok(())
    }

    #[test]
    fn preservation_done_false_multiple_known_haves_produces_common_for_each() -> Result<(), Box<dyn std::error::Error>> {
        let (_tmp, refs) = temporary_ref_store(&Vec::<(&str, String)>::new())?;
        let have_a = object_id("808e50d724f604f69ab93c6da2919c014667bedb");
        let have_b = object_id("9e320b9180e0b5580af68fa3255b7f3d9ecd5af0");
        let have_c = object_id("f99771fe6a1b535783af3163eba95a927aae21d5");

        let request = Fetch {
            haves: vec![have_a.clone(), have_b.clone(), have_c.clone()],
            done: false,
            ..Default::default()
        };
        let known_objects = [have_a.clone(), have_b.clone(), have_c.clone()]
            .into_iter()
            .collect::<BTreeSet<_>>();

        let negotiation = negotiate_fetch_with_repository(&request, &refs, |id| known_objects.contains(id))?;

        assert_eq!(
            negotiation.acknowledgements,
            vec![
                Acknowledgement::Common(have_a),
                Acknowledgement::Common(have_b),
                Acknowledgement::Common(have_c),
            ],
            "when done=false with multiple known haves, acknowledgements must be [Common(a), Common(b), Common(c)]"
        );
        assert!(
            !negotiation.acknowledgements.contains(&Acknowledgement::Ready),
            "done=false must never produce Ready even with multiple known haves"
        );
        Ok(())
    }

    #[test]
    fn preservation_done_false_mixed_known_unknown_haves_produces_common_for_known_only() -> Result<(), Box<dyn std::error::Error>> {
        let (_tmp, refs) = temporary_ref_store(&Vec::<(&str, String)>::new())?;
        let known_have = object_id("808e50d724f604f69ab93c6da2919c014667bedb");
        let unknown_have = object_id("9e320b9180e0b5580af68fa3255b7f3d9ecd5af0");

        let request = Fetch {
            haves: vec![known_have.clone(), unknown_have],
            done: false,
            ..Default::default()
        };
        let known_objects = [known_have.clone()].into_iter().collect::<BTreeSet<_>>();

        let negotiation = negotiate_fetch_with_repository(&request, &refs, |id| known_objects.contains(id))?;

        assert_eq!(
            negotiation.acknowledgements,
            vec![Acknowledgement::Common(known_have)],
            "when done=false with mixed haves, only known haves appear as Common entries"
        );
        assert!(
            !negotiation.acknowledgements.contains(&Acknowledgement::Ready),
            "done=false must never produce Ready"
        );
        Ok(())
    }

    #[test]
    fn preservation_done_false_no_haves_produces_nak() -> Result<(), Box<dyn std::error::Error>> {
        let (_tmp, refs) = temporary_ref_store(&Vec::<(&str, String)>::new())?;

        let request = Fetch {
            done: false,
            ..Default::default()
        };

        let negotiation = negotiate_fetch_with_repository(&request, &refs, |_| false)?;

        assert_eq!(
            negotiation.acknowledgements,
            vec![Acknowledgement::Nak],
            "when done=false and no haves exist, acknowledgements must be [Nak]"
        );
        Ok(())
    }

    #[test]
    fn preservation_done_false_all_unknown_haves_produces_nak() -> Result<(), Box<dyn std::error::Error>> {
        let (_tmp, refs) = temporary_ref_store(&Vec::<(&str, String)>::new())?;
        let unknown_a = object_id("808e50d724f604f69ab93c6da2919c014667bedb");
        let unknown_b = object_id("9e320b9180e0b5580af68fa3255b7f3d9ecd5af0");

        let request = Fetch {
            haves: vec![unknown_a, unknown_b],
            done: false,
            ..Default::default()
        };

        let negotiation = negotiate_fetch_with_repository(&request, &refs, |_| false)?;

        assert_eq!(
            negotiation.acknowledgements,
            vec![Acknowledgement::Nak],
            "when done=false and all haves are unknown, acknowledgements must be [Nak]"
        );
        Ok(())
    }

    #[test]
    fn preservation_done_false_duplicate_known_haves_are_deduplicated() -> Result<(), Box<dyn std::error::Error>> {
        let (_tmp, refs) = temporary_ref_store(&Vec::<(&str, String)>::new())?;
        let known_have = object_id("808e50d724f604f69ab93c6da2919c014667bedb");

        let request = Fetch {
            haves: vec![known_have.clone(), known_have.clone(), known_have.clone()],
            done: false,
            ..Default::default()
        };
        let known_objects = [known_have.clone()].into_iter().collect::<BTreeSet<_>>();

        let negotiation = negotiate_fetch_with_repository(&request, &refs, |id| known_objects.contains(id))?;

        assert_eq!(
            negotiation.acknowledgements,
            vec![Acknowledgement::Common(known_have)],
            "duplicate haves must be deduplicated in acknowledgements"
        );
        Ok(())
    }

    #[test]
    fn preservation_non_acknowledgement_fields_unaffected_by_done_flag() -> Result<(), Box<dyn std::error::Error>> {
        let main_id = object_id("808e50d724f604f69ab93c6da2919c014667bedb");
        let (_tmp, refs) = temporary_ref_store(&[
            ("HEAD", "ref: refs/heads/main\n".to_string()),
            ("refs/heads/main", format!("{main_id}\n")),
        ])?;
        let known_want = object_id("808e50d724f604f69ab93c6da2919c014667bedb");
        let missing_want = object_id("2d9d136fb0765f2e24c44a0f91984318d580d03b");
        let common_have = object_id("f99771fe6a1b535783af3163eba95a927aae21d5");
        let unknown_have = object_id("9e320b9180e0b5580af68fa3255b7f3d9ecd5af0");

        let known_objects = [known_want.clone(), common_have.clone()]
            .into_iter()
            .collect::<BTreeSet<_>>();

        // Request with done=false
        let request_not_done = Fetch {
            wants: vec![known_want.clone(), missing_want.clone()],
            haves: vec![common_have.clone(), unknown_have.clone()],
            want_refs: vec!["HEAD".into(), "refs/heads/missing".into()],
            done: false,
            ..Default::default()
        };

        // Request with done=true (same inputs except done flag)
        let request_done = Fetch {
            wants: vec![known_want.clone(), missing_want.clone()],
            haves: vec![common_have.clone(), unknown_have.clone()],
            want_refs: vec!["HEAD".into(), "refs/heads/missing".into()],
            done: true,
            ..Default::default()
        };

        let negotiation_not_done =
            negotiate_fetch_with_repository(&request_not_done, &refs, |id| known_objects.contains(id))?;
        let negotiation_done =
            negotiate_fetch_with_repository(&request_done, &refs, |id| known_objects.contains(id))?;

        assert_eq!(
            negotiation_not_done.known_wants, negotiation_done.known_wants,
            "known_wants must be unaffected by done flag"
        );
        assert_eq!(
            negotiation_not_done.missing_wants, negotiation_done.missing_wants,
            "missing_wants must be unaffected by done flag"
        );
        assert_eq!(
            negotiation_not_done.common_haves, negotiation_done.common_haves,
            "common_haves must be unaffected by done flag"
        );
        assert_eq!(
            negotiation_not_done.wanted_refs, negotiation_done.wanted_refs,
            "wanted_refs must be unaffected by done flag"
        );
        assert_eq!(
            negotiation_not_done.unresolved_want_refs, negotiation_done.unresolved_want_refs,
            "unresolved_want_refs must be unaffected by done flag"
        );
        Ok(())
    }

    // ServerConfig and validate_object_format tests
    // Requirements: 1.2, 2.1, 2.2, 2.3, 2.4, 3.1

    #[test]
    fn server_config_default_returns_sha1() {
        let config = ServerConfig::default();
        assert_eq!(
            config.object_hash,
            gix_hash::Kind::Sha1,
            "ServerConfig::default() should configure SHA-1 as the object hash"
        );
    }

    #[test]
    fn validate_object_format_matching_sha1_accepted() -> Result<(), Box<dyn std::error::Error>> {
        let input = request_bytes("ls-refs", &["object-format=sha1"], &[])?;
        let config = ServerConfig {
            object_hash: gix_hash::Kind::Sha1,
        };
        let request = parse_v2_request(input.as_slice(), &config)?;
        assert_eq!(
            request.features[0].name.as_bytes(),
            b"object-format",
            "object-format feature should be parsed"
        );
        Ok(())
    }

    #[test]
    fn validate_object_format_mismatched_sha256_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let input = request_bytes("ls-refs", &["object-format=sha256"], &[])?;
        let config = ServerConfig {
            object_hash: gix_hash::Kind::Sha1,
        };
        let err = parse_v2_request(input.as_slice(), &config)
            .expect_err("sha256 against sha1 config should be rejected");
        match err {
            Error::UnsupportedObjectFormat { requested, supported } => {
                assert_eq!(requested.as_bytes(), b"sha256", "requested format should be sha256");
                assert_eq!(supported.as_bytes(), b"sha1", "supported format should be sha1");
            }
            other => panic!("expected UnsupportedObjectFormat, got: {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn validate_object_format_invalid_blake3_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let input = request_bytes("ls-refs", &["object-format=blake3"], &[])?;
        let config = ServerConfig {
            object_hash: gix_hash::Kind::Sha1,
        };
        let err = parse_v2_request(input.as_slice(), &config)
            .expect_err("unrecognized hash name should be rejected");
        match err {
            Error::InvalidObjectFormat { value } => {
                assert_eq!(value.as_bytes(), b"blake3", "invalid value should be blake3");
            }
            other => panic!("expected InvalidObjectFormat, got: {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn validate_object_format_absent_feature_accepted() -> Result<(), Box<dyn std::error::Error>> {
        let input = request_bytes("ls-refs", &["agent=git/test"], &[])?;
        let config = ServerConfig {
            object_hash: gix_hash::Kind::Sha1,
        };
        let request = parse_v2_request(input.as_slice(), &config)?;
        assert_eq!(
            request.features.len(),
            1,
            "only agent feature should be present"
        );
        assert_eq!(
            request.features[0].name.as_bytes(),
            b"agent",
            "absent object-format should not cause rejection"
        );
        Ok(())
    }

    #[test]
    fn validate_object_format_empty_value_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let input = request_bytes("ls-refs", &["object-format="], &[])?;
        let config = ServerConfig::default();
        let err = parse_v2_request(input.as_slice(), &config)
            .expect_err("empty object-format value should be rejected");
        match err {
            Error::InvalidObjectFormat { value } => {
                assert_eq!(value.as_bytes(), b"", "invalid value should be empty");
            }
            other => panic!("expected InvalidObjectFormat, got: {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn validate_object_format_non_utf8_value_rejected() -> Result<(), Box<dyn std::error::Error>> {
        // Construct a request with a non-UTF-8 object-format value by manually
        // building the packet-line bytes. The value \xff\xfe is invalid UTF-8.
        let mut out = Vec::new();
        let mut writer = Writer::new(&mut out);
        writer.enable_text_mode();
        writer.write_all(b"command=ls-refs")?;
        // Write a feature line with non-UTF-8 value
        writer.write_all(b"object-format=\xff\xfe")?;
        encode::flush_to_write(writer.inner_mut())?;

        let config = ServerConfig::default();
        let err = parse_v2_request(out.as_slice(), &config)
            .expect_err("non-UTF-8 object-format value should be rejected");
        match err {
            Error::InvalidObjectFormat { value } => {
                assert_eq!(value.as_bytes(), b"\xff\xfe", "invalid value should preserve the raw bytes");
            }
            other => panic!("expected InvalidObjectFormat, got: {other:?}"),
        }
        Ok(())
    }

    // OID length enforcement tests
    // Requirements: 4.1, 4.2

    #[test]
    fn oid_length_sha1_config_rejects_64_char_hex() -> Result<(), Box<dyn std::error::Error>> {
        let sha256_oid = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let input = request_bytes("fetch", &[], &[&format!("want {sha256_oid}"), "done"])?;
        let config = ServerConfig {
            object_hash: gix_hash::Kind::Sha1,
        };

        let err = parse_v2_request(input.as_slice(), &config)
            .expect_err("SHA-1 config should reject 64-char hex OID");
        assert!(
            matches!(
                err,
                Error::ObjectIdLengthMismatch {
                    actual: 64,
                    expected: 40,
                    hash_kind: gix_hash::Kind::Sha1,
                }
            ),
            "expected ObjectIdLengthMismatch with actual=64, expected=40, got: {err:?}"
        );
        Ok(())
    }

    #[cfg(feature = "sha256")]
    #[test]
    fn oid_length_sha256_config_rejects_40_char_hex() -> Result<(), Box<dyn std::error::Error>> {
        let sha1_oid = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let input = request_bytes("fetch", &[], &[&format!("want {sha1_oid}"), "done"])?;
        let config = ServerConfig {
            object_hash: gix_hash::Kind::Sha256,
        };

        let err = parse_v2_request(input.as_slice(), &config)
            .expect_err("SHA-256 config should reject 40-char hex OID");
        assert!(
            matches!(
                err,
                Error::ObjectIdLengthMismatch {
                    actual: 40,
                    expected: 64,
                    hash_kind: gix_hash::Kind::Sha256,
                }
            ),
            "expected ObjectIdLengthMismatch with actual=40, expected=64, got: {err:?}"
        );
        Ok(())
    }

    #[test]
    fn oid_length_sha1_config_accepts_40_char_valid_hex() -> Result<(), Box<dyn std::error::Error>> {
        let sha1_oid = "808e50d724f604f69ab93c6da2919c014667bedb";
        let input = request_bytes("fetch", &[], &[&format!("want {sha1_oid}"), "done"])?;
        let config = ServerConfig {
            object_hash: gix_hash::Kind::Sha1,
        };

        let request = parse_v2_request(input.as_slice(), &config)
            .expect("SHA-1 config should accept 40-char hex OID");
        match request.command {
            Command::Fetch(fetch) => {
                assert_eq!(
                    fetch.wants,
                    vec![gix_hash::ObjectId::from_hex(sha1_oid.as_bytes())?],
                    "parsed want should match the provided SHA-1 OID"
                );
            }
            Command::LsRefs(_) => panic!("expected fetch command"),
        }
        Ok(())
    }

    #[cfg(feature = "sha256")]
    #[test]
    fn oid_length_sha256_config_accepts_64_char_valid_hex() -> Result<(), Box<dyn std::error::Error>> {
        let sha256_oid = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let input = request_bytes("fetch", &[], &[&format!("want {sha256_oid}"), "done"])?;
        let config = ServerConfig {
            object_hash: gix_hash::Kind::Sha256,
        };

        let request = parse_v2_request(input.as_slice(), &config)
            .expect("SHA-256 config should accept 64-char hex OID");
        match request.command {
            Command::Fetch(fetch) => {
                assert_eq!(
                    fetch.wants,
                    vec![gix_hash::ObjectId::from_hex(sha256_oid.as_bytes())?],
                    "parsed want should match the provided SHA-256 OID"
                );
            }
            Command::LsRefs(_) => panic!("expected fetch command"),
        }
        Ok(())
    }

    // Property-based tests for object-format validation
    // Feature: upload-pack-capability-validation, Property 1: Object-format validation accepts matching, rejects mismatched or invalid
    // **Validates: Requirements 2.1, 2.3, 3.1**
    #[cfg(feature = "blocking-server")]
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]
            #[test]
            fn property_object_format_validation_sha1(
                value in ".*",
            ) {
                let kind = gix_hash::Kind::Sha1;
                let config = ServerConfig { object_hash: kind };
                let features = vec![Feature {
                    name: "object-format".into(),
                    value: Some(value.clone().into()),
                }];
                let result = validate_object_format(&features, &config);

                let known_formats = ["sha1", "sha256"];
                if !known_formats.contains(&value.as_str()) {
                    // Unrecognized → InvalidObjectFormat
                    prop_assert!(
                        matches!(result, Err(Error::InvalidObjectFormat { .. })),
                        "unrecognized value {:?} should produce InvalidObjectFormat, got: {:?}",
                        value,
                        result
                    );
                } else if value == kind.to_string() {
                    // Matching → Ok
                    prop_assert!(
                        result.is_ok(),
                        "matching value {:?} for kind {:?} should succeed, got: {:?}",
                        value,
                        kind,
                        result
                    );
                } else {
                    // Recognized but mismatched → UnsupportedObjectFormat
                    prop_assert!(
                        matches!(result, Err(Error::UnsupportedObjectFormat { .. })),
                        "mismatched value {:?} for kind {:?} should produce UnsupportedObjectFormat, got: {:?}",
                        value,
                        kind,
                        result
                    );
                }
            }
        }

        #[cfg(feature = "sha256")]
        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]
            #[test]
            fn property_object_format_validation_sha256(
                value in ".*",
            ) {
                let kind = gix_hash::Kind::Sha256;
                let config = ServerConfig { object_hash: kind };
                let features = vec![Feature {
                    name: "object-format".into(),
                    value: Some(value.clone().into()),
                }];
                let result = validate_object_format(&features, &config);

                let known_formats = ["sha1", "sha256"];
                if !known_formats.contains(&value.as_str()) {
                    // Unrecognized → InvalidObjectFormat
                    prop_assert!(
                        matches!(result, Err(Error::InvalidObjectFormat { .. })),
                        "unrecognized value {:?} should produce InvalidObjectFormat, got: {:?}",
                        value,
                        result
                    );
                } else if value == kind.to_string() {
                    // Matching → Ok
                    prop_assert!(
                        result.is_ok(),
                        "matching value {:?} for kind {:?} should succeed, got: {:?}",
                        value,
                        kind,
                        result
                    );
                } else {
                    // Recognized but mismatched → UnsupportedObjectFormat
                    prop_assert!(
                        matches!(result, Err(Error::UnsupportedObjectFormat { .. })),
                        "mismatched value {:?} for kind {:?} should produce UnsupportedObjectFormat, got: {:?}",
                        value,
                        kind,
                        result
                    );
                }
            }
        }
    }

    // Property-based tests for upload-pack capability validation
    // Feature: upload-pack-capability-validation

    /// **Validates: Requirements 5.1, 5.2, 5.3**
    /// Property 3: Non-object-format features pass through without rejection
    #[cfg(feature = "blocking-server")]
    mod property_non_object_format_tests {
        use super::*;
        use proptest::prelude::*;

        fn arbitrary_server_config() -> gix_hash::Kind {
            // Use SHA-1 as the default; SHA-256 tested when feature is available
            gix_hash::Kind::Sha1
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]
            #[test]
            fn property_non_object_format_feature_pass_through(
                name in "[a-z][a-z0-9-]{0,30}".prop_filter("must not be object-format", |s| s != "object-format"),
                has_value in any::<bool>(),
                value in ".*",
            ) {
                let config = ServerConfig { object_hash: arbitrary_server_config() };
                let feature_value = if has_value { Some(BString::from(value.as_str())) } else { None };
                let features = vec![Feature { name: name.clone().into(), value: feature_value }];

                // Non-object-format features should never cause validation to fail
                let result = validate_object_format(&features, &config);
                prop_assert!(result.is_ok(), "non-object-format feature '{}' should not cause validation error, got: {:?}", name, result);
            }
        }

        #[cfg(feature = "sha256")]
        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]
            #[test]
            fn property_non_object_format_feature_pass_through_sha256(
                name in "[a-z][a-z0-9-]{0,30}".prop_filter("must not be object-format", |s| s != "object-format"),
                has_value in any::<bool>(),
                value in ".*",
            ) {
                let config = ServerConfig { object_hash: gix_hash::Kind::Sha256 };
                let feature_value = if has_value { Some(BString::from(value.as_str())) } else { None };
                let features = vec![Feature { name: name.clone().into(), value: feature_value }];

                // Non-object-format features should never cause validation to fail with SHA-256 config either
                let result = validate_object_format(&features, &config);
                prop_assert!(result.is_ok(), "non-object-format feature '{}' with sha256 config should not cause validation error, got: {:?}", name, result);
            }
        }
    }

    fn temporary_ref_store(
        files: &[(&str, String)],
    ) -> Result<(TempDir, gix_ref::file::Store), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        for (relative_path, content) in files {
            let path = temp.path.join(relative_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, content)?;
        }
        let store = gix_ref::file::Store::at(
            temp.path.clone(),
            gix_ref::store::init::Options {
                write_reflog: gix_ref::store::WriteReflog::Disable,
                object_hash: gix_hash::Kind::Sha1,
                ..Default::default()
            },
        );
        Ok((temp, store))
    }

    struct TempDir {
        path: PathBuf,
    }
    static TEMP_DIR_ID: AtomicU64 = AtomicU64::new(0);

    impl TempDir {
        fn new() -> Result<Self, std::io::Error> {
            let base = std::env::temp_dir();
            for _ in 0..16 {
                let unique = TEMP_DIR_ID.fetch_add(1, Ordering::Relaxed);
                let path = base.join(format!("gitoxide-upload-pack-test-{}-{unique}", std::process::id()));
                match fs::create_dir(&path) {
                    Ok(()) => return Ok(Self { path }),
                    Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(err) => return Err(err),
                }
            }

            Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "could not allocate unique temporary upload-pack test directory",
            ))
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            if let Err(_err) = fs::remove_dir_all(&self.path) {
                // Best-effort cleanup for tests.
            }
        }
    }

    /// Feature: upload-pack-capability-validation, Property 2: OID length enforcement
    ///
    /// For any `gix_hash::Kind` configured on the server, and for any hex string in a `want`
    /// argument line, `parse_object_id` SHALL succeed if and only if the hex string length equals
    /// `kind.len_in_hex()` AND the string contains only valid hex characters. If the length does
    /// not match, the parser SHALL return `Error::ObjectIdLengthMismatch`.
    ///
    /// **Validates: Requirements 4.1, 4.2, 7.4**
    #[cfg(feature = "blocking-server")]
    mod property_tests_oid_length {
        use super::*;
        use proptest::prelude::*;

        fn arb_hash_kind() -> impl Strategy<Value = gix_hash::Kind> {
            #[cfg(feature = "sha256")]
            {
                prop_oneof![
                    Just(gix_hash::Kind::Sha1),
                    Just(gix_hash::Kind::Sha256),
                ]
                .boxed()
            }
            #[cfg(not(feature = "sha256"))]
            {
                Just(gix_hash::Kind::Sha1).boxed()
            }
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]
            #[test]
            fn property_oid_length_enforcement(
                kind in arb_hash_kind(),
                hex_chars in prop::collection::vec(prop::char::range('\0', '\x7f'), 0..128usize),
            ) {
                let hex_string: String = hex_chars.into_iter().collect();
                let line = format!("want {hex_string}");
                let result = parse_object_id(line.as_bytes().as_bstr(), b"want ", "fetch", kind);

                let expected_len = kind.len_in_hex();
                if hex_string.len() != expected_len {
                    prop_assert!(
                        matches!(result, Err(Error::ObjectIdLengthMismatch { .. })),
                        "expected ObjectIdLengthMismatch for len {} != expected {}, got: {:?}",
                        hex_string.len(),
                        expected_len,
                        result,
                    );
                } else if hex_string.bytes().all(|b| b.is_ascii_hexdigit()) {
                    prop_assert!(
                        result.is_ok(),
                        "expected Ok for valid hex of correct length {}, got: {:?}",
                        expected_len,
                        result,
                    );
                } else {
                    prop_assert!(
                        matches!(result, Err(Error::InvalidObjectId { .. })),
                        "expected InvalidObjectId for invalid hex chars at correct length {}, got: {:?}",
                        expected_len,
                        result,
                    );
                }
            }
        }
    }
}

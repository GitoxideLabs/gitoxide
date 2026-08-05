//! An in-process upload-pack transport that drives `gix-protocol`'s `serve_v2()` directly,
//! avoiding the need to spawn an external `git-upload-pack` process for `file://` URLs.
//!
//! This module is gated behind `#[cfg(feature = "experimental")]`.

use std::{
    any::Any,
    borrow::Cow,
    io,
    sync::{Arc, Mutex},
};

use crate::bstr::{BStr, BString, ByteSlice, ByteVec};
use gix_pack::Find as _;
use gix_ref::file::ReferenceExt as _;
use gix_transport::{
    Protocol, Service,
    client::{
        self, Capabilities, MessageKind, WriteMode,
        blocking_io::{ExtendedBufRead, HandleProgress, ReadlineBufRead, RequestWriter, SetServiceResponse},
    },
    packetline::PacketLineRef,
};

use crate::protocol::upload_pack::{
    Capability, Delegate, Fetch, FetchOutput, LsRefs, ServerConfig,
    negotiate_fetch_with_repository, serve_v2,
};
use crate::protocol::handshake::Ref;

type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

// ---------------------------------------------------------------------------
// BuiltinUploadPack transport
// ---------------------------------------------------------------------------

/// State captured after a successful handshake with the target repository.
struct HandshakeState {
    ref_store: gix_ref::file::Store,
    odb: gix_odb::Handle,
    #[allow(dead_code)]
    capabilities: Capabilities,
    object_hash: gix_hash::Kind,
}

/// An in-process transport that serves upload-pack protocol V2 by directly opening
/// the target repository's ref store and object database.
///
/// This avoids spawning `git-upload-pack` for `file://` URLs when the `experimental`
/// feature is active and the runtime option requests it.
pub struct BuiltinUploadPack {
    /// Path to the target repository's git directory.
    path: BString,
    /// The protocol version to advertise.
    #[allow(dead_code)]
    desired_version: Protocol,
    /// Whether to trace packet lines.
    #[allow(dead_code)]
    trace: bool,
    /// State populated after `handshake()` is called.
    state: Option<HandshakeState>,
}

impl BuiltinUploadPack {
    /// Create a new in-process upload-pack transport targeting the repository at `path`.
    pub fn new(path: impl Into<BString>, version: Protocol, trace: bool) -> Self {
        Self {
            path: path.into(),
            desired_version: version,
            trace,
            state: None,
        }
    }

    /// Open the target repository's ref store and ODB.
    fn open_repository(&self) -> Result<(gix_ref::file::Store, gix_odb::Handle, gix_hash::Kind), client::Error> {
        let git_dir = gix_path::from_bstr(self.path.as_bstr());
        let git_dir = git_dir.as_ref();

        // Determine if this is a bare repo or has a .git subdir
        let actual_git_dir = if git_dir.join("objects").is_dir() && git_dir.join("refs").is_dir() {
            git_dir.to_owned()
        } else if git_dir.join(".git").is_dir() {
            git_dir.join(".git")
        } else {
            git_dir.to_owned()
        };

        let object_hash = gix_hash::Kind::Sha1;
        let ref_store = gix_ref::file::Store::at(
            actual_git_dir.clone().into(),
            gix_ref::store::init::Options {
                write_reflog: gix_ref::store::WriteReflog::Disable,
                object_hash,
                ..Default::default()
            },
        );

        let objects_dir = actual_git_dir.join("objects");
        let odb = gix_odb::at(objects_dir).map_err(|err| {
            client::Error::Io(io::Error::new(
                io::ErrorKind::Other,
                format!("failed to open object database at '{}': {err}", self.path),
            ))
        })?;

        Ok((ref_store, odb, object_hash))
    }

    /// Build the V2 capability advertisement lines and return them as a `Capabilities` value.
    fn build_capabilities(_object_hash: gix_hash::Kind) -> Capabilities {
        // Build a capabilities buffer that Capabilities::from_lines can parse.
        let mut buf = BString::from("version 2\n");
        buf.push_str(b"ls-refs\n");
        buf.push_str(b"fetch=shallow wait-for-done\n");
        buf.push_str(b"object-format=sha1\n");
        buf.push_str(b"agent=gix-builtin-upload-pack/0.1\n");

        Capabilities::from_lines(buf).expect("statically valid capability advertisement")
    }

    /// Build capability advertisement lines for `write_v2_capability_advertisement`.
    #[allow(dead_code, reason = "Prepared for capability advertisement in handshake; wired in a follow-up.")]
    fn advertisement_capabilities(_object_hash: gix_hash::Kind) -> Vec<Capability> {
        vec![
            Capability {
                name: "ls-refs".into(),
                values: vec![],
            },
            Capability {
                name: "fetch".into(),
                values: vec!["shallow".into(), "wait-for-done".into()],
            },
            Capability {
                name: "object-format".into(),
                values: vec!["sha1".into()],
            },
            Capability {
                name: "agent".into(),
                values: vec!["gix-builtin-upload-pack/0.1".into()],
            },
        ]
    }
}

impl client::TransportWithoutIO for BuiltinUploadPack {
    fn to_url(&self) -> Cow<'_, BStr> {
        let mut url = BString::from("file://");
        url.push_str(&self.path);
        Cow::Owned(url)
    }

    fn connection_persists_across_multiple_requests(&self) -> bool {
        true
    }

    fn configure(&mut self, _config: &dyn Any) -> Result<(), BoxError> {
        Ok(())
    }
}

impl client::blocking_io::Transport for BuiltinUploadPack {
    fn handshake<'a>(
        &mut self,
        _service: Service,
        _extra_parameters: &'a [(&'a str, Option<&'a str>)],
    ) -> Result<SetServiceResponse<'_>, client::Error> {
        let (ref_store, odb, object_hash) = self.open_repository()?;
        let capabilities = Self::build_capabilities(object_hash);

        self.state = Some(HandshakeState {
            ref_store,
            odb,
            capabilities: capabilities.clone(),
            object_hash,
        });

        Ok(SetServiceResponse {
            actual_protocol: Protocol::V2,
            capabilities,
            refs: None,
        })
    }

    fn request(
        &mut self,
        write_mode: WriteMode,
        on_into_read: MessageKind,
        trace: bool,
    ) -> Result<RequestWriter<'_>, client::Error> {
        let state = self.state.as_ref().ok_or(client::Error::MissingHandshake)?;

        // Shared buffer: the writer appends the request here, and the reader
        // consumes it when first read. This is safe because the protocol flow
        // is strictly sequential: write phase completes fully before read phase begins.
        let shared_request = Arc::new(Mutex::new(Vec::<u8>::new()));

        let reader: Box<dyn ExtendedBufRead<'_> + Unpin + '_> = Box::new(BuiltinReader::new(
            state.ref_store.clone(),
            state.odb.clone(),
            state.object_hash,
            self.path.clone(),
            Arc::clone(&shared_request),
        ));

        let writer = BuiltinWriter {
            buf: shared_request,
        };

        Ok(RequestWriter::new_from_bufread(
            writer,
            reader,
            write_mode,
            on_into_read,
            trace,
        ))
    }
}

// ---------------------------------------------------------------------------
// BuiltinReader - processes the request lazily on first read
// ---------------------------------------------------------------------------

/// A reader that, on first access, takes the buffered request from a shared buffer,
/// processes it through `serve_v2()`, and serves the response.
struct BuiltinReader {
    ref_store: gix_ref::file::Store,
    odb: gix_odb::Handle,
    object_hash: gix_hash::Kind,
    path: BString,
    /// Shared request buffer populated by the writer.
    shared_request: Arc<Mutex<Vec<u8>>>,
    /// The response data produced by serve_v2().
    response_buf: Vec<u8>,
    /// Current read position in response_buf.
    response_pos: usize,
    /// Whether we've already processed the request.
    processed: bool,
    /// The stop reason (always Flush for V2 responses).
    stopped_at: Option<MessageKind>,
}

impl BuiltinReader {
    fn new(
        ref_store: gix_ref::file::Store,
        odb: gix_odb::Handle,
        object_hash: gix_hash::Kind,
        path: BString,
        shared_request: Arc<Mutex<Vec<u8>>>,
    ) -> Self {
        Self {
            ref_store,
            odb,
            object_hash,
            path,
            shared_request,
            response_buf: Vec::new(),
            response_pos: 0,
            processed: false,
            stopped_at: None,
        }
    }

    fn process_request(&mut self) -> io::Result<()> {
        if self.processed {
            return Ok(());
        }
        self.processed = true;

        let request_data = self
            .shared_request
            .lock()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("lock poisoned: {err}")))?
            .clone();

        let config = ServerConfig {
            object_hash: self.object_hash,
        };
        let mut delegate = RepositoryDelegate {
            ref_store: self.ref_store.clone(),
            odb: self.odb.clone(),
            object_hash: self.object_hash,
            path: self.path.clone(),
        };

        let input = io::Cursor::new(request_data);
        self.response_buf.clear();

        serve_v2(input, &mut self.response_buf, &mut delegate, &config).map_err(|err| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("built-in upload-pack failed for '{}': {err}", self.path),
            )
        })?;

        self.response_pos = 0;
        self.stopped_at = Some(MessageKind::Flush);
        Ok(())
    }
}

impl io::Read for BuiltinReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.process_request()?;
        let available = &self.response_buf[self.response_pos..];
        let to_copy = buf.len().min(available.len());
        buf[..to_copy].copy_from_slice(&available[..to_copy]);
        self.response_pos += to_copy;
        Ok(to_copy)
    }
}

impl io::BufRead for BuiltinReader {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.process_request()?;
        Ok(&self.response_buf[self.response_pos..])
    }

    fn consume(&mut self, amt: usize) {
        self.response_pos += amt;
    }
}

impl ReadlineBufRead for BuiltinReader {
    fn readline(
        &mut self,
    ) -> Option<io::Result<Result<PacketLineRef<'_>, gix_transport::packetline::decode::Error>>> {
        if let Err(e) = self.process_request() {
            return Some(Err(e));
        }
        if self.response_pos >= self.response_buf.len() {
            return None;
        }
        let remaining = &self.response_buf[self.response_pos..];
        if remaining.len() < 4 {
            return None;
        }
        match gix_transport::packetline::decode::all_at_once(remaining) {
            Ok(line) => {
                // Compute bytes consumed: for special lines it's 4 bytes, for data lines
                // the length is encoded in the first 4 hex bytes.
                let bytes_consumed = match line {
                    PacketLineRef::Flush | PacketLineRef::Delimiter | PacketLineRef::ResponseEnd => 4,
                    PacketLineRef::Data(data) => 4 + data.len(),
                };
                self.response_pos += bytes_consumed;
                match line {
                    PacketLineRef::Flush => {
                        self.stopped_at = Some(MessageKind::Flush);
                        None
                    }
                    PacketLineRef::Delimiter => {
                        self.stopped_at = Some(MessageKind::Delimiter);
                        None
                    }
                    PacketLineRef::ResponseEnd => {
                        self.stopped_at = Some(MessageKind::ResponseEnd);
                        None
                    }
                    _ => Some(Ok(Ok(line))),
                }
            }
            Err(err) => Some(Ok(Err(err))),
        }
    }

    fn readline_str(&mut self, line: &mut String) -> io::Result<usize> {
        self.process_request()?;
        if self.response_pos >= self.response_buf.len() {
            return Ok(0);
        }
        let remaining = &self.response_buf[self.response_pos..];
        if remaining.len() < 4 {
            return Ok(0);
        }
        match gix_transport::packetline::decode::all_at_once(remaining) {
            Ok(pkt_line) => {
                let bytes_consumed = match pkt_line {
                    PacketLineRef::Flush | PacketLineRef::Delimiter | PacketLineRef::ResponseEnd => 4,
                    PacketLineRef::Data(data) => 4 + data.len(),
                };
                self.response_pos += bytes_consumed;
                match pkt_line {
                    PacketLineRef::Flush => {
                        self.stopped_at = Some(MessageKind::Flush);
                        Ok(0)
                    }
                    PacketLineRef::Delimiter => {
                        self.stopped_at = Some(MessageKind::Delimiter);
                        Ok(0)
                    }
                    PacketLineRef::ResponseEnd => {
                        self.stopped_at = Some(MessageKind::ResponseEnd);
                        Ok(0)
                    }
                    PacketLineRef::Data(data) => {
                        let s = std::str::from_utf8(data)
                            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                        line.push_str(s);
                        Ok(data.len())
                    }
                }
            }
            Err(err) => Err(io::Error::new(io::ErrorKind::InvalidData, err)),
        }
    }
}

impl<'a> ExtendedBufRead<'a> for BuiltinReader {
    fn set_progress_handler(&mut self, _handle_progress: Option<HandleProgress<'a>>) {
        // Progress handling is not needed for the in-process transport.
    }

    fn peek_data_line(&mut self) -> Option<io::Result<Result<&[u8], client::Error>>> {
        if let Err(e) = self.process_request() {
            return Some(Err(e));
        }
        let remaining = &self.response_buf[self.response_pos..];
        if remaining.len() < 4 {
            return None;
        }
        match gix_transport::packetline::decode::all_at_once(remaining) {
            Ok(PacketLineRef::Data(data)) => Some(Ok(Ok(data))),
            Ok(PacketLineRef::Flush | PacketLineRef::Delimiter | PacketLineRef::ResponseEnd) => None,
            Err(err) => Some(Ok(Err(client::Error::LineDecode { err }))),
        }
    }

    fn reset(&mut self, _version: Protocol) {
        self.stopped_at = None;
    }

    fn stopped_at(&self) -> Option<MessageKind> {
        self.stopped_at
    }
}

// ---------------------------------------------------------------------------
// BuiltinWriter - writes request data to the shared buffer
// ---------------------------------------------------------------------------

/// A writer that appends data to a shared request buffer.
struct BuiltinWriter {
    buf: Arc<Mutex<Vec<u8>>>,
}

impl io::Write for BuiltinWriter {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        let mut buf = self
            .buf
            .lock()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("lock poisoned: {err}")))?;
        buf.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// RepositoryDelegate
// ---------------------------------------------------------------------------

/// A delegate implementation that opens a repository's ref store and object database
/// to serve upload-pack requests in-process.
pub(crate) struct RepositoryDelegate {
    ref_store: gix_ref::file::Store,
    odb: gix_odb::Handle,
    object_hash: gix_hash::Kind,
    path: BString,
}

impl RepositoryDelegate {
    /// Create a new delegate for the repository at `path`.
    #[allow(dead_code)]
    pub(crate) fn new(
        ref_store: gix_ref::file::Store,
        odb: gix_odb::Handle,
        object_hash: gix_hash::Kind,
        path: BString,
    ) -> Self {
        Self {
            ref_store,
            odb,
            object_hash,
            path,
        }
    }
}

impl Delegate for RepositoryDelegate {
    fn ls_refs(&mut self, request: &LsRefs) -> Result<Vec<Ref>, BoxError> {
        let packed = self.ref_store.cached_packed_buffer().map_err(|err| -> BoxError {
            Box::new(io::Error::new(
                io::ErrorKind::Other,
                format!("failed to open packed-refs for '{}': {err}", self.path),
            ))
        })?;
        let packed = packed.as_ref().map(|b| &***b);

        let iter = if request.ref_prefixes.is_empty() {
            self.ref_store.iter_packed(packed).map_err(|err| -> BoxError {
                Box::new(io::Error::new(
                    io::ErrorKind::Other,
                    format!("failed to iterate refs for '{}': {err}", self.path),
                ))
            })?
        } else {
            // Use the first prefix for the underlying iteration, then filter in-memory
            // for all prefixes. This matches how git filters refs with multiple prefixes.
            let first_prefix = &request.ref_prefixes[0];
            let prefix_path: &gix_path::RelativePath =
                first_prefix.as_bstr().try_into().map_err(|err| -> BoxError {
                    Box::new(io::Error::new(
                        io::ErrorKind::Other,
                        format!(
                            "invalid ref prefix '{}' for '{}': {err}",
                            first_prefix, self.path
                        ),
                    ))
                })?;
            self.ref_store
                .iter_prefixed_packed(prefix_path, packed)
                .map_err(|err| -> BoxError {
                    Box::new(io::Error::new(
                        io::ErrorKind::Other,
                        format!("failed to iterate refs for '{}': {err}", self.path),
                    ))
                })?
        };

        let mut refs = Vec::new();
        for reference in iter {
            let reference = reference.map_err(|err| -> BoxError {
                Box::new(io::Error::new(
                    io::ErrorKind::Other,
                    format!("failed to read ref in '{}': {err}", self.path),
                ))
            })?;

            let full_ref_name: BString = reference.name.as_bstr().to_owned();

            // Apply prefix filter for multi-prefix case
            if !request.ref_prefixes.is_empty() {
                let matches = request
                    .ref_prefixes
                    .iter()
                    .any(|prefix| full_ref_name.starts_with(prefix.as_bytes()));
                if !matches {
                    continue;
                }
            }

            let r = self.build_ref_entry(reference, &full_ref_name, request, packed)?;
            refs.push(r);
        }

        Ok(refs)
    }

    fn fetch(&mut self, request: &Fetch) -> Result<FetchOutput, BoxError> {
        let negotiation =
            negotiate_fetch_with_repository(request, &self.ref_store, |oid| self.odb.contains(oid)).map_err(
                |err| -> BoxError {
                    Box::new(io::Error::new(
                        io::ErrorKind::Other,
                        format!("fetch negotiation failed for '{}': {err}", self.path),
                    ))
                },
            )?;

        negotiation
            .into_output_with_repository_pack(request, self.odb.clone(), self.object_hash)
            .map_err(|err| -> BoxError {
                Box::new(io::Error::new(
                    io::ErrorKind::Other,
                    format!("pack generation failed for '{}': {err}", self.path),
                ))
            })
    }
}

impl RepositoryDelegate {
    /// Build a `Ref` entry from a raw reference, resolving symref targets and peeled OIDs as needed.
    fn build_ref_entry(
        &self,
        mut reference: gix_ref::Reference,
        full_ref_name: &BString,
        request: &LsRefs,
        packed: Option<&gix_ref::packed::Buffer>,
    ) -> Result<Ref, BoxError> {
        // Determine if this is a symbolic ref
        let symref_target = match &reference.target {
            gix_ref::Target::Symbolic(target) => Some(target.as_bstr().to_owned()),
            gix_ref::Target::Object(_) => None,
        };

        // Resolve to object ID
        let object_id = reference
            .follow_to_object_packed(&self.ref_store, packed)
            .map_err(|err| -> BoxError {
                Box::new(io::Error::new(
                    io::ErrorKind::Other,
                    format!(
                        "failed to resolve ref '{}' in '{}': {err}",
                        full_ref_name, self.path
                    ),
                ))
            })?;

        // Try to peel (for tags)
        let peeled = if request.peel {
            let mut buf = Vec::new();
            match gix_object::Find::try_find(&self.odb, object_id.as_ref(), &mut buf) {
                Ok(Some(obj)) if obj.kind == gix_object::Kind::Tag => {
                    // Peel through tags to find the final object
                    self.peel_tag(object_id, &mut buf).ok()
                }
                _ => None,
            }
        } else {
            // Use peeled info from packed refs if available
            reference.peeled
        };

        match (symref_target, peeled) {
            (Some(target), _) if request.symrefs => Ok(Ref::Symbolic {
                full_ref_name: full_ref_name.clone(),
                target,
                tag: if peeled.is_some() && peeled != Some(object_id) {
                    Some(object_id)
                } else {
                    None
                },
                object: peeled.unwrap_or(object_id),
            }),
            (_, Some(peeled_id)) if peeled_id != object_id => Ok(Ref::Peeled {
                full_ref_name: full_ref_name.clone(),
                tag: object_id,
                object: peeled_id,
            }),
            _ => Ok(Ref::Direct {
                full_ref_name: full_ref_name.clone(),
                object: object_id,
            }),
        }
    }

    /// Peel a tag object to its final non-tag target.
    fn peel_tag(
        &self,
        start_id: gix_hash::ObjectId,
        buf: &mut Vec<u8>,
    ) -> Result<gix_hash::ObjectId, BoxError> {
        let mut id = start_id;
        for _ in 0..100 {
            // limit to prevent infinite loops
            let obj = gix_object::Find::try_find(&self.odb, id.as_ref(), buf)
                .map_err(|err| -> BoxError {
                    Box::new(io::Error::new(
                        io::ErrorKind::Other,
                        format!("failed to find object {} in '{}': {err}", id, self.path),
                    ))
                })?;
            match obj {
                Some(obj) if obj.kind == gix_object::Kind::Tag => {
                    id = gix_object::TagRefIter::from_bytes(obj.data, obj.object_hash)
                        .target_id()
                        .map_err(|err| -> BoxError {
                            Box::new(io::Error::new(
                                io::ErrorKind::Other,
                                format!("failed to decode tag {} in '{}': {err}", id, self.path),
                            ))
                        })?;
                }
                _ => return Ok(id),
            }
        }
        Ok(id)
    }
}

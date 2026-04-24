use bstr::BString;
use gix_hash::ObjectId;

/// One ref update command the client wants the server to perform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    /// The ref to update (e.g. `refs/heads/main`).
    pub refname: BString,
    /// Old OID the client believes the ref to be at, or the zero OID when
    /// creating a new ref.
    pub old_oid: ObjectId,
    /// New OID the client wants the ref at, or the zero OID to delete.
    pub new_oid: ObjectId,
}

impl Command {
    /// `true` if this command creates a ref (old_oid is zero).
    pub fn is_create(&self) -> bool {
        self.old_oid.is_null()
    }
    /// `true` if this command deletes a ref (new_oid is zero).
    pub fn is_delete(&self) -> bool {
        self.new_oid.is_null()
    }
}

/// A fully-formed send-pack request, ready to transmit.
#[derive(Debug, Default)]
pub struct Request {
    pub commands: Vec<Command>,
    /// Capability tokens to advertise on the first command-list line
    /// (e.g. `report-status`, `side-band-64k`, `agent=gix/<ver>`,
    /// `atomic`, `ofs-delta`, `push-options`, `quiet`).
    pub capabilities: Vec<BString>,
}

/// Options that tune the outgoing pack / command list.
#[derive(Debug, Default, Clone)]
pub struct Options {
    /// Build a thin pack (delta-against-remote objects).
    pub thin_pack: bool,
    /// Send `quiet` capability to suppress progress sideband.
    pub quiet: bool,
    /// Send `atomic` capability — server applies all updates or none.
    pub atomic: bool,
    /// Extra `push-option` strings to send after the command list.
    /// (Empty vec = no push-options; the capability is negotiated separately.)
    pub push_options: Vec<BString>,
}

/// Per-ref status from the server's report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefStatus {
    pub refname: BString,
    /// `Ok(())` on success; `Err(reason)` with the server's error text on failure.
    pub result: Result<(), BString>,
}

/// Parsed `report-status` response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// The `unpack` line's status: `Ok(())` for "unpack ok", `Err(msg)` otherwise.
    pub unpack: Result<(), BString>,
    /// Per-ref outcomes in the order the server reported.
    pub refs: Vec<RefStatus>,
}

/// Summary outcome of a send-pack call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub report: Report,
}

/// Errors that terminate the send-pack call before or during transmission.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error while writing to the transport")]
    Io(#[from] std::io::Error),
    #[error("unexpected end of stream while reading the push report")]
    UnexpectedEof,
    #[error("could not parse server report line: {line:?}")]
    MalformedReport { line: BString },
    #[error("transport error")]
    Transport(#[from] gix_transport::client::Error),
    #[error("pack generation failed")]
    Pack(String),
}

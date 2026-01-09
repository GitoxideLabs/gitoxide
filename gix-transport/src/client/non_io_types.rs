/// Configure how a `RequestWriter` behaves when writing bytes.
#[derive(Default, PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WriteMode {
    /// Each [write()][std::io::Write::write()] call writes the bytes verbatim as one or more packet lines.
    ///
    /// This mode also indicates to the transport that it should try to stream data as it is unbounded. This mode is typically used
    /// for sending packs whose exact size is not necessarily known in advance.
    Binary,
    /// Each [write()][std::io::Write::write()] call assumes text in the input, assures a trailing newline and writes it as single packet line.
    ///
    /// This mode also indicates that the lines written fit into memory, hence the transport may chose to not stream it but to buffer it
    /// instead. This is relevant for some transports, like the one for HTTP.
    #[default]
    OneLfTerminatedLinePerWriteCall,
}

/// The kind of packet line to write when transforming a `RequestWriter` into an `ExtendedBufRead`.
///
/// Both the type and the trait have different implementations for blocking vs async I/O.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MessageKind {
    /// A `flush` packet.
    Flush,
    /// A V2 delimiter.
    Delimiter,
    /// The end of a response.
    ResponseEnd,
    /// The given text.
    Text(&'static [u8]),
}

#[cfg(any(feature = "blocking-client", feature = "async-client"))]
pub(crate) mod connect {
    /// Options for connecting to a remote.
    #[derive(Debug, Default, Clone)]
    pub struct Options {
        /// Use `version` to set the desired protocol version to use when connecting, but note that the server may downgrade it.
        pub version: crate::Protocol,
        #[cfg(feature = "blocking-client")]
        /// Options to use if the scheme of the URL is `ssh`.
        pub ssh: crate::client::blocking_io::ssh::connect::Options,
        /// If `true`, all packetlines received or sent will be passed to the facilities of the `gix-trace` crate.
        pub trace: bool,
    }

    /// The error used in `connect()`.
    ///
    /// (Both blocking and async I/O use the same error type.)
    pub type Error = crate::Error;
}

/// The error used in most methods of the [`client`][crate::client] module.
pub type Error = crate::Error;

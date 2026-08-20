use std::io;

#[crate::bisync::only_async]
use crate::transport::client::async_io::ExtendedBufRead;
#[crate::bisync::only_sync]
use crate::transport::client::blocking_io::ExtendedBufRead;
use gix_error::{CorruptionError, ErrorExt, RetryableError, message};
use gix_transport::{Protocol, client, client::MessageKind};

use crate::fetch::{
    Response, response,
    response::{Acknowledgement, ShallowUpdate, WantedRef, shallow_update_from_line},
};

#[crate::bisync::bisync]
async fn parse_v2_section<'a, T>(
    line: &mut String,
    reader: &mut impl ExtendedBufRead<'a>,
    res: &mut Vec<T>,
    parse: impl Fn(&str) -> Result<T, response::Error>,
) -> Result<bool, response::Error> {
    line.clear();
    while reader.readline_str(line).await.map_err(read_error)? != 0 {
        res.push(parse(line)?);
        line.clear();
    }
    // End of message, or end of section?
    Ok(if reader.stopped_at() == Some(MessageKind::Delimiter) {
        // try reading more sections
        reader.reset(Protocol::V2);
        false
    } else {
        // we are done, there is no pack
        true
    })
}

impl Response {
    /// Parse a response of the given `version` of the protocol from `reader`.
    ///
    /// `client_expects_pack` is only relevant for V1 stateful connections, and if `false`, causes us to stop parsing when seeing `NAK`,
    /// and if `true` we will keep parsing until we get a pack as the client already signalled to the server that it's done.
    /// This way of doing things allows us to exploit knowledge about more recent versions of the protocol, which keeps code easier
    /// and more localized without having to support all the cruft that there is.
    ///
    /// `wants_to_negotiate` should be `false` for clones which is when we don't have sent any haves. The reason for this flag to exist
    /// is to predict how to parse V1 output only, and neither `client_expects_pack` nor `wants_to_negotiate` are relevant for V2.
    /// This ugliness is in place to avoid having to resort to an [an even more complex ugliness](https://github.com/git/git/blob/9e49351c3060e1fa6e0d2de64505b7becf157f28/fetch-pack.c#L583-L594)
    /// that `git` has to use to predict how many acks are supposed to be read. We also genuinely hope that this covers it all….
    #[crate::bisync::bisync]
    pub async fn from_line_reader<'a>(
        version: Protocol,
        reader: &mut impl ExtendedBufRead<'a>,
        client_expects_pack: bool,
        wants_to_negotiate: bool,
    ) -> Result<Response, response::Error> {
        match version {
            Protocol::V0 | Protocol::V1 => {
                let mut line = String::new();
                let mut acks = Vec::<Acknowledgement>::new();
                let mut shallows = Vec::<ShallowUpdate>::new();
                let mut saw_ready = false;
                let has_pack = 'lines: loop {
                    line.clear();
                    let peeked_line = match reader.peek_data_line().await {
                        Some(Ok(Ok(line))) => String::from_utf8_lossy(line),
                        // This special case (hang/block forever) deals with a single NAK being a legitimate EOF sometimes
                        // Note that this might block forever in stateful connections as there it's not really clear
                        // if something will be following or not by just looking at the response. Instead you have to know
                        // [a lot](https://github.com/git/git/blob/9e49351c3060e1fa6e0d2de64505b7becf157f28/fetch-pack.c#L583-L594)
                        // to deal with this correctly.
                        // For now this is acceptable, as V2 can be used as a workaround, which also is the default.
                        Some(Err(err)) if err.kind() == io::ErrorKind::UnexpectedEof => break 'lines false,
                        Some(Err(err)) => return Err(read_error(err)),
                        Some(Ok(Err(err))) => return Err(transport_error(err)),
                        None => {
                            // maybe we saw a shallow flush packet, let's reset and retry
                            debug_assert_eq!(
                                reader.stopped_at(),
                                Some(MessageKind::Flush),
                                "If this isn't a flush packet, we don't know what's going on"
                            );
                            reader.readline_str(&mut line).await.map_err(read_error)?;
                            reader.reset(Protocol::V1);
                            match reader.peek_data_line().await {
                                Some(Ok(Ok(line))) => String::from_utf8_lossy(line),
                                Some(Err(err)) => return Err(read_error(err)),
                                Some(Ok(Err(err))) => return Err(transport_error(err)),
                                None => break 'lines false, // EOF
                            }
                        }
                    };

                    if Response::parse_v1_ack_or_shallow_or_assume_pack(&mut acks, &mut shallows, &peeked_line) {
                        break 'lines true;
                    }
                    assert_ne!(
                        reader.readline_str(&mut line).await.map_err(read_error)?,
                        0,
                        "consuming a peeked line works"
                    );
                    // When the server sends ready, we know there is going to be a pack so no need to stop early.
                    saw_ready |= matches!(acks.last(), Some(Acknowledgement::Ready));
                    if let Some(Acknowledgement::Nak) = acks.last().filter(|_| !client_expects_pack || !saw_ready) {
                        if !wants_to_negotiate {
                            continue;
                        }
                        break 'lines false;
                    }
                };
                Ok(Response {
                    acks,
                    shallows,
                    wanted_refs: vec![],
                    has_pack,
                })
            }
            Protocol::V2 => {
                // NOTE: We only read acknowledgements and scrub to the pack file, until we have use for the other features
                let mut line = String::new();
                reader.reset(Protocol::V2);
                let mut acks = Vec::<Acknowledgement>::new();
                let mut shallows = Vec::<ShallowUpdate>::new();
                let mut wanted_refs = Vec::<WantedRef>::new();
                let has_pack = 'section: loop {
                    line.clear();
                    if reader.readline_str(&mut line).await.map_err(read_error)? == 0 {
                        return Err(read_error(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Could not read message headline",
                        )));
                    }

                    match line.trim_end() {
                        "acknowledgments" => {
                            if parse_v2_section(&mut line, reader, &mut acks, Acknowledgement::from_line).await? {
                                break 'section false;
                            }
                        }
                        "shallow-info" => {
                            if parse_v2_section(&mut line, reader, &mut shallows, shallow_update_from_line).await? {
                                break 'section false;
                            }
                        }
                        "wanted-refs" => {
                            if parse_v2_section(&mut line, reader, &mut wanted_refs, WantedRef::from_line).await? {
                                break 'section false;
                            }
                        }
                        "packfile" => {
                            // what follows is the packfile itself, which can be read with a sideband enabled reader
                            break 'section true;
                        }
                        _ => {
                            return Err(
                                CorruptionError::new(format!("Unknown or unsupported header: {line:?}")).raise_erased()
                            );
                        }
                    }
                };
                Ok(Response {
                    acks,
                    shallows,
                    wanted_refs,
                    has_pack,
                })
            }
        }
    }
}

fn read_error(err: io::Error) -> response::Error {
    let err = if err.kind() == io::ErrorKind::Other {
        match err.into_inner() {
            Some(err) => match err.downcast::<gix_transport::packetline::read::Error>() {
                Ok(err) => return (*err).and_raise(message("Failed to read from line reader")).erased(),
                Err(err) => io::Error::other(err),
            },
            None => io::ErrorKind::Other.into(),
        }
    } else {
        err
    };
    err.and_raise(message("Failed to read from line reader")).erased()
}

fn transport_error(err: client::Error) -> response::Error {
    use crate::transport::IsSpuriousError;

    let context = message("Failed to read from line reader");
    if err.is_spurious() {
        RetryableError::new(err).and_raise(context).erased()
    } else {
        err.and_raise(context).erased()
    }
}

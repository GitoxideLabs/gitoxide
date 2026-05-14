use crate::transport::client::blocking_io::ReadlineBufRead;
use crate::{
    fetch::response::ShallowUpdate,
    handshake::{Ref, refs, refs::parse::Error},
};

/// Parse refs from the given input line by line. Protocol V2 is required for this to succeed.
pub fn from_v2_refs(in_refs: &mut dyn ReadlineBufRead) -> Result<Vec<Ref>, Error> {
    let mut out_refs = Vec::new();
    while let Some(line) = in_refs.readline().transpose()?.transpose()?.and_then(|l| l.as_bstr()) {
        out_refs.push(refs::shared::parse_v2(line)?);
    }
    Ok(out_refs)
}

/// Parse refs from the return stream of the handshake as well as the server capabilities, also received as part of the
/// handshake.
/// Together they form a complete set of refs.
///
/// # Note
///
/// Symbolic refs are shoe-horned into server capabilities whereas refs (without symbolic ones) are sent automatically as
/// part of the handshake. Both symbolic and peeled refs need to be combined to fit into the [`Ref`] type provided here.
pub fn from_v1_refs_received_as_part_of_handshake_and_capabilities<'a>(
    in_refs: &mut dyn ReadlineBufRead,
    capabilities: impl Iterator<Item = gix_transport::client::capabilities::Capability<'a>>,
) -> Result<(Vec<Ref>, Vec<ShallowUpdate>), Error> {
    let mut out_refs = refs::shared::from_capabilities(capabilities)?;
    let mut out_shallow = Vec::new();
    let number_of_possible_symbolic_refs_for_lookup = out_refs.len();

    while let Some(line) = in_refs.readline().transpose()?.transpose()?.and_then(|l| l.as_bstr()) {
        refs::shared::parse_v1(
            number_of_possible_symbolic_refs_for_lookup,
            &mut out_refs,
            &mut out_shallow,
            line,
        )?;
    }
    Ok((out_refs.into_iter().map(Into::into).collect(), out_shallow))
}

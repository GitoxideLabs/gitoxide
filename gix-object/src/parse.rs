use bstr::{BStr, BString, ByteVec};
use winnow::{
    combinator::{preceded, repeat, terminated},
    error::{AddContext, ParserError, StrContext},
    prelude::*,
    token::{take_till, take_until, take_while},
};

use crate::ByteSlice;

pub(crate) const NL: &[u8] = b"\n";
pub(crate) const SPACE: &[u8] = b" ";
const SPACE_OR_NL: &[u8] = b" \n";

pub(crate) fn any_header_field_multi_line<'a, E: ParserError<&'a [u8]> + AddContext<&'a [u8], StrContext>>(
    i: &mut &'a [u8],
) -> ModalResult<(&'a [u8], BString), E> {
    (
        terminated(take_till(1.., SPACE_OR_NL), SPACE),
        (
            take_till(0.., NL),
            NL,
            repeat(1.., terminated((SPACE, take_until(0.., NL)), NL)).map(|()| ()),
        )
            .take()
            .map(|o: &[u8]| {
                let bytes = o.as_bstr();
                let mut out = BString::from(Vec::with_capacity(bytes.len()));
                let mut lines = bytes.lines_with_terminator();
                out.push_str(lines.next().expect("first line"));
                for line in lines {
                    out.push_str(&line[1..]); // cut leading space
                }
                out
            }),
    )
        .context(StrContext::Expected("name <multi-line-value>".into()))
        .parse_next(i)
}

pub(crate) fn header_field<'a, T, E: ParserError<&'a [u8]>>(
    i: &mut &'a [u8],
    name: &'static [u8],
    parse_value: impl ModalParser<&'a [u8], T, E>,
) -> ModalResult<T, E> {
    terminated(preceded(terminated(name, SPACE), parse_value), NL).parse_next(i)
}

pub(crate) fn any_header_field<'a, T, E: ParserError<&'a [u8]>>(
    i: &mut &'a [u8],
    parse_value: impl ModalParser<&'a [u8], T, E>,
) -> ModalResult<(&'a [u8], T), E> {
    terminated((terminated(take_till(1.., SPACE_OR_NL), SPACE), parse_value), NL).parse_next(i)
}

fn is_hex_digit_lc(b: u8) -> bool {
    matches!(b, b'0'..=b'9' | b'a'..=b'f')
}

pub fn hex_hash<'a, E: ParserError<&'a [u8]>>(i: &mut &'a [u8]) -> ModalResult<&'a BStr, E> {
    take_while(
        gix_hash::Kind::shortest().len_in_hex()..=gix_hash::Kind::longest().len_in_hex(),
        is_hex_digit_lc,
    )
    .map(ByteSlice::as_bstr)
    .parse_next(i)
}

pub(crate) fn signature<'a, E: ParserError<&'a [u8]> + AddContext<&'a [u8], StrContext>>(
    i: &mut &'a [u8],
) -> ModalResult<gix_actor::SignatureRef<'a>, E> {
    gix_actor::signature::decode(i)
}

pub(crate) fn signature_with_raw<'a, E: ParserError<&'a [u8]> + AddContext<&'a [u8], StrContext>>(
    i: &mut &'a [u8],
) -> ModalResult<(gix_actor::SignatureRef<'a>, &'a BStr), E> {
    let original = *i;
    gix_actor::signature::decode(i).map(|signature| {
        let consumed = original.len() - i.len();
        (signature, original[..consumed].as_bstr())
    })
}

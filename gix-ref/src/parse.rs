use gix_object::bstr::{BStr, ByteSlice};

type ParseResult<T> = Result<T, ()>;

fn is_hex_digit(b: u8) -> bool {
    b.is_ascii_hexdigit()
}

/// Copy from `gix-object`, validating the hash against `object_hash`.
pub fn hex_hash<'a>(i: &mut &'a [u8], object_hash: gix_hash::Kind) -> ParseResult<&'a BStr> {
    let len = object_hash.len_in_hex();
    let Some(hex) = i.get(..len) else {
        return Err(());
    };
    if !hex.iter().all(|b| is_hex_digit(*b)) {
        return Err(());
    }
    *i = &i[len..];
    Ok(hex.as_bstr())
}

/// All supported hash lengths, if they match perfectly.
pub fn hex_hash_any<'a>(i: &mut &'a [u8]) -> ParseResult<&'a BStr> {
    let max = gix_hash::Kind::longest().len_in_hex();
    let len = i.iter().take(max).take_while(|b| is_hex_digit(**b)).count();
    if !gix_hash::Kind::all().iter().any(|kind| kind.len_in_hex() == len) {
        return Err(());
    }
    let (hex, rest) = i.split_at(len);
    *i = rest;
    Ok(hex.as_bstr())
}

/// Parse CRLF or LF, independently of the platform.
pub fn newline<'a>(i: &mut &'a [u8]) -> ParseResult<&'a [u8]> {
    if let Some(rest) = i.strip_prefix(b"\r\n") {
        let out = &i[..2];
        *i = rest;
        Ok(out)
    } else if let Some(rest) = i.strip_prefix(b"\n") {
        let out = &i[..1];
        *i = rest;
        Ok(out)
    } else {
        Err(())
    }
}

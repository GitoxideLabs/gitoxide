use crate::Encoding;

const INVALID: u32 = u32::MAX;
const EMPTY: u32 = u32::MAX - 1;
const CHILD: u32 = 0x8000_0000;
const PAIR: u32 = 0x4000_0000;

#[derive(Debug, Default, Clone)]
pub(crate) struct State {
    #[cfg(feature = "asian")]
    iso_mode: u8,
    #[cfg(feature = "legacy")]
    combining: usize,
    #[cfg(feature = "asian")]
    hkscs_pending: Option<char>,
}

pub(crate) enum Decoded {
    Scalar(char, usize),
    Pair(char, char, usize),
    Skip(usize),
    Incomplete,
}

#[derive(Clone, Copy)]
struct Table(&'static [u8]);

#[inline]
fn u32_at(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data.as_chunks::<4>().0[offset / 4])
}

impl Table {
    /// Decode a single table entry for the bulk UTF-8 path. Stored scalars are
    /// generated and checked exhaustively; markers cannot enter the result.
    #[inline]
    fn scalar(self, input: &[u8]) -> Option<(u32, usize)> {
        let first = input[0];
        let mut value = u32_at(self.0, 16 + usize::from(first) * 4);
        let mut consumed = 1;
        if value & CHILD != 0 && value != INVALID {
            let &second = input.get(1)?;
            value = u32_at(self.0, 16 + (usize::from(first) + 1) * 1024 + usize::from(second) * 4);
            consumed = 2;
            if value & CHILD != 0 && value != INVALID {
                let &third = input.get(2)?;
                value = u32_at(self.0, 16 + (value & !CHILD) as usize * 1024 + usize::from(third) * 4);
                consumed = 3;
            }
        }
        (value < 0x110000).then_some((value, consumed))
    }

    #[inline]
    fn decode(self, input: &[u8]) -> Result<Decoded, ()> {
        let mut node = 0;
        for (index, &byte) in input.iter().enumerate() {
            let value = u32_at(self.0, 16 + node * 1024 + usize::from(byte) * 4);
            if value == INVALID {
                return Err(());
            }
            if value & CHILD != 0 {
                // Second-byte pages are dense, avoiding a dependent pointer
                // lookup on the common two-byte CJK path.
                node = if index == 0 {
                    usize::from(byte) + 1
                } else {
                    (value & !CHILD) as usize
                };
                continue;
            }
            return if value & PAIR != 0 {
                let offset = 16
                    + u32_at(self.0, 0) as usize * 1024
                    + 512
                    + u32_at(self.0, 4) as usize * 1024
                    + u32_at(self.0, 8) as usize * 8
                    + (value & !PAIR) as usize * 8;
                Ok(Decoded::Pair(
                    char::from_u32(u32_at(self.0, offset)).ok_or(())?,
                    char::from_u32(u32_at(self.0, offset + 4)).ok_or(())?,
                    index + 1,
                ))
            } else {
                Ok(Decoded::Scalar(char::from_u32(value).ok_or(())?, index + 1))
            };
        }
        Ok(Decoded::Incomplete)
    }

    #[inline]
    fn encoded(self, scalar: char) -> Result<u32, ()> {
        self.encoder().encoded(scalar)
    }

    #[inline]
    fn encoder(self) -> EncodeTable {
        let pages_offset = 16 + u32_at(self.0, 0) as usize * 1024;
        let supplementary_offset = pages_offset + 512 + u32_at(self.0, 4) as usize * 1024;
        EncodeTable {
            page_map: &self.0[pages_offset..pages_offset + 512],
            pages: &self.0[pages_offset + 512..supplementary_offset],
            supplementary: &self.0[supplementary_offset..supplementary_offset + u32_at(self.0, 8) as usize * 8],
            dense: u32_at(self.0, 4) == 256,
        }
    }
}

#[derive(Clone, Copy)]
struct EncodeTable {
    page_map: &'static [u8],
    pages: &'static [u8],
    supplementary: &'static [u8],
    dense: bool,
}

impl EncodeTable {
    #[inline]
    fn encoded(self, scalar: char) -> Result<u32, ()> {
        self.encoded_scalar(scalar as u32)
    }

    #[inline]
    fn encoded_scalar(self, scalar: u32) -> Result<u32, ()> {
        let scalar = scalar as usize;
        let value = if scalar < 0x10000 && self.dense {
            u32_at(self.pages, scalar * 4)
        } else if scalar < 0x10000 {
            let page_offset = (scalar >> 8) * 2;
            let page = u16::from_le_bytes([self.page_map[page_offset], self.page_map[page_offset + 1]]);
            if page == u16::MAX {
                return Err(());
            }
            u32_at(self.pages, usize::from(page) * 1024 + (scalar & 255) * 4)
        } else {
            let mut left = 0;
            let mut right = self.supplementary.len() / 8;
            while left < right {
                let middle = left + (right - left) / 2;
                if u32_at(self.supplementary, middle * 8) < scalar as u32 {
                    left = middle + 1;
                } else {
                    right = middle;
                }
            }
            if left == self.supplementary.len() / 8 || u32_at(self.supplementary, left * 8) != scalar as u32 {
                return Err(());
            }
            u32_at(self.supplementary, left * 8 + 4)
        };
        if value == INVALID { Err(()) } else { Ok(value) }
    }
}

macro_rules! compressed {
    ($name:literal) => {{
        static TABLE: crate::table::CompressedTable =
            crate::table::CompressedTable::new(include_bytes!(concat!(env!("OUT_DIR"), "/", $name, ".bin")));
        TABLE.get()
    }};
}

fn table(encoding: Encoding) -> Table {
    use Encoding::*;
    Table(match encoding {
        Ascii => compressed!("ASCII"),
        Iso8859_1 => compressed!("ISO-8859-1"),
        #[cfg(feature = "legacy")]
        Iso8859_2 => compressed!("ISO-8859-2"),
        #[cfg(feature = "legacy")]
        Iso8859_3 => compressed!("ISO-8859-3"),
        #[cfg(feature = "legacy")]
        Iso8859_4 => compressed!("ISO-8859-4"),
        #[cfg(feature = "legacy")]
        Iso8859_5 => compressed!("ISO-8859-5"),
        #[cfg(feature = "legacy")]
        Iso8859_6 => compressed!("ISO-8859-6"),
        #[cfg(feature = "legacy")]
        Iso8859_7 => compressed!("ISO-8859-7"),
        #[cfg(feature = "legacy")]
        Iso8859_8 => compressed!("ISO-8859-8"),
        #[cfg(feature = "legacy")]
        Iso8859_9 => compressed!("ISO-8859-9"),
        #[cfg(feature = "legacy")]
        Iso8859_10 => compressed!("ISO-8859-10"),
        #[cfg(feature = "legacy")]
        Iso8859_11 => compressed!("ISO-8859-11"),
        #[cfg(feature = "legacy")]
        Iso8859_13 => compressed!("ISO-8859-13"),
        #[cfg(feature = "legacy")]
        Iso8859_14 => compressed!("ISO-8859-14"),
        #[cfg(feature = "western")]
        Iso8859_15 => compressed!("ISO-8859-15"),
        #[cfg(feature = "legacy")]
        Iso8859_16 => compressed!("ISO-8859-16"),
        #[cfg(feature = "legacy")]
        Windows1250 => compressed!("WINDOWS-1250"),
        #[cfg(feature = "legacy")]
        Windows1251 => compressed!("WINDOWS-1251"),
        #[cfg(feature = "western")]
        Windows1252 => compressed!("WINDOWS-1252"),
        #[cfg(feature = "legacy")]
        Windows1253 => compressed!("WINDOWS-1253"),
        #[cfg(feature = "legacy")]
        Windows1254 => compressed!("WINDOWS-1254"),
        #[cfg(feature = "legacy")]
        Windows1255 => compressed!("WINDOWS-1255"),
        #[cfg(feature = "legacy")]
        Windows1256 => compressed!("WINDOWS-1256"),
        #[cfg(feature = "legacy")]
        Windows1257 => compressed!("WINDOWS-1257"),
        #[cfg(feature = "legacy")]
        Windows1258 => compressed!("WINDOWS-1258"),
        #[cfg(feature = "legacy")]
        Windows874 => compressed!("WINDOWS-874"),
        #[cfg(feature = "legacy")]
        Tis620 => compressed!("TIS-620"),
        #[cfg(feature = "legacy")]
        Koi8R => compressed!("KOI8-R"),
        #[cfg(feature = "legacy")]
        Koi8U => compressed!("KOI8-U"),
        #[cfg(feature = "legacy")]
        Macintosh => compressed!("MACINTOSH"),
        #[cfg(feature = "legacy")]
        MacCyrillic => compressed!("MAC-CYRILLIC"),
        #[cfg(feature = "legacy")]
        Ibm437 => compressed!("CP437"),
        #[cfg(feature = "legacy")]
        Ibm850 => compressed!("CP850"),
        #[cfg(feature = "legacy")]
        Ibm866 => compressed!("CP866"),
        #[cfg(feature = "legacy")]
        Cp1125 => compressed!("CP1125"),
        #[cfg(feature = "asian")]
        ShiftJis => compressed!("SHIFT_JIS"),
        #[cfg(feature = "asian")]
        Cp932 => compressed!("CP932"),
        #[cfg(feature = "asian")]
        EucJp | Iso2022Jp => compressed!("EUC-JP"),
        #[cfg(feature = "asian")]
        EucKr => compressed!("EUC-KR"),
        #[cfg(feature = "asian")]
        Cp949 => compressed!("CP949"),
        #[cfg(feature = "asian")]
        Gb2312 => compressed!("GB2312"),
        #[cfg(feature = "asian")]
        Gbk => compressed!("GBK"),
        #[cfg(feature = "asian")]
        Gb18030 => compressed!("GB18030"),
        #[cfg(feature = "asian")]
        Big5 => compressed!("BIG5"),
        #[cfg(feature = "asian")]
        Big5Hkscs => compressed!("BIG5-HKSCS"),
        // Unicode conversion uses unicode.rs and never requests a legacy table.
        _ => unreachable!("Unicode has an algorithmic converter"),
    })
}

#[cfg(feature = "legacy")]
fn combining_table(encoding: Encoding) -> &'static [u8] {
    match encoding {
        Encoding::Windows1255 => compressed!("WINDOWS-1255-combining"),
        Encoding::Windows1258 => compressed!("WINDOWS-1258-combining"),
        _ => unreachable!("only Hebrew and Vietnamese use composition tables"),
    }
}

pub(crate) fn decode(encoding: Encoding, input: &[u8], state: &mut State) -> Result<Decoded, ()> {
    if input.is_empty() {
        return Ok(Decoded::Incomplete);
    }
    #[cfg(any(feature = "legacy", feature = "asian"))]
    let first = input[0];
    #[cfg(not(any(feature = "legacy", feature = "asian")))]
    let _ = state;
    match encoding {
        #[cfg(feature = "asian")]
        Encoding::Iso2022Jp => decode_iso2022(input, state),
        #[cfg(feature = "legacy")]
        Encoding::Windows1255 | Encoding::Windows1258 => {
            let table = combining_table(encoding);
            let offset = 4 + state.combining * (4 + 256 * 12) + 4 + usize::from(first) * 12;
            let first = u32_at(table, offset);
            let second = u32_at(table, offset + 4);
            if first == INVALID {
                return Err(());
            }
            let result = if first == EMPTY {
                Decoded::Skip(1)
            } else if second == EMPTY {
                Decoded::Scalar(char::from_u32(first).ok_or(())?, 1)
            } else {
                Decoded::Pair(char::from_u32(first).ok_or(())?, char::from_u32(second).ok_or(())?, 1)
            };
            state.combining = u32_at(table, offset + 8) as usize;
            Ok(result)
        }
        #[cfg(feature = "asian")]
        Encoding::Gb18030 if (0x81..=0xfe).contains(&first) => {
            if let Some(&second) = input.get(1)
                && (0x30..=0x39).contains(&second)
            {
                if input.len() < 4 {
                    return if input.get(2).is_none_or(|third| (0x81..=0xfe).contains(third)) {
                        Ok(Decoded::Incomplete)
                    } else {
                        Err(())
                    };
                }
                if !(0x81..=0xfe).contains(&input[2]) || !(0x30..=0x39).contains(&input[3]) {
                    return Err(());
                }
                let pointer = (((u32::from(first) - 0x81) * 10 + u32::from(second) - 0x30) * 126 + u32::from(input[2])
                    - 0x81)
                    * 10
                    + u32::from(input[3])
                    - 0x30;
                let scalar = range_lookup(gb18030_decode_ranges(), pointer).ok_or(())?;
                return Ok(Decoded::Scalar(char::from_u32(scalar).ok_or(())?, 4));
            }
            table(encoding).decode(input)
        }
        _ => table(encoding).decode(input),
    }
}

pub(crate) fn decode_finish(encoding: Encoding, state: &mut State) -> Option<char> {
    #[cfg(feature = "legacy")]
    if matches!(encoding, Encoding::Windows1255 | Encoding::Windows1258) && state.combining != 0 {
        let scalar = u32_at(combining_table(encoding), 4 + state.combining * (4 + 256 * 12));
        state.combining = 0;
        return char::from_u32(scalar);
    }
    let _ = (encoding, state);
    None
}

#[cfg(feature = "asian")]
fn decode_iso2022(input: &[u8], state: &mut State) -> Result<Decoded, ()> {
    let first = input[0];
    if first == 0x1b {
        if input.len() < 3 {
            return Ok(Decoded::Incomplete);
        }
        let mode = match &input[1..3] {
            b"(B" => Some(0),
            b"(J" => Some(1),
            b"$@" | b"$B" => Some(2),
            _ => None,
        };
        if let Some(mode) = mode {
            state.iso_mode = mode;
            return Ok(Decoded::Skip(3));
        }
        return Ok(Decoded::Scalar('\x1b', 1));
    }
    if first >= 0x80 {
        return Err(());
    }
    if state.iso_mode != 2 || first < 0x21 || first == 0x7f {
        return Ok(Decoded::Scalar(
            match (state.iso_mode, first) {
                (1, 0x5c) => '\u{a5}',
                (1, 0x7e) => '\u{203e}',
                _ => char::from(first),
            },
            1,
        ));
    }
    let Some(&second) = input.get(1) else {
        return Ok(Decoded::Incomplete);
    };
    if !(0x21..=0x7e).contains(&second) {
        return Err(());
    }
    table(Encoding::EucJp).decode(&[first | 0x80, second | 0x80])
}

#[cfg(feature = "asian")]
fn gb18030_decode_ranges() -> &'static [u8] {
    compressed!("gb18030-ranges")
}

#[cfg(feature = "asian")]
fn gb18030_encode_ranges() -> &'static [u8] {
    compressed!("gb18030-encode-ranges")
}

#[cfg(feature = "asian")]
fn range_lookup(table: &[u8], key: u32) -> Option<u32> {
    // The final range covers supplementary Unicode characters. Test it first
    // to keep emoji and other non-BMP text out of the binary search.
    let last = table.len().checked_sub(12)?;
    if key >= u32_at(table, last) {
        let delta = key - u32_at(table, last);
        return (delta < u32_at(table, last + 4)).then(|| u32_at(table, last + 8) + delta);
    }
    let mut left = 0;
    let mut right = table.len() / 12;
    while left < right {
        let middle = left + (right - left) / 2;
        if u32_at(table, middle * 12) <= key {
            left = middle + 1;
        } else {
            right = middle;
        }
    }
    let offset = left.checked_sub(1)? * 12;
    let delta = key - u32_at(table, offset);
    (delta < u32_at(table, offset + 4)).then(|| u32_at(table, offset + 8) + delta)
}

#[inline]
fn emit(value: u32, out: &mut [u8]) -> usize {
    if value <= 0xff {
        out[0] = value as u8;
        1
    } else if value <= 0xffff {
        out[..2].copy_from_slice(&(value as u16).to_be_bytes());
        2
    } else if value <= 0xffffff {
        out[0] = (value >> 16) as u8;
        out[1] = (value >> 8) as u8;
        out[2] = value as u8;
        3
    } else if value == EMPTY {
        0
    } else {
        out[..4].copy_from_slice(&value.to_be_bytes());
        4
    }
}

#[inline]
fn emit_utf8(scalar: u32, out: &mut [u8]) -> usize {
    // CJK is the common multibyte-table case; ASCII bypasses the table loop.
    if (0x800..0x10000).contains(&scalar) {
        out[0] = 0xe0 | (scalar >> 12) as u8;
        out[1] = 0x80 | ((scalar >> 6) & 0x3f) as u8;
        out[2] = 0x80 | (scalar & 0x3f) as u8;
        3
    } else if scalar < 0x80 {
        out[0] = scalar as u8;
        1
    } else if scalar < 0x800 {
        out[0] = 0xc0 | (scalar >> 6) as u8;
        out[1] = 0x80 | (scalar & 0x3f) as u8;
        2
    } else {
        out[0] = 0xf0 | (scalar >> 18) as u8;
        out[1] = 0x80 | ((scalar >> 12) & 0x3f) as u8;
        out[2] = 0x80 | ((scalar >> 6) & 0x3f) as u8;
        out[3] = 0x80 | (scalar & 0x3f) as u8;
        4
    }
}

pub(crate) fn encode(encoding: Encoding, scalar: char, state: &mut State, out: &mut [u8; 16]) -> Result<usize, ()> {
    #[cfg(not(feature = "asian"))]
    let _ = state;
    #[cfg(feature = "asian")]
    if encoding == Encoding::Iso2022Jp {
        return encode_iso2022(scalar, state, out);
    }
    #[cfg(feature = "asian")]
    if encoding == Encoding::Big5Hkscs {
        let mut written = 0;
        if let Some(pending) = state.hkscs_pending {
            let paired = match (pending, scalar) {
                ('\u{ca}', '\u{304}') => Some(0x8862),
                ('\u{ca}', '\u{30c}') => Some(0x8864),
                ('\u{ea}', '\u{304}') => Some(0x88a3),
                ('\u{ea}', '\u{30c}') => Some(0x88a5),
                _ => None,
            };
            if let Some(paired) = paired {
                state.hkscs_pending = None;
                return Ok(emit(paired, out));
            }
            written = emit(table(encoding).encoded(pending)?, out);
        }
        if matches!(scalar, '\u{ca}' | '\u{ea}') {
            state.hkscs_pending = Some(scalar);
            return Ok(written);
        }
        let value = table(encoding).encoded(scalar)?;
        state.hkscs_pending = None;
        return Ok(written + emit(value, &mut out[written..]));
    }
    match table(encoding).encoded(scalar) {
        Ok(value) => Ok(emit(value, out)),
        #[cfg(feature = "asian")]
        Err(()) if encoding == Encoding::Gb18030 => {
            let mut pointer = range_lookup(gb18030_encode_ranges(), scalar as u32).ok_or(())?;
            out[3] = (pointer % 10) as u8 + 0x30;
            pointer /= 10;
            out[2] = (pointer % 126) as u8 + 0x81;
            pointer /= 126;
            out[1] = (pointer % 10) as u8 + 0x30;
            pointer /= 10;
            out[0] = pointer as u8 + 0x81;
            Ok(4)
        }
        Err(()) => Err(()),
    }
}

#[cfg(feature = "asian")]
fn encode_iso2022(scalar: char, state: &mut State, out: &mut [u8; 16]) -> Result<usize, ()> {
    let (mode, value) = match scalar {
        '\u{a5}' => (1, 0x5c),
        '\u{203e}' => (1, 0x7e),
        _ if scalar.is_ascii() => (0, scalar as u32),
        _ => {
            let value = table(Encoding::EucJp).encoded(scalar)?;
            if value == EMPTY {
                return Err(());
            }
            if !(0xa1a1..=0xfefe).contains(&value) || value & 0xff < 0xa1 || value & 0xff > 0xfe {
                return Err(());
            }
            (2, value & 0x7f7f)
        }
    };
    let mut offset = 0;
    if mode != state.iso_mode {
        out[..3].copy_from_slice(match mode {
            0 => b"\x1b(B",
            1 => b"\x1b(J",
            _ => b"\x1b$B",
        });
        offset = 3;
    }
    let length = emit(value, &mut out[offset..]);
    state.iso_mode = mode;
    Ok(offset + length)
}

pub(crate) fn finish(encoding: Encoding, state: &mut State, out: &mut [u8; 16]) -> Result<usize, ()> {
    #[cfg(feature = "asian")]
    if let Some(pending) = state.hkscs_pending {
        let length = emit(table(encoding).encoded(pending)?, out);
        state.hkscs_pending = None;
        return Ok(length);
    }
    let _ = (encoding, state, out);
    // Git's reencode_string_iconv() does not reset ISO-2022-JP at EOF.
    Ok(0)
}

/// Hoist descriptor selection and table layout out of the per-character loop.
/// Stateful codecs and the final few output bytes use the transactional path.
pub(crate) fn bulk(
    from: Encoding,
    to: Encoding,
    input: &[u8],
    output: &mut [u8],
    source: &mut State,
    destination: &mut State,
) -> (usize, usize) {
    if input.is_empty() || output.is_empty() {
        return (0, 0);
    }
    #[cfg(not(any(feature = "asian", feature = "legacy")))]
    let _ = source;
    #[cfg(not(feature = "asian"))]
    let _ = destination;
    #[cfg(feature = "asian")]
    if to == Encoding::Utf8 && from == Encoding::Iso2022Jp {
        return decode_iso2022_bulk(input, output, source);
    }
    #[cfg(feature = "asian")]
    if from == Encoding::Utf8 && to == Encoding::Iso2022Jp {
        return encode_iso2022_bulk(input, output, destination);
    }
    #[cfg(feature = "asian")]
    if from == Encoding::Utf8 && to == Encoding::Gb18030 {
        return encode_gb18030_bulk(input, output);
    }
    #[cfg(feature = "asian")]
    if from == Encoding::Gb18030 && to == Encoding::Utf8 {
        return decode_gb18030_bulk(input, output);
    }
    #[cfg(feature = "legacy")]
    if to == Encoding::Utf8 && matches!(from, Encoding::Windows1255 | Encoding::Windows1258) {
        let table = combining_table(from);
        let mut read = 0;
        let mut written = 0;
        while read < input.len() && output.len() - written >= 8 {
            let offset = 4 + source.combining * (4 + 256 * 12) + 4 + usize::from(input[read]) * 12;
            let first = u32_at(table, offset);
            if first == INVALID {
                break;
            }
            for scalar in [first, u32_at(table, offset + 4)] {
                if scalar != EMPTY {
                    let ch = char::from_u32(scalar).expect("generated composition tables contain Unicode scalars");
                    written += ch.encode_utf8(&mut output[written..]).len();
                }
            }
            source.combining = u32_at(table, offset + 8) as usize;
            read += 1;
        }
        return (read, written);
    }
    if from == Encoding::Utf8 && bulk_encode_supported(to) {
        let table = table(to).encoder();
        let mut read = 0;
        let mut written = 0;
        while read < input.len() && output.len() - written >= 4 {
            let Some((scalar, length)) = crate::fast::utf8_scalar(&input[read..]) else {
                break;
            };
            if scalar < 0x80 {
                output[written] = scalar as u8;
                written += 1;
                read += 1;
                continue;
            }
            let Ok(value) = table.encoded_scalar(scalar) else {
                break;
            };
            written += emit(value, &mut output[written..]);
            read += length;
        }
        return (read, written);
    }
    if to == Encoding::Utf8 && bulk_decode_supported(from) {
        let table = table(from);
        let mut read = 0;
        let mut written = 0;
        while read < input.len() && output.len() - written >= 8 {
            let byte = input[read];
            let ascii = byte < 0x80;
            #[cfg(feature = "asian")]
            let ascii = ascii && !(from == Encoding::ShiftJis && matches!(byte, 0x5c | 0x7e));
            if ascii {
                output[written] = byte;
                written += 1;
                read += 1;
                continue;
            }
            if let Some((scalar, consumed)) = table.scalar(&input[read..]) {
                written += emit_utf8(scalar, &mut output[written..]);
                read += consumed;
                continue;
            }
            let decoded = match table.decode(&input[read..]) {
                Ok(decoded) => decoded,
                Err(()) => break,
            };
            match decoded {
                Decoded::Scalar(ch, length) => {
                    written += ch.encode_utf8(&mut output[written..]).len();
                    read += length;
                }
                Decoded::Pair(first, second, length) => {
                    written += first.encode_utf8(&mut output[written..]).len();
                    written += second.encode_utf8(&mut output[written..]).len();
                    read += length;
                }
                _ => break,
            }
        }
        return (read, written);
    }
    (0, 0)
}

fn bulk_encode_supported(encoding: Encoding) -> bool {
    match encoding {
        #[cfg(feature = "asian")]
        Encoding::Iso2022Jp | Encoding::Big5Hkscs => false,
        _ => !crate::unicode::is_unicode(encoding),
    }
}

fn bulk_decode_supported(encoding: Encoding) -> bool {
    match encoding {
        #[cfg(feature = "asian")]
        Encoding::Iso2022Jp => false,
        #[cfg(feature = "legacy")]
        Encoding::Windows1255 | Encoding::Windows1258 => false,
        _ => !crate::unicode::is_unicode(encoding),
    }
}

#[cfg(feature = "asian")]
fn decode_iso2022_bulk(input: &[u8], output: &mut [u8], state: &mut State) -> (usize, usize) {
    let table = table(Encoding::EucJp);
    let mut read = 0;
    let mut written = 0;
    while read < input.len() && output.len() - written >= 4 {
        let first = input[read];
        if first >= 0x80 {
            break;
        }
        if first == 0x1b {
            if input.len() - read < 3 {
                break;
            }
            let mode = match &input[read + 1..read + 3] {
                b"(B" => Some(0),
                b"(J" => Some(1),
                b"$@" | b"$B" => Some(2),
                _ => None,
            };
            if let Some(mode) = mode {
                state.iso_mode = mode;
                read += 3;
                continue;
            }
            output[written] = first;
            written += 1;
            read += 1;
        } else if state.iso_mode != 2 || first < 0x21 || first == 0x7f {
            let ch = match (state.iso_mode, first) {
                (1, 0x5c) => '\u{a5}',
                (1, 0x7e) => '\u{203e}',
                _ => char::from(first),
            };
            written += ch.encode_utf8(&mut output[written..]).len();
            read += 1;
        } else {
            let Some(&second) = input.get(read + 1) else {
                break;
            };
            if !(0x21..=0x7e).contains(&second) {
                break;
            }
            let Some((scalar, _)) = table.scalar(&[first | 0x80, second | 0x80]) else {
                break;
            };
            written += emit_utf8(scalar, &mut output[written..]);
            read += 2;
        }
    }
    (read, written)
}

#[cfg(feature = "asian")]
fn encode_iso2022_bulk(input: &[u8], output: &mut [u8], state: &mut State) -> (usize, usize) {
    let table = table(Encoding::EucJp).encoder();
    let mut read = 0;
    let mut written = 0;
    while read < input.len() && output.len() - written >= 5 {
        let Some((scalar, length)) = crate::fast::utf8_scalar(&input[read..]) else {
            break;
        };
        let (mode, value) = match scalar {
            0xa5 => (1, 0x5c),
            0x203e => (1, 0x7e),
            _ if scalar < 0x80 => (0, scalar),
            _ => {
                let Ok(value) = table.encoded_scalar(scalar) else {
                    break;
                };
                if !(0xa1a1..=0xfefe).contains(&value) || value & 0xff < 0xa1 || value & 0xff > 0xfe {
                    break;
                }
                (2, value & 0x7f7f)
            }
        };
        if mode != state.iso_mode {
            output[written..written + 3].copy_from_slice(match mode {
                0 => b"\x1b(B",
                1 => b"\x1b(J",
                _ => b"\x1b$B",
            });
            written += 3;
            state.iso_mode = mode;
        }
        written += emit(value, &mut output[written..]);
        read += length;
    }
    (read, written)
}

#[cfg(feature = "asian")]
fn encode_gb18030_bulk(input: &[u8], output: &mut [u8]) -> (usize, usize) {
    let table = table(Encoding::Gb18030).encoder();
    let ranges = gb18030_encode_ranges();
    let mut read = 0;
    let mut written = 0;
    while read < input.len() && output.len() - written >= 4 {
        let Some((scalar, length)) = crate::fast::utf8_scalar(&input[read..]) else {
            break;
        };
        if scalar < 0x80 {
            output[written] = scalar as u8;
            written += 1;
        } else {
            // GB18030 maps every supplementary character through its final
            // four-byte range; it has no supplementary two-byte table entries.
            if scalar < 0x10000 {
                let value = u32_at(table.pages, scalar as usize * 4);
                if value != INVALID {
                    written += emit(value, &mut output[written..]);
                    read += length;
                    continue;
                }
            }
            let Some(mut pointer) = range_lookup(ranges, scalar) else {
                break;
            };
            output[written + 3] = (pointer % 10) as u8 + 0x30;
            pointer /= 10;
            output[written + 2] = (pointer % 126) as u8 + 0x81;
            pointer /= 126;
            output[written + 1] = (pointer % 10) as u8 + 0x30;
            output[written] = (pointer / 10) as u8 + 0x81;
            written += 4;
        }
        read += length;
    }
    (read, written)
}

#[cfg(feature = "asian")]
fn decode_gb18030_bulk(input: &[u8], output: &mut [u8]) -> (usize, usize) {
    let table = table(Encoding::Gb18030);
    let ranges = gb18030_decode_ranges();
    let mut read = 0;
    let mut written = 0;
    while read < input.len() && output.len() - written >= 4 {
        let first = input[read];
        if first < 0x80 {
            output[written] = first;
            written += 1;
            read += 1;
            continue;
        }
        if !(0x81..=0xfe).contains(&first) {
            break;
        }
        let Some(&second) = input.get(read + 1) else {
            break;
        };
        let (scalar, consumed) = if (0x30..=0x39).contains(&second) {
            if read + 4 > input.len()
                || !(0x81..=0xfe).contains(&input[read + 2])
                || !(0x30..=0x39).contains(&input[read + 3])
            {
                break;
            }
            let pointer =
                (((u32::from(first) - 0x81) * 10 + u32::from(second) - 0x30) * 126 + u32::from(input[read + 2]) - 0x81)
                    * 10
                    + u32::from(input[read + 3])
                    - 0x30;
            let Some(scalar) = range_lookup(ranges, pointer) else {
                break;
            };
            (scalar, 4)
        } else {
            // Every non-ASCII GB18030 lead has a dense second-byte page.
            let scalar = u32_at(table.0, 16 + (usize::from(first) + 1) * 1024 + usize::from(second) * 4);
            if scalar == INVALID {
                break;
            }
            (scalar, 2)
        };
        written += emit_utf8(scalar, &mut output[written..]);
        read += consumed;
    }
    (read, written)
}

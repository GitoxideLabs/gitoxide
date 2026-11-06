use crate::{Encoding, legacy::Decoded};

/// UTF-16/32 byte order after the first input unit (and optional BOM).
#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    pub(crate) started: bool,
    pub(crate) little: bool,
}

pub(crate) fn decode(encoding: Encoding, input: &[u8], state: &mut State) -> Result<Decoded, ()> {
    use Encoding::*;
    if input.is_empty() {
        return Ok(Decoded::Incomplete);
    }
    // Git only recognizes UTF-16LE-BOM as an input alias; the BE spelling is output-only.
    if encoding == Encoding::Utf16BeBom {
        return Err(());
    }
    if encoding == Utf8 {
        let len = match input[0] {
            0..=0x7f => return Ok(Decoded::Scalar(char::from(input[0]), 1)),
            0xc2..=0xdf => 2,
            0xe0..=0xef => 3,
            0xf0..=0xf4 => 4,
            _ => return Err(()),
        };
        if input.len() < len {
            // A malformed prefix is invalid even if more bytes could follow it.
            if std::str::from_utf8(input).is_err_and(|err| err.error_len().is_some()) {
                return Err(());
            }
            return Ok(Decoded::Incomplete);
        }
        let text = std::str::from_utf8(&input[..len]).map_err(|_| ())?;
        return Ok(Decoded::Scalar(text.chars().next().ok_or(())?, len));
    }
    let (width, detect, little) = match encoding {
        Utf16 | Utf16LeBom | Utf16BeBom => (2, true, cfg!(target_endian = "little")),
        Utf16Le => (2, false, true),
        Utf16Be => (2, false, false),
        Utf32 => (4, true, cfg!(target_endian = "little")),
        Utf32Le => (4, false, true),
        Utf32Be => (4, false, false),
        _ => return Err(()),
    };
    if input.len() < width {
        return Ok(Decoded::Incomplete);
    }
    if !state.started {
        state.little = little;
        if detect {
            let bom = if width == 2 {
                match &input[..2] {
                    [0xff, 0xfe] => Some(true),
                    [0xfe, 0xff] => Some(false),
                    _ => None,
                }
            } else {
                match &input[..4] {
                    [0xff, 0xfe, 0, 0] => Some(true),
                    [0, 0, 0xfe, 0xff] => Some(false),
                    _ => None,
                }
            };
            if let Some(little) = bom {
                state.little = little;
                state.started = true;
                return Ok(Decoded::Skip(width));
            }
        }
        state.started = true;
    }
    if width == 4 {
        let bytes = [input[0], input[1], input[2], input[3]];
        let scalar = if state.little {
            u32::from_le_bytes(bytes)
        } else {
            u32::from_be_bytes(bytes)
        };
        return char::from_u32(scalar).map(|ch| Decoded::Scalar(ch, 4)).ok_or(());
    }
    let unit = |bytes: &[u8]| {
        let bytes = [bytes[0], bytes[1]];
        if state.little {
            u16::from_le_bytes(bytes)
        } else {
            u16::from_be_bytes(bytes)
        }
    };
    let first = unit(input);
    if (0xd800..=0xdbff).contains(&first) {
        if input.len() < 4 {
            return Ok(Decoded::Incomplete);
        }
        let second = unit(&input[2..]);
        if !(0xdc00..=0xdfff).contains(&second) {
            return Err(());
        }
        let scalar = 0x10000 + ((u32::from(first) - 0xd800) << 10) + u32::from(second) - 0xdc00;
        return char::from_u32(scalar).map(|ch| Decoded::Scalar(ch, 4)).ok_or(());
    }
    char::from_u32(u32::from(first))
        .map(|ch| Decoded::Scalar(ch, 2))
        .ok_or(())
}

pub(crate) fn is_unicode(encoding: Encoding) -> bool {
    use Encoding::*;
    matches!(
        encoding,
        Utf8 | Utf16 | Utf16Le | Utf16Be | Utf16LeBom | Utf16BeBom | Utf32 | Utf32Le | Utf32Be
    )
}

pub(crate) fn bom(encoding: Encoding) -> &'static [u8] {
    use Encoding::*;
    match encoding {
        Utf16LeBom => &[0xff, 0xfe],
        Utf16BeBom => &[0xfe, 0xff],
        Utf16 if cfg!(target_endian = "little") => &[0xff, 0xfe],
        Utf16 => &[0xfe, 0xff],
        Utf32 if cfg!(target_endian = "little") => &[0xff, 0xfe, 0, 0],
        Utf32 => &[0, 0, 0xfe, 0xff],
        _ => &[],
    }
}

pub(crate) fn encode(encoding: Encoding, ch: char, output: &mut [u8; 16]) -> usize {
    use Encoding::*;
    if encoding == Utf8 {
        return ch.encode_utf8(output).len();
    }
    let little = matches!(encoding, Utf16Le | Utf16LeBom | Utf32Le)
        || cfg!(target_endian = "little") && matches!(encoding, Utf16 | Utf32);
    if matches!(encoding, Utf32 | Utf32Le | Utf32Be) {
        output[..4].copy_from_slice(&if little {
            (ch as u32).to_le_bytes()
        } else {
            (ch as u32).to_be_bytes()
        });
        return 4;
    }
    let mut units = [0; 2];
    let units = ch.encode_utf16(&mut units);
    for (unit, bytes) in units.iter().zip(output.as_chunks_mut::<2>().0) {
        bytes.copy_from_slice(&if little { unit.to_le_bytes() } else { unit.to_be_bytes() });
    }
    units.len() * 2
}

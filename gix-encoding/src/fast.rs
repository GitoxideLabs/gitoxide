//! Bulk paths stop before errors or incomplete characters, leaving diagnostics to the converter.

use crate::Encoding;

pub(crate) fn run(from: Encoding, to: Encoding, input: &[u8], output: &mut [u8]) -> (usize, usize) {
    use Encoding::*;
    if from == Utf8 {
        match to {
            Utf16Le => return utf8_unicode::<2, true>(input, output),
            Utf16Be => return utf8_unicode::<2, false>(input, output),
            Utf32Le => return utf8_unicode::<4, true>(input, output),
            Utf32Be => return utf8_unicode::<4, false>(input, output),
            Utf8 | Ascii | Iso8859_1 => {}
            _ => return (0, 0),
        }
        let input = &input[..input.len().min(output.len())];
        let text = match std::str::from_utf8(input) {
            Ok(text) => text,
            Err(err) => std::str::from_utf8(&input[..err.valid_up_to()]).expect("prefix validated by from_utf8"),
        };
        if to == Utf8 {
            output[..text.len()].copy_from_slice(text.as_bytes());
            return (text.len(), text.len());
        }
        let mut read = 0;
        let mut written = 0;
        let max = if to == Ascii { 0x7f } else { 0xff };
        for ch in text.chars() {
            if ch as u32 > max {
                break;
            }
            output[written] = ch as u8;
            written += 1;
            read += ch.len_utf8();
        }
        return (read, written);
    }
    if to != Utf8 {
        return (0, 0);
    }
    match from {
        Ascii | Iso8859_1 => {
            let mut read = 0;
            let mut written = 0;
            for &byte in input {
                if byte < 0x80 {
                    if written == output.len() {
                        break;
                    }
                    output[written] = byte;
                    written += 1;
                } else {
                    if from == Ascii || output.len() - written < 2 {
                        break;
                    }
                    output[written] = 0xc0 | (byte >> 6);
                    output[written + 1] = 0x80 | (byte & 0x3f);
                    written += 2;
                }
                read += 1;
            }
            (read, written)
        }
        Utf16Le | Utf16Be => {
            let mut read = 0;
            let mut written = 0;
            for bytes in input.as_chunks::<2>().0 {
                let unit = if from == Utf16Le {
                    u16::from_le_bytes([bytes[0], bytes[1]])
                } else {
                    u16::from_be_bytes([bytes[0], bytes[1]])
                };
                if (0xd800..=0xdfff).contains(&unit) {
                    break;
                }
                let len = utf8(u32::from(unit), &mut output[written..]);
                if len == 0 {
                    break;
                }
                written += len;
                read += 2;
            }
            (read, written)
        }
        Utf32Le | Utf32Be => {
            let mut read = 0;
            let mut written = 0;
            for &bytes in input.as_chunks::<4>().0 {
                let scalar = if from == Utf32Le {
                    u32::from_le_bytes(bytes)
                } else {
                    u32::from_be_bytes(bytes)
                };
                if scalar > 0x10ffff || (0xd800..=0xdfff).contains(&scalar) {
                    break;
                }
                let len = utf8(scalar, &mut output[written..]);
                if len == 0 {
                    break;
                }
                written += len;
                read += 4;
            }
            (read, written)
        }
        _ => (0, 0),
    }
}

fn utf8_unicode<const WIDTH: usize, const LITTLE: bool>(input: &[u8], output: &mut [u8]) -> (usize, usize) {
    let ascii_len = input.len().min(output.len() / WIDTH);
    if input[..ascii_len].is_ascii() {
        for (byte, unit) in input[..ascii_len].iter().zip(output.as_chunks_mut::<WIDTH>().0) {
            unit.fill(0);
            unit[if LITTLE { 0 } else { WIDTH - 1 }] = *byte;
        }
        return (ascii_len, ascii_len * WIDTH);
    }
    let mut read = 0;
    let mut written = 0;
    while read < input.len() && output.len() - written >= 4 {
        let Some((scalar, len)) = utf8_scalar(&input[read..]) else {
            break;
        };
        if WIDTH == 4 {
            output[written..written + 4].copy_from_slice(&if LITTLE {
                scalar.to_le_bytes()
            } else {
                scalar.to_be_bytes()
            });
            written += 4;
        } else if scalar < 0x10000 {
            let unit = scalar as u16;
            output[written..written + 2].copy_from_slice(&if LITTLE { unit.to_le_bytes() } else { unit.to_be_bytes() });
            written += 2;
        } else {
            let scalar = scalar - 0x10000;
            let high = 0xd800 | (scalar >> 10) as u16;
            let low = 0xdc00 | (scalar & 0x3ff) as u16;
            output[written..written + 2].copy_from_slice(&if LITTLE { high.to_le_bytes() } else { high.to_be_bytes() });
            output[written + 2..written + 4].copy_from_slice(&if LITTLE {
                low.to_le_bytes()
            } else {
                low.to_be_bytes()
            });
            written += 4;
        }
        read += len;
    }
    (read, written)
}

#[inline]
pub(crate) fn utf8_scalar(input: &[u8]) -> Option<(u32, usize)> {
    let first = input[0];
    if first < 0x80 {
        return Some((u32::from(first), 1));
    }
    let second = *input.get(1)?;
    if second & 0xc0 != 0x80 {
        return None;
    }
    if (0xc2..=0xdf).contains(&first) {
        return Some(((u32::from(first & 0x1f) << 6) | u32::from(second & 0x3f), 2));
    }
    let third = *input.get(2)?;
    if third & 0xc0 != 0x80 {
        return None;
    }
    if (0xe0..=0xef).contains(&first) {
        if first == 0xe0 && second < 0xa0 || first == 0xed && second >= 0xa0 {
            return None;
        }
        return Some((
            (u32::from(first & 0x0f) << 12) | (u32::from(second & 0x3f) << 6) | u32::from(third & 0x3f),
            3,
        ));
    }
    let fourth = *input.get(3)?;
    if !(0xf0..=0xf4).contains(&first)
        || fourth & 0xc0 != 0x80
        || first == 0xf0 && second < 0x90
        || first == 0xf4 && second >= 0x90
    {
        return None;
    }
    Some((
        (u32::from(first & 7) << 18)
            | (u32::from(second & 0x3f) << 12)
            | (u32::from(third & 0x3f) << 6)
            | u32::from(fourth & 0x3f),
        4,
    ))
}

fn utf8(scalar: u32, output: &mut [u8]) -> usize {
    if scalar < 0x80 && !output.is_empty() {
        output[0] = scalar as u8;
        1
    } else if scalar < 0x800 && output.len() >= 2 {
        output[0] = 0xc0 | (scalar >> 6) as u8;
        output[1] = 0x80 | (scalar & 0x3f) as u8;
        2
    } else if scalar < 0x10000 && output.len() >= 3 {
        output[0] = 0xe0 | (scalar >> 12) as u8;
        output[1] = 0x80 | ((scalar >> 6) & 0x3f) as u8;
        output[2] = 0x80 | (scalar & 0x3f) as u8;
        3
    } else if scalar >= 0x10000 && output.len() >= 4 {
        output[0] = 0xf0 | (scalar >> 18) as u8;
        output[1] = 0x80 | ((scalar >> 12) & 0x3f) as u8;
        output[2] = 0x80 | ((scalar >> 6) & 0x3f) as u8;
        output[3] = 0x80 | (scalar & 0x3f) as u8;
        4
    } else {
        0
    }
}

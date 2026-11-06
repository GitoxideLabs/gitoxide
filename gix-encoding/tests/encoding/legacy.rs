use gix_encoding::Encoding;
#[cfg(any(feature = "asian", feature = "western"))]
use gix_encoding::convert;
use gix_error::TestResult;

#[test]
fn native_names_preserve_distinct_character_sets() -> TestResult {
    for &encoding in Encoding::ALL {
        assert_eq!(
            encoding.name().parse::<Encoding>()?,
            encoding,
            "canonical names roundtrip"
        );
        assert_eq!(
            encoding.to_string(),
            encoding.name(),
            "Display uses the canonical Git spelling"
        );
        assert_eq!(
            Encoding::for_label(encoding.name().to_lowercase().as_bytes()),
            Some(encoding),
            "encoding names ignore ASCII case"
        );
    }
    assert_eq!(
        "latin-1".parse::<Encoding>()?,
        Encoding::Iso8859_1,
        "Git's fallback retains Latin-1"
    );
    #[cfg(feature = "asian")]
    assert_eq!(
        "SJIS".parse::<Encoding>()?,
        Encoding::ShiftJis,
        "SJIS selects the JIS standard"
    );
    #[cfg(feature = "asian")]
    assert_eq!(
        "WINDOWS-31J".parse::<Encoding>()?,
        Encoding::Cp932,
        "Windows Japanese is distinct"
    );
    for label in ["", "replacement", "x-user-defined", "UTF-8//IGNORE", "UTF-8//TRANSLIT"] {
        assert!(
            Encoding::for_label(label.as_bytes()).is_none(),
            "{label:?} is not a strict native encoding"
        );
    }
    Ok(())
}

#[cfg(feature = "western")]
#[test]
fn western_encodings_retain_distinct_euro_mappings() -> TestResult {
    for (encoding, byte) in [(Encoding::Windows1252, 0x80), (Encoding::Iso8859_15, 0xa4)] {
        assert_eq!(
            convert("€".as_bytes(), Encoding::Utf8, encoding)?,
            [byte],
            "{encoding} uses its own euro mapping, not Latin-1"
        );
        assert_eq!(
            convert(&[byte], encoding, Encoding::Utf8)?,
            "€".as_bytes(),
            "{encoding} decodes its euro mapping"
        );
    }
    Ok(())
}

#[cfg(feature = "asian")]
#[test]
fn hkscs_composition_survives_stream_completion() -> TestResult {
    assert_eq!(
        convert(&[0x88, 0x62], Encoding::Big5Hkscs, Encoding::Utf8)?,
        "Ê\u{304}".as_bytes(),
        "HKSCS expands its combining sequence"
    );
    assert_eq!(
        convert("Ê\u{304}".as_bytes(), Encoding::Utf8, Encoding::Big5Hkscs)?,
        [0x88, 0x62],
        "HKSCS combines the two Unicode scalars"
    );
    assert_eq!(
        convert("Ê".as_bytes(), Encoding::Utf8, Encoding::Big5Hkscs)?,
        [0x88, 0x66],
        "finishing preserves a buffered HKSCS character"
    );
    Ok(())
}

#[cfg(feature = "legacy")]
#[test]
fn vietnamese_composition_survives_stream_completion() -> TestResult {
    assert_eq!(
        convert(&[b'a', 0xec], Encoding::Windows1258, Encoding::Utf8)?,
        "á".as_bytes(),
        "iconv composes the Vietnamese acute accent"
    );
    assert_eq!(
        convert(b"a", Encoding::Windows1258, Encoding::Utf8)?,
        b"a",
        "finishing preserves a buffered Vietnamese base"
    );
    Ok(())
}

// GNU iconv is the mapping oracle used by Git on the compatibility platform.
// The library remains pure Rust; only these comparisons call the system API.
#[cfg(all(target_os = "linux", target_env = "gnu"))]
mod iconv {
    use std::ffi::{CString, c_char, c_void};

    use gix_encoding::{Converter, Encoding, convert::Status};
    use gix_error::TestResult;

    unsafe extern "C" {
        fn iconv_open(to: *const c_char, from: *const c_char) -> *mut c_void;
        fn iconv_close(descriptor: *mut c_void) -> i32;
        fn iconv(
            descriptor: *mut c_void,
            input: *mut *mut c_char,
            input_left: *mut usize,
            output: *mut *mut c_char,
            output_left: *mut usize,
        ) -> usize;
    }

    struct Oracle(*mut c_void);

    impl Oracle {
        fn new(from: Encoding, to: Encoding) -> Self {
            let from = CString::new(from.name()).expect("canonical labels have no NUL");
            let to = CString::new(to.name()).expect("canonical labels have no NUL");
            // SAFETY: both strings are NUL-terminated and alive during this call.
            let descriptor = unsafe { iconv_open(to.as_ptr(), from.as_ptr()) };
            assert_ne!(
                descriptor as usize,
                usize::MAX,
                "the glibc oracle supports the selected encoding"
            );
            Self(descriptor)
        }

        fn convert(&mut self, input: &[u8], output: &mut [u8; 64], flush: bool) -> Option<usize> {
            let mut source = input.as_ptr().cast::<c_char>().cast_mut();
            let mut remaining_input = input.len();
            let mut destination = output.as_mut_ptr().cast::<c_char>();
            let mut remaining_output = output.len();
            // SAFETY: the descriptor is uniquely borrowed and live. Input and output
            // pointer/count pairs refer to valid, disjoint buffers for the entire call.
            unsafe {
                iconv(
                    self.0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                if iconv(
                    self.0,
                    &mut source,
                    &mut remaining_input,
                    &mut destination,
                    &mut remaining_output,
                ) == usize::MAX
                {
                    return None;
                }
                if flush
                    && iconv(
                        self.0,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        &mut destination,
                        &mut remaining_output,
                    ) == usize::MAX
                {
                    return None;
                }
            }
            assert_eq!(remaining_input, 0, "the oracle consumes all input");
            Some(output.len() - remaining_output)
        }
    }

    impl Drop for Oracle {
        fn drop(&mut self) {
            // SAFETY: this wrapper owns the live descriptor and closes it once.
            unsafe { iconv_close(self.0) };
        }
    }

    fn check(oracle: &mut Oracle, input: &[u8], from: Encoding, to: Encoding) {
        let mut expected = [0; 64];
        let expected_len = oracle.convert(input, &mut expected, to.name() != "ISO-2022-JP");
        let mut actual = [0; 64];
        let actual_len = Converter::new(from, to)
            .convert(input, &mut actual, true)
            .ok()
            .map(|progress| {
                assert_eq!(
                    progress.status,
                    Status::Finished,
                    "the output fits in the oracle buffer"
                );
                assert_eq!(
                    progress.read,
                    input.len(),
                    "the converter consumes the entire test input"
                );
                progress.written
            });
        assert_eq!(actual_len, expected_len, "{from} -> {to}, input {input:02x?}");
        if let Some(length) = actual_len {
            assert_eq!(
                actual[..length],
                expected[..length],
                "{from} -> {to}, input {input:02x?}"
            );
        }
    }

    fn legacy_encodings() -> impl Iterator<Item = Encoding> {
        Encoding::ALL
            .iter()
            .copied()
            .filter(|encoding| !encoding.name().starts_with("UTF"))
    }

    #[test]
    fn all_single_bytes_match_iconv() {
        for encoding in legacy_encodings() {
            let mut oracle = Oracle::new(encoding, Encoding::Utf8);
            for byte in 0..=255 {
                check(&mut oracle, &[byte], encoding, Encoding::Utf8);
            }
        }
    }

    #[test]
    fn all_legacy_byte_pairs_match_iconv() {
        for encoding in [
            Encoding::Ascii,
            Encoding::Iso8859_1,
            #[cfg(feature = "western")]
            Encoding::Windows1252,
            #[cfg(feature = "western")]
            Encoding::Iso8859_15,
            #[cfg(feature = "asian")]
            Encoding::ShiftJis,
            #[cfg(feature = "asian")]
            Encoding::Cp932,
            #[cfg(feature = "asian")]
            Encoding::EucJp,
            #[cfg(feature = "asian")]
            Encoding::EucKr,
            #[cfg(feature = "asian")]
            Encoding::Cp949,
            #[cfg(feature = "asian")]
            Encoding::Gb2312,
            #[cfg(feature = "asian")]
            Encoding::Gbk,
            #[cfg(feature = "asian")]
            Encoding::Gb18030,
            #[cfg(feature = "asian")]
            Encoding::Big5,
            #[cfg(feature = "asian")]
            Encoding::Big5Hkscs,
            #[cfg(feature = "legacy")]
            Encoding::Windows1255,
            #[cfg(feature = "legacy")]
            Encoding::Windows1258,
        ] {
            let mut oracle = Oracle::new(encoding, Encoding::Utf8);
            for pair in 0..=u16::MAX {
                check(&mut oracle, &pair.to_be_bytes(), encoding, Encoding::Utf8);
            }
        }
    }

    #[cfg(feature = "asian")]
    #[test]
    fn extended_euc_jp_sequences_match_iconv() {
        let mut oracle = Oracle::new(Encoding::EucJp, Encoding::Utf8);
        for pair in 0..=u16::MAX {
            let [first, second] = pair.to_be_bytes();
            check(&mut oracle, &[0x8f, first, second], Encoding::EucJp, Encoding::Utf8);
        }
    }

    #[cfg(feature = "asian")]
    #[test]
    fn extended_gb18030_sequences_match_iconv() {
        let mut oracle = Oracle::new(Encoding::Gb18030, Encoding::Utf8);
        for pointer in 0..126 * 10 * 126 * 10 {
            let mut pointer = pointer;
            let fourth = (pointer % 10) as u8 + 0x30;
            pointer /= 10;
            let third = (pointer % 126) as u8 + 0x81;
            pointer /= 126;
            let second = (pointer % 10) as u8 + 0x30;
            pointer /= 10;
            let first = pointer as u8 + 0x81;
            check(
                &mut oracle,
                &[first, second, third, fourth],
                Encoding::Gb18030,
                Encoding::Utf8,
            );
        }
    }

    #[cfg(feature = "legacy")]
    #[test]
    fn windows_composition_matches_iconv() {
        for (encoding, alphabet) in [
            (
                Encoding::Windows1255,
                &[0xe0, 0xe1, 0xe9, 0xf9, 0xc4, 0xcc, 0xd1, 0xd2, b' ', b'a'][..],
            ),
            (
                Encoding::Windows1258,
                &[b'a', b'o', b'U', 0xc3, 0xd0, 0xec, 0xf2, 0xde, b' ', b'b'][..],
            ),
        ] {
            let mut oracle = Oracle::new(encoding, Encoding::Utf8);
            let mut random = 17u64;
            for _ in 0..4096 {
                let mut input = [0; 16];
                for byte in &mut input {
                    random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
                    *byte = alphabet[(random >> 32) as usize % alphabet.len()];
                }
                check(&mut oracle, &input, encoding, Encoding::Utf8);
            }
        }
    }

    #[cfg(feature = "asian")]
    #[test]
    fn hkscs_composition_matches_iconv() {
        let mut oracle = Oracle::new(Encoding::Utf8, Encoding::Big5Hkscs);
        for text in [
            "Ê\u{304}",
            "Ê\u{30c}",
            "ê\u{304}",
            "ê\u{30c}",
            "ÊêÊ",
            "Êx",
            "Ê\u{e0020}\u{304}",
        ] {
            check(&mut oracle, text.as_bytes(), Encoding::Utf8, Encoding::Big5Hkscs);
        }
    }

    #[test]
    fn representative_unicode_scalars_match_iconv() {
        for encoding in legacy_encodings() {
            let mut oracle = Oracle::new(Encoding::Utf8, encoding);
            // Cover all single-byte repertoires, JIS Roman distinctions, Vietnamese,
            // Hebrew presentation forms, Chinese, supplementary HKSCS and tag scalars.
            for scalar in (0..=0x500).chain(0x1e00..=0x20ff).chain(0xfb00..=0xffff).chain([
                0x4e00, 0x4e2d, 0x65e5, 0x672c, 0x20000, 0x1f642, 0xe0000, 0xe0020, 0x10ffff,
            ]) {
                if let Some(ch) = char::from_u32(scalar) {
                    check(
                        &mut oracle,
                        ch.encode_utf8(&mut [0; 4]).as_bytes(),
                        Encoding::Utf8,
                        encoding,
                    );
                }
            }
        }
    }

    #[cfg(feature = "asian")]
    #[test]
    fn iso2022_escape_sequences_and_transitions_match_iconv() {
        let mut decoder = Oracle::new(Encoding::Iso2022Jp, Encoding::Utf8);
        for first in 0..=127 {
            for second in 0..=127 {
                check(
                    &mut decoder,
                    &[0x1b, first, second],
                    Encoding::Iso2022Jp,
                    Encoding::Utf8,
                );
                check(
                    &mut decoder,
                    &[0x1b, b'$', b'B', first, second],
                    Encoding::Iso2022Jp,
                    Encoding::Utf8,
                );
            }
        }
        let mut encoder = Oracle::new(Encoding::Utf8, Encoding::Iso2022Jp);
        for text in ["日本語", "日本 A", "日本\n", "日本\0", "¥\n¥", "‾¥\\~日本\u{1b}"] {
            check(&mut encoder, text.as_bytes(), Encoding::Utf8, Encoding::Iso2022Jp);
        }
    }

    #[test]
    #[ignore = "exhaustive Unicode oracle sweep; run when regenerating mapping tables"]
    fn every_unicode_scalar_matches_iconv() -> TestResult {
        for encoding in legacy_encodings() {
            let mut oracle = Oracle::new(Encoding::Utf8, encoding);
            for scalar in 0..=0x10ffff {
                if let Some(ch) = char::from_u32(scalar) {
                    check(
                        &mut oracle,
                        ch.encode_utf8(&mut [0; 4]).as_bytes(),
                        Encoding::Utf8,
                        encoding,
                    );
                }
            }
        }
        Ok(())
    }
}

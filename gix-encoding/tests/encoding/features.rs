use gix_encoding::{Detection, Encoding, detect};
use gix_error::TestResult;

#[test]
fn labels_all_and_detection_candidates_match_enabled_features() -> TestResult {
    const {
        assert!(
            !cfg!(feature = "legacy") || cfg!(feature = "western"),
            "legacy includes the western group even without default features"
        );
    }
    // Each row starts with the canonical name, followed by aliases. Keep this
    // independent of ALL so an omitted codec cannot silently skip its own test.
    let groups = [
        (
            true,
            &[
                "UTF-8 UTF8",
                "UTF-16 UTF16",
                "UTF-16LE UTF16LE",
                "UTF-16BE UTF16BE",
                "UTF-16LE-BOM UTF16LE-BOM",
                "UTF-16BE-BOM UTF16BE-BOM",
                "UTF-32 UTF32",
                "UTF-32LE UTF32LE",
                "UTF-32BE UTF32BE",
                "ASCII US-ASCII ANSI_X3.4-1968 ANSI_X3.4-1986 ISO646-US ISO_646.IRV:1991 ISO-IR-6 IBM367 CP367 CSASCII US",
                "ISO-8859-1 ISO8859-1 ISO88591 ISO_8859-1 ISO_8859-1:1987 ISO-IR-100 LATIN1 LATIN-1 L1 IBM819 CP819 CSISOLATIN1",
            ][..],
        ),
        (
            cfg!(feature = "western"),
            &[
                "WINDOWS-1252 CP1252 MS-ANSI",
                "ISO-8859-15 ISO8859-15 ISO885915 ISO_8859-15 ISO_8859-15:1998 LATIN9",
            ][..],
        ),
        (
            cfg!(feature = "legacy"),
            &[
                "ISO-8859-2 ISO8859-2 ISO88592 ISO_8859-2 ISO_8859-2:1987 ISO-IR-101 LATIN2 L2 CSISOLATIN2",
                "ISO-8859-3 ISO8859-3 ISO88593 ISO_8859-3 ISO_8859-3:1988 ISO-IR-109 LATIN3 L3 CSISOLATIN3",
                "ISO-8859-4 ISO8859-4 ISO88594 ISO_8859-4 ISO_8859-4:1988 ISO-IR-110 LATIN4 L4 CSISOLATIN4",
                "ISO-8859-5 ISO8859-5 ISO88595 ISO_8859-5 ISO_8859-5:1988 ISO-IR-144 CYRILLIC CSISOLATINCYRILLIC",
                "ISO-8859-6 ISO8859-6 ISO88596 ISO_8859-6 ISO_8859-6:1987 ISO-IR-127 ECMA-114 ASMO-708 ARABIC CSISOLATINARABIC",
                "ISO-8859-7 ISO8859-7 ISO88597 ISO_8859-7 ISO_8859-7:1987 ISO-IR-126 ECMA-118 GREEK GREEK8 CSISOLATINGREEK",
                "ISO-8859-8 ISO8859-8 ISO88598 ISO_8859-8 ISO_8859-8:1988 ISO-IR-138 HEBREW CSISOLATINHEBREW",
                "ISO-8859-9 ISO8859-9 ISO88599 ISO_8859-9 ISO_8859-9:1989 ISO-IR-148 LATIN5 L5 CSISOLATIN5",
                "ISO-8859-10 ISO8859-10 ISO885910 ISO_8859-10 ISO_8859-10:1992 ISO-IR-157 LATIN6 L6 CSISOLATIN6",
                "ISO-8859-11 ISO8859-11 ISO885911",
                "ISO-8859-13 ISO8859-13 ISO885913 LATIN7 L7",
                "ISO-8859-14 ISO8859-14 ISO885914 ISO_8859-14 ISO_8859-14:1998 ISO-IR-199 LATIN8 L8 ISO-CELTIC",
                "ISO-8859-16 ISO8859-16 ISO885916 ISO_8859-16 ISO_8859-16:2001 ISO-IR-226 LATIN10 L10",
                "WINDOWS-1250 CP1250 MS-EE",
                "WINDOWS-1251 CP1251 MS-CYRL",
                "WINDOWS-1253 CP1253 MS-GREEK",
                "WINDOWS-1254 CP1254 MS-TURK",
                "WINDOWS-1255 CP1255 MS-HEBR",
                "WINDOWS-1256 CP1256 MS-ARAB",
                "WINDOWS-1257 CP1257 WINBALTRIM",
                "WINDOWS-1258 CP1258",
                "WINDOWS-874 CP874",
                "TIS-620 TIS620 TIS620-0 TIS620.2529-1 TIS620.2533-0",
                "KOI8-R KOI8R CSKOI8R",
                "KOI8-U KOI8U",
                "MACINTOSH MAC CSMACINTOSH",
                "MAC-CYRILLIC MACCYRILLIC",
                "CP437 IBM437 437 CSPC8CODEPAGE437",
                "CP850 IBM850 850 CSPC850MULTILINGUAL",
                "CP866 IBM866 866 CSIBM866",
                "CP1125",
            ][..],
        ),
        (
            cfg!(feature = "asian"),
            &[
                "SHIFT-JIS SHIFT_JIS SJIS CSSHIFTJIS MS_KANJI",
                "CP932 MS932 SJIS-OPEN SJIS-WIN WINDOWS-31J",
                "EUC-JP EUCJP CSEUCPKDFMTJAPANESE",
                "ISO-2022-JP ISO2022JP CSISO2022JP",
                "EUC-KR EUCKR CSEUCKR",
                "CP949 UHC MSCP949",
                "GB2312 EUC-CN EUCCN CN-GB CSGB2312",
                "GBK CP936 MS936 WINDOWS-936",
                "GB18030",
                "BIG5 BIG-5 BIG-FIVE BIGFIVE CN-BIG5 CP950",
                "BIG5-HKSCS BIG5HKSCS",
            ][..],
        ),
    ];
    let mut expected_names = Vec::new();
    for (enabled, rows) in groups {
        for row in rows {
            let canonical = row
                .split_ascii_whitespace()
                .next()
                .expect("each row has a canonical name");
            if enabled {
                expected_names.push(canonical);
            }
            for label in row.split_ascii_whitespace() {
                for spelling in [label.to_owned(), label.to_ascii_lowercase()] {
                    assert_eq!(
                        Encoding::for_label(spelling.as_bytes()).map(Encoding::name),
                        enabled.then_some(canonical),
                        "{spelling}: canonical names and aliases follow their feature group"
                    );
                    if enabled {
                        assert_eq!(
                            spelling.parse::<Encoding>()?.name(),
                            canonical,
                            "FromStr accepts enabled aliases with the same mapping"
                        );
                    } else {
                        assert!(
                            spelling.parse::<Encoding>().is_err(),
                            "FromStr rejects disabled labels instead of falling back: {spelling}"
                        );
                    }
                }
            }
        }
    }
    expected_names.sort_unstable();
    let mut actual_names: Vec<_> = Encoding::ALL.iter().map(|encoding| encoding.name()).collect();
    actual_names.sort_unstable();
    assert_eq!(actual_names, expected_names, "ALL contains exactly the enabled codecs");
    assert!(
        Encoding::ALL.windows(2).all(|pair| pair[0] < pair[1]),
        "ALL has no duplicates and retains declaration order"
    );

    // A single NUL is valid in every supported non-Unicode codec and UTF-8,
    // but cannot form a UTF-16/32 code unit or provide preferred text evidence.
    let Detection::Ambiguous { candidates } = detect(b"\0") else {
        panic!("a single NUL must not select a preferred interpretation");
    };
    let mut candidate_names: Vec<_> = candidates.iter().map(|encoding| encoding.name()).collect();
    candidate_names.sort_unstable();
    expected_names.retain(|name| !name.starts_with("UTF-") || *name == "UTF-8");
    assert_eq!(
        candidate_names, expected_names,
        "detection includes every enabled compatible codec and no disabled codec"
    );
    Ok(())
}

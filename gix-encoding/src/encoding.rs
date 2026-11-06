use std::{fmt, str::FromStr};

use gix_error::{Error, ErrorExt, Result};

/// Character encodings supported without an operating-system conversion library.
///
/// Names and character mappings follow Git and GNU `iconv`, rather than the web
/// platform's encoding standard: notably, ASCII, Latin-1, and Windows-1252 are
/// distinct, as are Shift-JIS and CP932, and EUC-KR and CP949.
///
/// # Feature availability
///
/// UTF-8/16/32, including byte-order-mark variants, ASCII, and ISO 8859-1 are always
/// available. Other variants and their labels require `western`, `legacy` (single-byte
/// encodings, including Thai/TIS-620), or `asian` (Japanese, Korean, and Chinese encodings).
/// See the [crate-level feature flags](crate#feature-flags) for each group's repertoire.
///
/// Disabled variants are absent from this enum and [`Self::ALL`]; their labels are
/// rejected by [`Self::for_label()`] and [`FromStr`]. Conversions using compressed
/// mapping tables may allocate on first use to lazily decompress and cache those tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Encoding {
    /// Unicode encoded as UTF-8, preserving a leading byte-order mark.
    Utf8,
    /// UTF-16 with a byte-order mark, using native byte order when writing.
    Utf16,
    /// Little-endian UTF-16, without implicit byte-order marks.
    Utf16Le,
    /// Big-endian UTF-16, without implicit byte-order marks.
    Utf16Be,
    /// Git's UTF-16LE-BOM spelling: read either byte order, write a little-endian mark.
    Utf16LeBom,
    /// Git's output-only UTF-16BE-BOM spelling.
    Utf16BeBom,
    /// UTF-32 with a byte-order mark, using native byte order when writing.
    Utf32,
    /// Little-endian UTF-32, without implicit byte-order marks.
    Utf32Le,
    /// Big-endian UTF-32, without implicit byte-order marks.
    Utf32Be,
    /// Seven-bit US ASCII.
    Ascii,
    /// ISO 8859-1 (Latin-1).
    Iso8859_1,
    /// ISO 8859-2 (Latin-2).
    #[cfg(feature = "legacy")]
    Iso8859_2,
    /// ISO 8859-3 (Latin-3).
    #[cfg(feature = "legacy")]
    Iso8859_3,
    /// ISO 8859-4 (Latin-4).
    #[cfg(feature = "legacy")]
    Iso8859_4,
    /// ISO 8859-5 (Cyrillic).
    #[cfg(feature = "legacy")]
    Iso8859_5,
    /// ISO 8859-6 (Arabic).
    #[cfg(feature = "legacy")]
    Iso8859_6,
    /// ISO 8859-7 (Greek).
    #[cfg(feature = "legacy")]
    Iso8859_7,
    /// ISO 8859-8 (Hebrew).
    #[cfg(feature = "legacy")]
    Iso8859_8,
    /// ISO 8859-9 (Latin-5).
    #[cfg(feature = "legacy")]
    Iso8859_9,
    /// ISO 8859-10 (Latin-6).
    #[cfg(feature = "legacy")]
    Iso8859_10,
    /// ISO 8859-11 (Thai).
    #[cfg(feature = "legacy")]
    Iso8859_11,
    /// ISO 8859-13 (Latin-7).
    #[cfg(feature = "legacy")]
    Iso8859_13,
    /// ISO 8859-14 (Latin-8).
    #[cfg(feature = "legacy")]
    Iso8859_14,
    /// ISO 8859-15 (Latin-9).
    #[cfg(feature = "western")]
    Iso8859_15,
    /// ISO 8859-16 (Latin-10).
    #[cfg(feature = "legacy")]
    Iso8859_16,
    /// Windows Central European code page 1250.
    #[cfg(feature = "legacy")]
    Windows1250,
    /// Windows Cyrillic code page 1251.
    #[cfg(feature = "legacy")]
    Windows1251,
    /// Windows Western European code page 1252.
    #[cfg(feature = "western")]
    Windows1252,
    /// Windows Greek code page 1253.
    #[cfg(feature = "legacy")]
    Windows1253,
    /// Windows Turkish code page 1254.
    #[cfg(feature = "legacy")]
    Windows1254,
    /// Windows Hebrew code page 1255, including `iconv`'s combining behavior.
    #[cfg(feature = "legacy")]
    Windows1255,
    /// Windows Arabic code page 1256.
    #[cfg(feature = "legacy")]
    Windows1256,
    /// Windows Baltic code page 1257.
    #[cfg(feature = "legacy")]
    Windows1257,
    /// Windows Vietnamese code page 1258, including `iconv`'s combining behavior.
    #[cfg(feature = "legacy")]
    Windows1258,
    /// Windows Thai code page 874.
    #[cfg(feature = "legacy")]
    Windows874,
    /// Thai Industrial Standard 620.
    #[cfg(feature = "legacy")]
    Tis620,
    /// KOI8-R Cyrillic.
    #[cfg(feature = "legacy")]
    Koi8R,
    /// KOI8-U Ukrainian.
    #[cfg(feature = "legacy")]
    Koi8U,
    /// Classic Macintosh Roman.
    #[cfg(feature = "legacy")]
    Macintosh,
    /// Classic Macintosh Cyrillic.
    #[cfg(feature = "legacy")]
    MacCyrillic,
    /// IBM PC code page 437.
    #[cfg(feature = "legacy")]
    Ibm437,
    /// IBM PC code page 850.
    #[cfg(feature = "legacy")]
    Ibm850,
    /// IBM PC code page 866.
    #[cfg(feature = "legacy")]
    Ibm866,
    /// Ukrainian PC code page 1125.
    #[cfg(feature = "legacy")]
    Cp1125,
    /// Shift-JIS, with the JIS Roman yen and overline mappings.
    #[cfg(feature = "asian")]
    ShiftJis,
    /// Windows Japanese code page 932.
    #[cfg(feature = "asian")]
    Cp932,
    /// EUC-JP, including JIS X 0212 three-byte sequences.
    #[cfg(feature = "asian")]
    EucJp,
    /// Stateful ISO-2022-JP.
    #[cfg(feature = "asian")]
    Iso2022Jp,
    /// EUC-KR, without CP949's additional Hangul syllables.
    #[cfg(feature = "asian")]
    EucKr,
    /// Windows Korean code page 949.
    #[cfg(feature = "asian")]
    Cp949,
    /// Simplified Chinese GB 2312.
    #[cfg(feature = "asian")]
    Gb2312,
    /// Simplified Chinese GBK.
    #[cfg(feature = "asian")]
    Gbk,
    /// Simplified Chinese GB 18030, including four-byte Unicode mappings.
    #[cfg(feature = "asian")]
    Gb18030,
    /// Traditional Chinese Big5.
    #[cfg(feature = "asian")]
    Big5,
    /// Big5 with the Hong Kong Supplementary Character Set.
    #[cfg(feature = "asian")]
    Big5Hkscs,
}

macro_rules! encodings {
    ($( $(#[$attr:meta])* $variant:ident => $name:literal $(, $alias:literal)*; )*) => {
        impl Encoding {
            /// Every encoding enabled by the active features, in declaration order.
            pub const ALL: &'static [Self] = &[$($(#[$attr])* Self::$variant),*];

            /// The canonical spelling accepted by Git.
            pub const fn name(self) -> &'static str {
                match self { $($(#[$attr])* Self::$variant => $name),* }
            }

            /// Parse an ASCII encoding label without allocating.
            ///
            /// Recognized `iconv` aliases and Git's `latin-1` fallback are
            /// case-insensitive. Unknown labels, conversion suffixes such as
            /// `//TRANSLIT`, and web-only aliases are rejected. Labels for encodings
            /// whose required feature is disabled also return `None`.
            pub fn for_label(label: &[u8]) -> Option<Self> {
                $($(#[$attr])* if label.eq_ignore_ascii_case($name.as_bytes())
                    $(|| label.eq_ignore_ascii_case($alias.as_bytes()))* {
                    return Some(Self::$variant);
                })*
                None
            }
        }
    };
}

encodings! {
    Utf8 => "UTF-8", "UTF8";
    Utf16 => "UTF-16", "UTF16";
    Utf16Le => "UTF-16LE", "UTF16LE";
    Utf16Be => "UTF-16BE", "UTF16BE";
    Utf16LeBom => "UTF-16LE-BOM", "UTF16LE-BOM";
    Utf16BeBom => "UTF-16BE-BOM", "UTF16BE-BOM";
    Utf32 => "UTF-32", "UTF32";
    Utf32Le => "UTF-32LE", "UTF32LE";
    Utf32Be => "UTF-32BE", "UTF32BE";
    Ascii => "ASCII", "US-ASCII", "ANSI_X3.4-1968", "ANSI_X3.4-1986", "ISO646-US", "ISO_646.IRV:1991", "ISO-IR-6", "IBM367", "CP367", "CSASCII", "US";
    Iso8859_1 => "ISO-8859-1", "ISO8859-1", "ISO88591", "ISO_8859-1", "ISO_8859-1:1987", "ISO-IR-100", "LATIN1", "LATIN-1", "L1", "IBM819", "CP819", "CSISOLATIN1";
    #[cfg(feature = "legacy")]
    Iso8859_2 => "ISO-8859-2", "ISO8859-2", "ISO88592", "ISO_8859-2", "ISO_8859-2:1987", "ISO-IR-101", "LATIN2", "L2", "CSISOLATIN2";
    #[cfg(feature = "legacy")]
    Iso8859_3 => "ISO-8859-3", "ISO8859-3", "ISO88593", "ISO_8859-3", "ISO_8859-3:1988", "ISO-IR-109", "LATIN3", "L3", "CSISOLATIN3";
    #[cfg(feature = "legacy")]
    Iso8859_4 => "ISO-8859-4", "ISO8859-4", "ISO88594", "ISO_8859-4", "ISO_8859-4:1988", "ISO-IR-110", "LATIN4", "L4", "CSISOLATIN4";
    #[cfg(feature = "legacy")]
    Iso8859_5 => "ISO-8859-5", "ISO8859-5", "ISO88595", "ISO_8859-5", "ISO_8859-5:1988", "ISO-IR-144", "CYRILLIC", "CSISOLATINCYRILLIC";
    #[cfg(feature = "legacy")]
    Iso8859_6 => "ISO-8859-6", "ISO8859-6", "ISO88596", "ISO_8859-6", "ISO_8859-6:1987", "ISO-IR-127", "ECMA-114", "ASMO-708", "ARABIC", "CSISOLATINARABIC";
    #[cfg(feature = "legacy")]
    Iso8859_7 => "ISO-8859-7", "ISO8859-7", "ISO88597", "ISO_8859-7", "ISO_8859-7:1987", "ISO-IR-126", "ECMA-118", "GREEK", "GREEK8", "CSISOLATINGREEK";
    #[cfg(feature = "legacy")]
    Iso8859_8 => "ISO-8859-8", "ISO8859-8", "ISO88598", "ISO_8859-8", "ISO_8859-8:1988", "ISO-IR-138", "HEBREW", "CSISOLATINHEBREW";
    #[cfg(feature = "legacy")]
    Iso8859_9 => "ISO-8859-9", "ISO8859-9", "ISO88599", "ISO_8859-9", "ISO_8859-9:1989", "ISO-IR-148", "LATIN5", "L5", "CSISOLATIN5";
    #[cfg(feature = "legacy")]
    Iso8859_10 => "ISO-8859-10", "ISO8859-10", "ISO885910", "ISO_8859-10", "ISO_8859-10:1992", "ISO-IR-157", "LATIN6", "L6", "CSISOLATIN6";
    #[cfg(feature = "legacy")]
    Iso8859_11 => "ISO-8859-11", "ISO8859-11", "ISO885911";
    #[cfg(feature = "legacy")]
    Iso8859_13 => "ISO-8859-13", "ISO8859-13", "ISO885913", "LATIN7", "L7";
    #[cfg(feature = "legacy")]
    Iso8859_14 => "ISO-8859-14", "ISO8859-14", "ISO885914", "ISO_8859-14", "ISO_8859-14:1998", "ISO-IR-199", "LATIN8", "L8", "ISO-CELTIC";
    #[cfg(feature = "western")]
    Iso8859_15 => "ISO-8859-15", "ISO8859-15", "ISO885915", "ISO_8859-15", "ISO_8859-15:1998", "LATIN9";
    #[cfg(feature = "legacy")]
    Iso8859_16 => "ISO-8859-16", "ISO8859-16", "ISO885916", "ISO_8859-16", "ISO_8859-16:2001", "ISO-IR-226", "LATIN10", "L10";
    #[cfg(feature = "legacy")]
    Windows1250 => "WINDOWS-1250", "CP1250", "MS-EE";
    #[cfg(feature = "legacy")]
    Windows1251 => "WINDOWS-1251", "CP1251", "MS-CYRL";
    #[cfg(feature = "western")]
    Windows1252 => "WINDOWS-1252", "CP1252", "MS-ANSI";
    #[cfg(feature = "legacy")]
    Windows1253 => "WINDOWS-1253", "CP1253", "MS-GREEK";
    #[cfg(feature = "legacy")]
    Windows1254 => "WINDOWS-1254", "CP1254", "MS-TURK";
    #[cfg(feature = "legacy")]
    Windows1255 => "WINDOWS-1255", "CP1255", "MS-HEBR";
    #[cfg(feature = "legacy")]
    Windows1256 => "WINDOWS-1256", "CP1256", "MS-ARAB";
    #[cfg(feature = "legacy")]
    Windows1257 => "WINDOWS-1257", "CP1257", "WINBALTRIM";
    #[cfg(feature = "legacy")]
    Windows1258 => "WINDOWS-1258", "CP1258";
    #[cfg(feature = "legacy")]
    Windows874 => "WINDOWS-874", "CP874";
    #[cfg(feature = "legacy")]
    Tis620 => "TIS-620", "TIS620", "TIS620-0", "TIS620.2529-1", "TIS620.2533-0";
    #[cfg(feature = "legacy")]
    Koi8R => "KOI8-R", "KOI8R", "CSKOI8R";
    #[cfg(feature = "legacy")]
    Koi8U => "KOI8-U", "KOI8U";
    #[cfg(feature = "legacy")]
    Macintosh => "MACINTOSH", "MAC", "CSMACINTOSH";
    #[cfg(feature = "legacy")]
    MacCyrillic => "MAC-CYRILLIC", "MACCYRILLIC";
    #[cfg(feature = "legacy")]
    Ibm437 => "CP437", "IBM437", "437", "CSPC8CODEPAGE437";
    #[cfg(feature = "legacy")]
    Ibm850 => "CP850", "IBM850", "850", "CSPC850MULTILINGUAL";
    #[cfg(feature = "legacy")]
    Ibm866 => "CP866", "IBM866", "866", "CSIBM866";
    #[cfg(feature = "legacy")]
    Cp1125 => "CP1125";
    #[cfg(feature = "asian")]
    ShiftJis => "SHIFT-JIS", "SHIFT_JIS", "SJIS", "CSSHIFTJIS", "MS_KANJI";
    #[cfg(feature = "asian")]
    Cp932 => "CP932", "MS932", "SJIS-OPEN", "SJIS-WIN", "WINDOWS-31J";
    #[cfg(feature = "asian")]
    EucJp => "EUC-JP", "EUCJP", "CSEUCPKDFMTJAPANESE";
    #[cfg(feature = "asian")]
    Iso2022Jp => "ISO-2022-JP", "ISO2022JP", "CSISO2022JP";
    #[cfg(feature = "asian")]
    EucKr => "EUC-KR", "EUCKR", "CSEUCKR";
    #[cfg(feature = "asian")]
    Cp949 => "CP949", "UHC", "MSCP949";
    #[cfg(feature = "asian")]
    Gb2312 => "GB2312", "EUC-CN", "EUCCN", "CN-GB", "CSGB2312";
    #[cfg(feature = "asian")]
    Gbk => "GBK", "CP936", "MS936", "WINDOWS-936";
    #[cfg(feature = "asian")]
    Gb18030 => "GB18030";
    #[cfg(feature = "asian")]
    Big5 => "BIG5", "BIG-5", "BIG-FIVE", "BIGFIVE", "CN-BIG5", "CP950";
    #[cfg(feature = "asian")]
    Big5Hkscs => "BIG5-HKSCS", "BIG5HKSCS";
}

impl fmt::Display for Encoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Encoding {
    type Err = Error;

    fn from_str(label: &str) -> Result<Self> {
        Self::for_label(label.as_bytes()).ok_or_else(|| {
            gix_error::message!("An encoding named '{label}' is not supported")
                .validation()
                .raise()
        })
    }
}

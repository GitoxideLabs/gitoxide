use std::cmp::Ordering;

use bstr::BStr;

pub fn cmp_ignore_ascii_case(a: &BStr, b: &BStr) -> Ordering {
    a.iter()
        .map(u8::to_ascii_lowercase)
        .cmp(b.iter().map(u8::to_ascii_lowercase))
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use bstr::ByteSlice;

    use super::cmp_ignore_ascii_case;

    #[test]
    fn basic_ascii_case_folding() {
        assert_eq!(
            cmp_ignore_ascii_case("FooBar".into(), "foobar".into()),
            Ordering::Equal,
            "ASCII case folding applies to every byte"
        );
    }

    #[test]
    fn no_advanced_unicode_folding() {
        assert_ne!(
            cmp_ignore_ascii_case("Masse".into(), "Maße".into()),
            Ordering::Equal,
            "non-ASCII characters are not folded"
        );
    }

    #[test]
    fn non_utf8_keys_use_the_same_case_folding() {
        assert_eq!(
            cmp_ignore_ascii_case(b"FOO\xff".as_bstr(), b"foo\xff".as_bstr()),
            Ordering::Equal,
            "invalid UTF-8 does not disable ASCII case folding"
        );
        assert_eq!(
            cmp_ignore_ascii_case(b"A".as_bstr(), b"B\xff".as_bstr()),
            cmp_ignore_ascii_case(b"a".as_bstr(), b"B\xff".as_bstr()),
            "equal keys must compare consistently with keys of any encoding"
        );
    }
}

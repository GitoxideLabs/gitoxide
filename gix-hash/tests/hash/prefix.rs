mod cmp_oid {
    use std::cmp::Ordering;

    use crate::hex_to_id;

    #[test]
    fn it_detects_inequality_sha1() {
        let prefix = gix_hash::Prefix::new(&hex_to_id("b920bbb055e1efb9080592a409d3975738b6efb3"), 7).unwrap();
        assert_eq!(
            prefix.cmp_oid(&hex_to_id("a920bbb055e1efb9080592a409d3975738b6efb3")),
            Ordering::Greater
        );
        assert_eq!(
            prefix.cmp_oid(&hex_to_id("b920bbf055e1efb9080592a409d3975738b6efb3")),
            Ordering::Less
        );
        assert_eq!(prefix.to_string(), "b920bbb");
    }

    #[test]
    #[cfg(feature = "sha256")]
    fn it_detects_inequality_sha256() {
        let prefix = gix_hash::Prefix::new(
            &hex_to_id("b920bbb055e1efb9080592a409d3975738b6efb338b6efb338b6efb338b6efb3"),
            7,
        )
        .unwrap();
        assert_eq!(
            prefix.cmp_oid(&hex_to_id(
                "a920bbb055e1efb9080592a409d3975738b6efb338b6efb338b6efb338b6efb3"
            )),
            Ordering::Greater
        );
        assert_eq!(
            prefix.cmp_oid(&hex_to_id(
                "b920bbf055e1efb9080592a409d3975738b6efb338b6efb338b6efb338b6efb3"
            )),
            Ordering::Less
        );
        assert_eq!(prefix.to_string(), "b920bbb");
    }

    #[test]
    #[cfg(all(feature = "sha1", feature = "sha256"))]
    fn it_detects_inequality_sha1_and_sha256() {
        let prefix_sha1 = gix_hash::Prefix::new(&hex_to_id("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"), 7).unwrap();
        let prefix_sha256 = gix_hash::Prefix::new(
            &hex_to_id("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            7,
        )
        .unwrap();
        assert_eq!(prefix_sha256.cmp(&prefix_sha1), Ordering::Greater);
        assert_eq!(prefix_sha1.to_string(), "bbbbbbb");
        assert_eq!(prefix_sha256.to_string(), "aaaaaaa");
    }

    #[test]
    fn it_detects_equality_sha1() {
        let id = hex_to_id("a920bbb055e1efb9080592a409d3975738b6efb3");
        let prefix = gix_hash::Prefix::new(&id, 6).unwrap();
        assert_eq!(prefix.cmp_oid(&id), Ordering::Equal);
        assert_eq!(
            prefix.cmp_oid(&hex_to_id("a920bbffffffffffffffffffffffffffffffffff")),
            Ordering::Equal
        );
        assert_eq!(prefix.to_string(), "a920bb");
    }

    #[test]
    #[cfg(feature = "sha256")]
    fn it_detects_equality_sha256() {
        let id = hex_to_id("a920bbb055e1efb9080592a409d3975738b6efb338b6efb338b6efb338b6efb3");
        let prefix = gix_hash::Prefix::new(&id, 6).unwrap();
        assert_eq!(prefix.cmp_oid(&id), Ordering::Equal);
        assert_eq!(
            prefix.cmp_oid(&hex_to_id("a920bbffffffffffffffffffffffffffffffffff")),
            Ordering::Equal
        );
        assert_eq!(prefix.to_string(), "a920bb");
    }
}

mod new {
    use std::cmp::Ordering;

    use gix_hash::{Kind, ObjectId};

    use crate::hex_to_id;

    #[test]
    fn various_valid_inputs_sha1() {
        let oid_hex = "abcdefabcdefabcdefabcdefabcdefabcdefabcd";
        let oid = hex_to_id(oid_hex);

        for hex_len in 4..oid.kind().len_in_hex() {
            let mut expected = String::from(&oid_hex[..hex_len]);
            let num_of_zeros = oid.kind().len_in_hex() - hex_len;
            expected.extend(std::iter::repeat_n('0', num_of_zeros));
            let prefix = gix_hash::Prefix::new(&oid, hex_len).unwrap();
            assert_eq!(prefix.as_oid().to_hex().to_string(), expected, "{hex_len}");
            assert_eq!(prefix.hex_len(), hex_len);
            assert_eq!(prefix.cmp_oid(&oid), Ordering::Equal);
        }
    }

    #[test]
    #[cfg(feature = "sha256")]
    fn various_valid_inputs_sha256() {
        let oid_hex = "abcdefabcdefabcdefabcdefabcdefabcdefabcdedabcdedabcdedabcdedabcd";
        let oid = hex_to_id(oid_hex);

        for hex_len in 4..oid.kind().len_in_hex() {
            let mut expected = String::from(&oid_hex[..hex_len]);
            let num_of_zeros = oid.kind().len_in_hex() - hex_len;
            expected.extend(std::iter::repeat_n('0', num_of_zeros));
            let prefix = gix_hash::Prefix::new(&oid, hex_len).unwrap();
            assert_eq!(prefix.as_oid().to_hex().to_string(), expected, "{hex_len}");
            assert_eq!(prefix.hex_len(), hex_len);
            assert_eq!(prefix.cmp_oid(&oid), Ordering::Equal);
        }
    }

    #[test]
    fn errors_if_hex_len_is_longer_than_oid_len_in_hex() {
        let kind = Kind::Sha1;
        assert!(matches!(
            gix_hash::Prefix::new(&ObjectId::null(kind), kind.len_in_hex() + 1),
            Err(gix_hash::prefix::Error::TooLong { .. })
        ));
    }

    #[test]
    fn errors_if_hex_len_is_too_short() {
        let kind = Kind::Sha1;
        assert!(matches!(
            gix_hash::Prefix::new(&ObjectId::null(kind), 3),
            Err(gix_hash::prefix::Error::TooShort { .. })
        ));
    }
}

mod try_from {
    use std::cmp::Ordering;

    use gix_hash::{prefix::from_hex::Error, Prefix};

    use crate::hex_to_id;

    #[test]
    fn id_6_chars() {
        let oid_hex = "abcdefabcdefabcdefabcdefabcdefabcdefabcd";
        let input = "abcdef";

        let expected = hex_to_id(oid_hex);
        let actual = Prefix::try_from(input).expect("No errors");
        assert_eq!(actual.cmp_oid(&expected), Ordering::Equal);
    }

    #[test]
    fn id_7_chars() {
        let oid_hex = "abcdefabcdefabcdefabcdefabcdefabcdefabcd";
        let input = "abcdefa";

        let expected = hex_to_id(oid_hex);
        let actual = Prefix::try_from(input).expect("No errors");
        assert_eq!(actual.cmp_oid(&expected), Ordering::Equal);
    }
    #[test]
    fn id_to_short() {
        let input = "ab";
        let expected = Error::TooShort { hex_len: 2 };
        let actual = Prefix::try_from(input).unwrap_err();
        assert_eq!(actual, expected);
    }

    #[test]
    #[cfg(all(not(feature = "sha256"), feature = "sha1"))]
    fn id_too_long() {
        let input = "abcdefabcdefabcdefabcdefabcdefabcdefabcd123123123123123123";
        let expected = Error::TooLong { hex_len: 58 };
        let actual = Prefix::try_from(input).unwrap_err();
        assert_eq!(actual, expected);
    }

    #[test]
    #[cfg(all(feature = "sha256", feature = "sha1"))]
    fn id_too_long() {
        let input = "abcdefabcdefabcdefabcdefabcdefabcdefabcd123123123123123123123123123123";
        let expected = Error::TooLong { hex_len: 70 };
        let actual = Prefix::try_from(input).unwrap_err();
        assert_eq!(actual, expected);
    }

    #[test]
    fn invalid_chars() {
        let input = "abcdfOsd";
        let expected = Error::Invalid;
        let actual = Prefix::try_from(input).unwrap_err();
        assert_eq!(actual, expected);
    }
}

mod from_hex_nonempty {
    use std::cmp::Ordering;

    use gix_hash::{prefix::from_hex::Error, Prefix};

    use crate::hex_to_id;

    #[test]
    fn id_6_chars() {
        let oid_hex = "abcdefabcdefabcdefabcdefabcdefabcdefabcd";
        let input = "abcdef";

        let expected = hex_to_id(oid_hex);
        let actual = Prefix::from_hex_nonempty(input).expect("No errors");
        assert_eq!(actual.cmp_oid(&expected), Ordering::Equal);
    }

    #[test]
    fn id_7_chars() {
        let oid_hex = "abcdefabcdefabcdefabcdefabcdefabcdefabcd";
        let input = "abcdefa";

        let expected = hex_to_id(oid_hex);
        let actual = Prefix::from_hex_nonempty(input).expect("No errors");
        assert_eq!(actual.cmp_oid(&expected), Ordering::Equal);
    }

    #[test]
    fn id_2_chars_and_less() {
        let oid_hex = "abcdefabcdefabcdefabcdefabcdefabcdefabcd";

        let oid = hex_to_id(oid_hex);
        let actual = Prefix::from_hex_nonempty("ab").expect("no errors");
        assert_eq!(actual.cmp_oid(&oid), Ordering::Equal);

        let actual = Prefix::from_hex_nonempty("a").expect("no errors");
        assert_eq!(actual.cmp_oid(&oid), Ordering::Equal);
    }

    #[test]
    fn id_empty() {
        let input = "";
        let expected = Error::TooShort { hex_len: 0 };
        let actual = Prefix::from_hex_nonempty(input).unwrap_err();
        assert_eq!(actual, expected);
    }

    #[test]
    #[cfg(all(not(feature = "sha256"), feature = "sha1"))]
    fn id_too_long() {
        let input = "abcdefabcdefabcdefabcdefabcdefabcdefabcd123123123123123123";
        let expected = Error::TooLong { hex_len: 58 };
        let actual = Prefix::from_hex_nonempty(input).unwrap_err();
        assert_eq!(actual, expected);
    }

    #[test]
    #[cfg(all(feature = "sha256", feature = "sha1"))]
    fn id_too_long() {
        let input = "abcdefabcdefabcdefabcdefabcdefabcdefabcd123123123123123123123123123123";
        let expected = Error::TooLong { hex_len: 70 };
        let actual = Prefix::from_hex_nonempty(input).unwrap_err();
        assert_eq!(actual, expected);
    }
}

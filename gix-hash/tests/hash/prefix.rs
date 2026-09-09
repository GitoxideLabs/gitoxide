mod hex_output {
    use crate::hex_to_id;

    #[test]
    fn writes_only_the_significant_hex_digits() {
        let prefix =
            gix_hash::Prefix::new(&hex_to_id("abcdefabcdefabcdefabcdefabcdefabcdefabcd"), 7).expect("valid prefix");
        let mut buf = [0; 7];
        assert_eq!(prefix.hex_to_buf(&mut buf), "abcdefa");
        assert_eq!(prefix.to_string(), "abcdefa");

        let mut written = Vec::new();
        prefix.write_hex_to(&mut written).expect("in-memory writes succeed");
        assert_eq!(written, b"abcdefa");
    }
}

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
        let len = 7;
        let prefix_sha1 = gix_hash::Prefix::new(&hex_to_id("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"), len).unwrap();
        let prefix_sha256 = gix_hash::Prefix::new(
            &hex_to_id("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            len,
        )
        .unwrap();
        assert_eq!(
            prefix_sha256.cmp(&prefix_sha1),
            Ordering::Greater,
            "prefixes of larger hashes are always larger"
        );
        assert_eq!(
            prefix_sha1.to_string(),
            prefix_sha256.to_string(),
            "even though they look the same"
        );
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

        let sha1 = hex_to_id("a920bbffffffffffffffffffffffffffffffffff");
        assert_eq!(
            prefix.cmp_oid(&sha1),
            Ordering::Equal,
            "cmp_oid specifies that it only looks at the prefix, ignoring everything past that.\
            This is why it compares against a sha1 as well, which shouldn't matter in practice."
        );

        let sha256 = hex_to_id("a920bbffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
        assert_eq!(prefix.cmp_oid(&sha256), Ordering::Equal);
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
        assert_eq!(
            gix_hash::Prefix::new(&ObjectId::null(kind), kind.len_in_hex() + 1)
                .unwrap_err()
                .to_string(),
            "An object of kind sha1 cannot be larger than 40 in hex, but 41 was requested"
        );
    }

    #[test]
    fn errors_if_hex_len_is_too_short() {
        let kind = Kind::Sha1;
        assert_eq!(
            gix_hash::Prefix::new(&ObjectId::null(kind), 3).unwrap_err().to_string(),
            "The minimum hex length of a short object id is 4, got 3"
        );
    }
}

mod try_from {
    use std::cmp::Ordering;

    use gix_hash::Prefix;

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
        let actual = Prefix::try_from(input).unwrap_err();
        assert_eq!(
            actual.to_string(),
            "The minimum hex length of a short object id is 4, got 2"
        );
    }

    #[test]
    #[cfg(all(not(feature = "sha256"), feature = "sha1"))]
    fn id_too_long() {
        let input = "abcdefabcdefabcdefabcdefabcdefabcdefabcd123123123123123123";
        let actual = Prefix::try_from(input).unwrap_err();
        assert_eq!(
            actual.to_string(),
            "An id cannot be larger than 40 chars in hex, but 58 was requested"
        );
    }

    #[test]
    fn id_always_too_long() {
        let input = "abcdefabcdefabcdefabcdefabcdefabcdefabcd123123123123123123123123123123";
        let actual = Prefix::try_from(input).unwrap_err();
        assert_eq!(
            actual.to_string(),
            format!(
                "An id cannot be larger than {} chars in hex, but 70 was requested",
                gix_hash::Kind::longest().len_in_hex()
            )
        );
    }

    #[test]
    fn invalid_chars() {
        let input = "abcdfOsd";
        let actual = Prefix::try_from(input).unwrap_err();
        assert_eq!(actual.to_string(), "Invalid hex character");
    }
}

mod from_hex_nonempty {
    use std::cmp::Ordering;

    use gix_hash::Prefix;

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
        let actual = Prefix::from_hex_nonempty(input).unwrap_err();
        assert_eq!(
            actual.to_string(),
            "The minimum hex length of a short object id is 4, got 0"
        );
    }

    #[test]
    #[cfg(all(not(feature = "sha256"), feature = "sha1"))]
    fn id_too_long() {
        let input = "abcdefabcdefabcdefabcdefabcdefabcdefabcd123123123123123123";
        let actual = Prefix::from_hex_nonempty(input).unwrap_err();
        assert_eq!(
            actual.to_string(),
            "An id cannot be larger than 40 chars in hex, but 58 was requested"
        );
    }

    #[test]
    fn id_always_too_long() {
        let input = "abcdefabcdefabcdefabcdefabcdefabcdefabcd123123123123123123123123123123";
        let actual = Prefix::from_hex_nonempty(input).unwrap_err();
        assert_eq!(
            actual.to_string(),
            format!(
                "An id cannot be larger than {} chars in hex, but 70 was requested",
                gix_hash::Kind::longest().len_in_hex()
            )
        );
    }
}

mod reverse_hex {
    use std::cmp::Ordering;

    use gix_hash::{ChangeId, Prefix};

    #[test]
    fn matches_change_ids_at_odd_nibbles() -> gix_testtools::Result {
        let full = "zzyxwvutsrqponmlkzyxwvutsrqponmlkzyxwvut";
        let change_id = ChangeId::from_reverse_hex(full.as_bytes())?;
        let prefix = Prefix::from_reverse_hex_nonempty("zzy")?;

        assert_eq!(prefix.cmp_oid(&change_id), Ordering::Equal, "zzy matches zzyx...");
        assert_eq!(prefix.to_reverse_hex().to_string(), "zzy");
        assert_eq!(Prefix::from(change_id).to_reverse_hex().to_string(), full);
        Ok(())
    }

    #[test]
    fn validates_reverse_hex_like_forward_hex() -> gix_testtools::Result {
        assert_eq!(
            Prefix::from_reverse_hex("zzy")
                .expect_err("three digits are below the safe minimum")
                .to_string(),
            "The minimum hex length of a short object id is 4, got 3"
        );
        let prefix = Prefix::from_reverse_hex("ZZYX")?;
        assert_eq!(
            prefix.to_reverse_hex().to_string(),
            "zzyx",
            "output is canonical lowercase"
        );
        assert_eq!(
            Prefix::from_reverse_hex_nonempty("jj")
                .expect_err("j is outside the reverse alphabet")
                .to_string(),
            "Invalid hex character"
        );
        Ok(())
    }
}

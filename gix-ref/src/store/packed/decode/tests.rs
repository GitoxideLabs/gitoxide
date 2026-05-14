type Result = std::result::Result<(), Box<dyn std::error::Error>>;

mod reference {
    use super::Result;
    use crate::{
        FullNameRef,
        store_impl::{packed, packed::decode},
    };

    const HASH_KIND: gix_hash::Kind = gix_hash::Kind::Sha1;

    /// Convert a hexadecimal hash into its corresponding `ObjectId` or _panic_.
    fn hex_to_id(hex: &str) -> gix_hash::ObjectId {
        gix_hash::ObjectId::from_hex(hex.as_bytes()).expect("40 bytes hex")
    }

    #[test]
    fn invalid() {
        let mut input = b"# what looks like a comment".as_slice();
        assert!(decode::reference(&mut input, HASH_KIND).is_err());
        let mut input = b"^e9cdc958e7ce2290e2d7958cdb5aa9323ef35d37\n".as_slice();
        assert!(decode::reference(&mut input, HASH_KIND).is_err(), "lonely peel");
        let mut input =
            b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa refs/heads/main\n".as_slice();
        assert!(
            decode::reference(&mut input, gix_hash::Kind::Sha1).is_err(),
            "sha1 refs reject sha256-sized ids"
        );
        let mut input = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa refs/heads/main\n".as_slice();
        assert!(
            decode::reference(&mut input, gix_hash::Kind::Sha256).is_err(),
            "sha256 refs reject sha1-sized ids"
        );
    }

    #[test]
    fn uppercase_hex() -> Result {
        let mut input: &[u8] = b"D53C4B0F91F1B29769C9430F2D1C0BCAB1170C75 refs/heads/uppercase
^E9CDC958E7CE2290E2D7958CDB5AA9323EF35D37\n";
        let parsed = decode::reference(&mut input, HASH_KIND).unwrap();

        assert!(input.is_empty(), "exhausted");
        assert_eq!(parsed.name, FullNameRef::new_unchecked("refs/heads/uppercase".into()));
        assert_eq!(parsed.target(), hex_to_id("d53c4b0f91f1b29769c9430f2d1c0bcab1170c75"));
        assert_eq!(parsed.object(), hex_to_id("e9cdc958e7ce2290e2d7958cdb5aa9323ef35d37"));
        Ok(())
    }

    #[test]
    fn sha256_hex() -> Result {
        let mut input: &[u8] = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa refs/heads/main\n";
        let parsed = decode::reference(&mut input, gix_hash::Kind::Sha256).unwrap();

        assert!(input.is_empty(), "exhausted");
        assert_eq!(parsed.name, FullNameRef::new_unchecked("refs/heads/main".into()));
        assert_eq!(parsed.target().kind(), gix_hash::Kind::Sha256);
        Ok(())
    }

    #[test]
    fn two_refs_in_a_row() -> Result {
        let mut input: &[u8] = b"d53c4b0f91f1b29769c9430f2d1c0bcab1170c75 refs/heads/alternates-after-packs-and-loose
^e9cdc958e7ce2290e2d7958cdb5aa9323ef35d37\neaae9c1bc723209d793eb93f5587fa2604d5cd92 refs/heads/avoid-double-lookup\n";
        let parsed = decode::reference(&mut input, HASH_KIND).unwrap();

        assert_eq!(
            parsed,
            packed::Reference {
                name: FullNameRef::new_unchecked("refs/heads/alternates-after-packs-and-loose".into()),
                target: "d53c4b0f91f1b29769c9430f2d1c0bcab1170c75".into(),
                object: Some("e9cdc958e7ce2290e2d7958cdb5aa9323ef35d37".into())
            }
        );
        assert_eq!(parsed.target(), hex_to_id("d53c4b0f91f1b29769c9430f2d1c0bcab1170c75"));
        assert_eq!(parsed.object(), hex_to_id("e9cdc958e7ce2290e2d7958cdb5aa9323ef35d37"));

        let parsed = decode::reference(&mut input, HASH_KIND).unwrap();
        assert!(input.is_empty(), "exhausted");
        assert_eq!(
            parsed.name,
            FullNameRef::new_unchecked("refs/heads/avoid-double-lookup".into())
        );
        assert_eq!(parsed.target, "eaae9c1bc723209d793eb93f5587fa2604d5cd92");
        assert!(parsed.object.is_none());
        Ok(())
    }
}

mod header {
    use gix_object::bstr::ByteSlice;

    use super::Result;
    use crate::store_impl::packed::{
        decode,
        decode::{Header, Peeled},
    };

    #[test]
    fn invalid() {
        let mut input = b"# some user comment".as_slice();
        assert!(decode::header(&mut input).is_err(), "something the user put there");
        assert_eq!(input[0], b'#', "it consumed nothing");
        let mut input = b"# pack-refs: ".as_slice();
        assert!(decode::header(&mut input).is_err(), "looks right but isn't");
        let mut input = b" # pack-refs with: ".as_slice();
        assert!(decode::header(&mut input).is_err(), "does not start with #");
    }

    #[test]
    fn valid_fully_peeled_stored() -> Result {
        let mut input: &[u8] = b"# pack-refs with: peeled fully-peeled sorted  \nsomething else";
        let header = decode::header(&mut input).expect("valid input");

        assert_eq!(input.as_bstr(), "something else", "remainder starts after newline");
        assert_eq!(
            header,
            Header {
                peeled: Peeled::Fully,
                sorted: true
            }
        );
        Ok(())
    }

    #[test]
    fn valid_peeled_unsorted() -> Result {
        let mut input: &[u8] = b"# pack-refs with: peeled\n";
        let header = decode::header(&mut input).unwrap();

        assert!(input.is_empty());
        assert_eq!(
            header,
            Header {
                peeled: Peeled::Partial,
                sorted: false
            }
        );
        Ok(())
    }

    #[test]
    fn valid_empty() -> Result {
        let mut input: &[u8] = b"# pack-refs with: \n";
        let header = decode::header(&mut input).unwrap();

        assert!(input.is_empty());
        assert_eq!(
            header,
            Header {
                peeled: Peeled::Unspecified,
                sorted: false
            }
        );
        Ok(())
    }
}

/// Because the `TryFrom` implementations don't return proper errors
/// on failure
#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("")]
    TryFromError,
}

macro_rules! round_trip {
    ($owned:ty, $borrowed:ty, $( $files:literal ), +) => {
        #[test]
        fn round_trip() -> Result<(), Box<dyn std::error::Error>> {
            use std::convert::TryFrom;
            use std::io::Write;
            use crate::fixture_bytes;
            use gix_object::{ObjectRef, Object, WriteTo};
            use bstr::ByteSlice;

            for input_name in &[
                $( $files ),*
            ] {
                let input = fixture_bytes(input_name);
                // Test the parse->borrowed->owned->write chain for an object kind
                let mut output = Vec::new();
                let item = <$borrowed>::from_bytes(&input)?;
                item.write_to(&mut output)?;
                assert_eq!(output.as_bstr(), input.as_bstr(), "borrowed: {input_name}");

                let item: $owned = <$owned>::try_from(item)?;
                output.clear();
                item.write_to(&mut output)?;
                assert_eq!(output.as_bstr(), input.as_bstr());

                // Test the parse->borrowed->owned->write chain for the top-level objects
                let item = ObjectRef::from(<$borrowed>::from_bytes(&input)?);
                output.clear();
                item.write_to(&mut output)?;
                assert_eq!(output.as_bstr(), input.as_bstr(), "object-ref");

                let item: Object = Object::try_from(item)?;
                output.clear();
                item.write_to(&mut output)?;
                assert_eq!(output.as_bstr(), input.as_bstr(), "owned");

                // Test the loose serialisation -> parse chain for an object kind
                let item = <$borrowed>::from_bytes(&input)?;
                // serialise a borowed item to a tagged loose object
                output.clear();
                {
                    let w = &mut output;
                    w.write_all(&item.loose_header())?;
                    item.write_to(w)?;
                    let parsed = ObjectRef::from_loose(&output)?;
                    let item2 = <$borrowed>::try_from(parsed).or(Err(super::Error::TryFromError))?;
                    assert_eq!(item2, item, "object-ref loose: {input_name} {:?}\n{:?}", output.as_bstr(), input.as_bstr());
                }

                let item: $owned = <$owned>::try_from(item)?;
                // serialise an owned to a tagged loose object
                output.clear();
                let w = &mut output;
                w.write_all(&item.loose_header())?;
                item.write_to(w)?;
                let parsed = ObjectRef::from_loose(&output)?;
                let parsed_borrowed = <$borrowed>::try_from(parsed).or(Err(super::Error::TryFromError))?;
                let item2: $owned = <$owned>::try_from(parsed_borrowed).or(Err(super::Error::TryFromError))?;
                assert_eq!(item2, item, "object-ref loose owned: {input_name} {:?}\n{:?}", output.as_bstr(), input.as_bstr());
            }
            Ok(())
        }
    };
}

mod tag {
    round_trip!(
        gix_object::Tag,
        gix_object::TagRef,
        "tag/empty_missing_nl.txt",
        "tag/empty.txt",
        "tag/no-tagger.txt",
        "tag/whitespace.txt",
        "tag/with-newlines.txt",
        "tag/signed.txt"
    );
}

mod commit {
    round_trip!(
        gix_object::Commit,
        gix_object::CommitRef,
        "commit/email-with-space.txt",
        "commit/signed-whitespace.txt",
        "commit/two-multiline-headers.txt",
        "commit/mergetag.txt",
        "commit/merge.txt",
        "commit/signed.txt",
        "commit/signed-singleline.txt",
        "commit/signed-with-encoding.txt",
        "commit/unsigned.txt",
        "commit/whitespace.txt",
        "commit/with-encoding.txt",
        "commit/subtle.txt"
    );
}

mod tree {
    use gix_object::{tree, tree::EntryKind, WriteTo};

    #[test]
    fn write_to_does_not_validate() {
        let mut tree = gix_object::Tree::empty();
        tree.entries.push(tree::Entry {
            mode: EntryKind::Blob.into(),
            filename: "".into(),
            oid: gix_hash::Kind::Sha1.null(),
        });
        tree.entries.push(tree::Entry {
            mode: EntryKind::Tree.into(),
            filename: "something\nwith\newlines\n".into(),
            oid: gix_hash::ObjectId::empty_tree(gix_hash::Kind::Sha1),
        });
        tree.write_to(&mut std::io::sink())
            .expect("write succeeds, no validation is performed");
    }

    #[test]
    fn write_to_does_not_allow_separator() {
        let mut tree = gix_object::Tree::empty();
        tree.entries.push(tree::Entry {
            mode: EntryKind::Blob.into(),
            filename: "hi\0ho".into(),
            oid: gix_hash::Kind::Sha1.null(),
        });
        let err = tree.write_to(&mut std::io::sink()).unwrap_err();
        assert_eq!(
            err.to_string(),
            r#"Nullbytes are invalid in file paths as they are separators: "hi\0ho""#
        );
    }

    round_trip!(gix_object::Tree, gix_object::TreeRef, "tree/everything.tree");
}

mod blob {
    // It doesn't matter which data we use - it's not interpreted.
    round_trip!(gix_object::Blob, gix_object::BlobRef, "tree/everything.tree");
}

mod loose_header {
    use bstr::ByteSlice;
    use gix_object::{decode, encode, Kind};

    #[test]
    fn round_trip() -> Result<(), Box<dyn std::error::Error>> {
        for (kind, size, expected) in &[
            (Kind::Tree, 1234, "tree 1234\0".as_bytes()),
            (Kind::Blob, 0, b"blob 0\0"),
            (Kind::Commit, 24241, b"commit 24241\0"),
            (Kind::Tag, 9999999999, b"tag 9999999999\0"),
        ] {
            let buf = encode::loose_header(*kind, *size);
            assert_eq!(buf.as_bstr(), expected.as_bstr());
            let (actual_kind, actual_size, actual_read) = decode::loose_header(&buf)?;
            assert_eq!(actual_kind, *kind);
            assert_eq!(actual_size, *size);
            assert_eq!(actual_read, buf.len());
        }
        Ok(())
    }
}

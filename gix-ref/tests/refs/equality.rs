use gix_ref::{
    FullName, FullNameRef, PartialName, Target,
    bstr::{BStr, BString, ByteSlice},
};

macro_rules! assert_natural_equality {
    ($value:ident, $matching:literal, $different:literal) => {{
        let matching = $matching;
        let matching_string = matching.to_owned();
        let matching_bstr = matching.as_bytes().as_bstr();
        let matching_bstring: BString = matching.as_bytes().into();

        assert_eq!($value, matching, "the value matches str");
        assert_eq!(matching, $value, "str comparison is symmetric");
        assert_eq!($value, matching_string, "the value matches String");
        assert_eq!(matching_string, $value, "String comparison is symmetric");
        assert_eq!($value, matching_bstr, "the value matches BStr");
        assert_eq!(matching_bstr, $value, "BStr comparison is symmetric");
        assert_eq!($value, matching_bstring, "the value matches BString");
        assert_eq!(matching_bstring, $value, "BString comparison is symmetric");

        let different = $different;
        let different_string = different.to_owned();
        let different_bstr = different.as_bytes().as_bstr();
        let different_bstring: BString = different.as_bytes().into();

        assert_ne!($value, different, "the value differs from str");
        assert_ne!(different, $value, "str inequality is symmetric");
        assert_ne!($value, different_string, "the value differs from String");
        assert_ne!(different_string, $value, "String inequality is symmetric");
        assert_ne!($value, different_bstr, "the value differs from BStr");
        assert_ne!(different_bstr, $value, "BStr inequality is symmetric");
        assert_ne!($value, different_bstring, "the value differs from BString");
        assert_ne!(different_bstring, $value, "BString inequality is symmetric");
    }};
}

macro_rules! assert_binary_equality {
    ($value:ident, $matching:expr, $different:expr) => {{
        let matching: &BStr = $matching;
        let matching_owned = matching.to_owned();
        assert_eq!($value, matching, "the value matches non-UTF-8 BStr");
        assert_eq!(matching, $value, "non-UTF-8 BStr comparison is symmetric");
        assert_eq!($value, matching_owned, "the value matches non-UTF-8 BString");
        assert_eq!(matching_owned, $value, "non-UTF-8 BString comparison is symmetric");

        let different: &BStr = $different;
        let different_owned = different.to_owned();
        assert_ne!($value, different, "the value differs from BStr");
        assert_ne!(different, $value, "BStr inequality is symmetric");
        assert_ne!($value, different_owned, "the value differs from BString");
        assert_ne!(different_owned, $value, "BString inequality is symmetric");
    }};
}

macro_rules! assert_reference_name_equality {
    ($reference:ident, $name:ident) => {{
        let name_ref: &FullNameRef = $name.as_ref();
        assert_eq!($reference, $name, "the reference matches its owned name");
        assert_eq!($reference, name_ref, "the reference matches its borrowed name");

        let matching = "refs/heads/main";
        assert_eq!($reference, matching, "the reference matches str");
        assert_eq!($reference, matching.to_owned(), "the reference matches String");
        assert_eq!($reference, matching.as_bytes().as_bstr(), "the reference matches BStr");
        assert_eq!($reference, BString::from(matching), "the reference matches BString");

        let different = "refs/heads/other";
        assert_ne!($reference, different, "the reference differs from str");
        assert_ne!($reference, different.to_owned(), "the reference differs from String");
        assert_ne!(
            $reference,
            different.as_bytes().as_bstr(),
            "the reference differs from BStr"
        );
        assert_ne!(
            $reference,
            BString::from(different),
            "the reference differs from BString"
        );
    }};
}

#[test]
fn name_types_compare_with_text_and_byte_strings() -> gix_testtools::Result {
    let full = FullName::try_from("refs/heads/main")?;
    let full_ref: &FullNameRef = full.as_ref();
    assert_eq!(full, full_ref, "owned and borrowed full names match");
    assert_eq!(full_ref, full, "full-name comparison is symmetric");
    assert_natural_equality!(full, "refs/heads/main", "refs/heads/other");
    assert_natural_equality!(full_ref, "refs/heads/main", "refs/heads/other");

    let partial = PartialName::try_from("heads/main")?;
    let partial_ref = partial.as_ref();
    assert_eq!(partial, partial_ref, "owned and borrowed partial names match");
    assert_eq!(partial_ref, partial, "partial-name comparison is symmetric");
    assert_natural_equality!(partial, "heads/main", "heads/other");
    assert_natural_equality!(partial_ref, "heads/main", "heads/other");

    let namespace = gix_ref::namespace::expand("foo")?;
    assert_natural_equality!(namespace, "refs/namespaces/foo/", "refs/namespaces/bar/");
    Ok(())
}

#[test]
fn names_compare_as_exact_bytes() -> gix_testtools::Result {
    let full_bytes = b"refs/heads/\xff".as_bstr();
    let full = FullName::try_from(full_bytes)?;
    let full_ref: &FullNameRef = full.as_ref();
    assert_binary_equality!(full, full_bytes, b"refs/heads/other".as_bstr());
    assert_binary_equality!(full_ref, full_bytes, b"refs/heads/other".as_bstr());

    let partial_bytes = b"heads/\xff".as_bstr();
    let partial = PartialName::try_from(partial_bytes.to_owned())?;
    let partial_ref = partial.as_ref();
    assert_binary_equality!(partial, partial_bytes, b"heads/other".as_bstr());
    assert_binary_equality!(partial_ref, partial_bytes, b"heads/other".as_bstr());

    let namespace = gix_ref::namespace::expand(b"\xff".as_bstr())?;
    assert_binary_equality!(
        namespace,
        b"refs/namespaces/\xff/".as_bstr(),
        b"refs/namespaces/other/".as_bstr()
    );
    Ok(())
}

#[test]
fn references_compare_by_name_without_changing_structural_equality() -> gix_testtools::Result {
    let name = FullName::try_from("refs/heads/main")?;
    let raw = gix_ref::Reference {
        name: name.clone(),
        target: Target::Symbolic(FullName::try_from("refs/heads/target-a")?),
        peeled: None,
    };
    let raw_with_other_target = gix_ref::Reference {
        name: name.clone(),
        target: Target::Symbolic(FullName::try_from("refs/heads/target-b")?),
        peeled: None,
    };
    assert_ne!(raw, raw_with_other_target, "structural equality still includes targets");
    assert_reference_name_equality!(raw, name);

    let loose = gix_ref::file::loose::Reference {
        name: name.clone(),
        target: Target::Symbolic(FullName::try_from("refs/heads/target-a")?),
    };
    let loose_with_other_target = gix_ref::file::loose::Reference {
        name: name.clone(),
        target: Target::Symbolic(FullName::try_from("refs/heads/target-b")?),
    };
    assert_ne!(
        loose, loose_with_other_target,
        "loose-reference structural equality still includes targets"
    );
    assert_reference_name_equality!(loose, name);

    let packed = gix_ref::packed::Reference {
        name: name.as_ref(),
        target: b"0000000000000000000000000000000000000000".as_bstr(),
        object: None,
    };
    let packed_with_other_target = gix_ref::packed::Reference {
        name: name.as_ref(),
        target: b"1111111111111111111111111111111111111111".as_bstr(),
        object: None,
    };
    assert_ne!(
        packed, packed_with_other_target,
        "packed-reference structural equality still includes targets"
    );
    assert_reference_name_equality!(packed, name);
    Ok(())
}

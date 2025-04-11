#[test]
fn into_namespaced_prefix() {
    assert_eq!(
        gix_ref::namespace::expand("foo")
            .unwrap()
            .into_namespaced_prefix("prefix".as_ref()),
        "refs/namespaces/foo/prefix".as_ref(),
    );
    assert_eq!(
        gix_ref::namespace::expand("foo")
            .unwrap()
            .into_namespaced_prefix("prefix/".as_ref()),
        "refs/namespaces/foo/prefix/".as_ref(),
    );
}

mod expand {
    #[test]
    fn components_end_with_trailing_slash_to_help_with_prefix_stripping() {
        assert_eq!(
            gix_ref::namespace::expand("foo").unwrap().as_bstr(),
            "refs/namespaces/foo/"
        );
    }

    #[test]
    fn each_component_expands_to_the_namespace_prefix_individually() {
        assert_eq!(
            gix_ref::namespace::expand("foo/bar").unwrap().as_bstr(),
            "refs/namespaces/foo/refs/namespaces/bar/"
        );
    }

    #[test]
    fn backslashes_are_no_component_separators_and_invalid() {
        assert!(matches!(
            gix_ref::namespace::expand(r"foo\bar").expect_err("empty invalid"),
            gix_validate::reference::name::Error::Tag(
                gix_validate::tag::name::Error::InvalidByte{byte}
            ) if byte == r"\"
        ));
    }

    #[test]
    fn trailing_slashes_are_not_allowed() {
        assert!(matches!(
            gix_ref::namespace::expand("foo/").expect_err("empty invalid"),
            gix_validate::reference::name::Error::Tag(gix_validate::tag::name::Error::EndsWithSlash)
        ));
    }

    #[test]
    fn empty_namespaces_are_not_allowed() {
        assert!(matches!(
            gix_ref::namespace::expand("").expect_err("empty invalid"),
            gix_validate::reference::name::Error::Tag(gix_validate::tag::name::Error::Empty)
        ));
    }
    #[test]
    fn bare_slashes_are_not_allowed() {
        assert!(matches!(
            gix_ref::namespace::expand("/").expect_err("empty invalid"),
            gix_validate::reference::name::Error::Tag(gix_validate::tag::name::Error::EndsWithSlash)
        ));
    }
    #[test]
    fn repeated_slashes_are_invalid() {
        assert!(matches!(
            gix_ref::namespace::expand("foo//bar").expect_err("empty invalid"),
            gix_validate::reference::name::Error::Tag(gix_validate::tag::name::Error::RepeatedSlash)
        ));
    }
}

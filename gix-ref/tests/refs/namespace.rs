#[test]
fn into_namespaced_prefix() {
    assert_eq!(
        gix_ref::namespace::expand("foo")
            .unwrap()
            .into_namespaced_prefix("prefix".try_into().unwrap()),
        "refs/namespaces/foo/prefix",
    );
    assert_eq!(
        gix_ref::namespace::expand("foo")
            .unwrap()
            .into_namespaced_prefix("prefix/".try_into().unwrap()),
        "refs/namespaces/foo/prefix/",
    );
}

mod expand {
    #[test]
    fn components_end_with_trailing_slash_to_help_with_prefix_stripping() {
        assert_eq!(gix_ref::namespace::expand("foo").unwrap(), "refs/namespaces/foo/");
    }

    #[test]
    fn each_component_expands_to_the_namespace_prefix_individually() {
        assert_eq!(
            gix_ref::namespace::expand("foo/bar").unwrap(),
            "refs/namespaces/foo/refs/namespaces/bar/"
        );
    }

    #[test]
    fn backslashes_are_no_component_separators_and_invalid() {
        assert!(matches!(
            gix_ref::namespace::expand(r"foo\bar").expect_err("empty invalid"),
            gix_validate::reference::name::Error::InvalidByte { byte } if byte == r"\"
        ));
    }

    #[test]
    fn trailing_slashes_are_not_allowed() {
        assert!(matches!(
            gix_ref::namespace::expand("foo/").expect_err("empty invalid"),
            gix_validate::reference::name::Error::EndsWithSlash
        ));
    }

    #[test]
    fn empty_namespaces_are_not_allowed() {
        assert!(matches!(
            gix_ref::namespace::expand("").expect_err("empty invalid"),
            gix_validate::reference::name::Error::Empty
        ));
    }
    #[test]
    fn bare_slashes_are_not_allowed() {
        assert!(matches!(
            gix_ref::namespace::expand("/").expect_err("empty invalid"),
            gix_validate::reference::name::Error::EndsWithSlash
        ));
    }
    #[test]
    fn repeated_slashes_are_invalid() {
        assert!(matches!(
            gix_ref::namespace::expand("foo//bar").expect_err("empty invalid"),
            gix_validate::reference::name::Error::RepeatedSlash
        ));
    }
}

#[test]
fn partial_names_roundtrip_through_namespace_expansion() -> gix_testtools::TestResult {
    for name in ["team", "team/project", "refs/namespaces", "refs/namespaces/nested"] {
        let expanded = gix_ref::namespace::expand(name)?;
        let partial = expanded.to_partial_name();
        assert_eq!(partial.as_ref().as_bstr(), name);
        assert_eq!(gix_ref::namespace::expand(partial.as_ref())?, expanded);
    }
    Ok(())
}

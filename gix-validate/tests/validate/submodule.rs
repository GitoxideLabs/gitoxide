use bstr::ByteSlice;

#[test]
fn valid() -> gix_testtools::TestResult {
    for name in [
        b"a/./b/..[".as_slice(),
        b"..a/./b/",
        br"..a\./b\",
        "你好".as_bytes(),
        b"raw-\xff",
    ] {
        assert_eq!(
            gix_validate::submodule::name(name.as_bstr())?,
            name.as_bstr(),
            "valid submodule names retain their bytes"
        );
    }
    Ok(())
}

mod invalid {
    use bstr::ByteSlice;

    macro_rules! mktest {
        ($name:ident, $input:literal, $expected:ident, @$snapshot:literal) => {
            #[test]
            fn $name() {
                let err = gix_validate::submodule::name($input.as_bstr()).expect_err("the input is invalid");
                    insta::assert_debug_snapshot!(err, "invalid submodule names retain their specific failure", @$snapshot);
                    assert!(matches!(err, gix_validate::submodule::name::Error::$expected), "the failure retains its error variant");
            }
        };
    }

    mktest!(empty, b"", Empty, @"Empty");
    mktest!(starts_with_parent_component, b"../", ParentComponent, @"ParentComponent");
    mktest!(parent_component_in_middle, b"hi/../ho", ParentComponent, @"ParentComponent");
    mktest!(ends_with_parent_component, b"hi/ho/..", ParentComponent, @"ParentComponent");
    mktest!(only_parent_component, b"..", ParentComponent, @"ParentComponent");
    mktest!(starts_with_parent_component_backslash, br"..\", ParentComponent, @"ParentComponent");
    mktest!(parent_component_in_middle_backslash, br"hi\..\ho", ParentComponent, @"ParentComponent");
    mktest!(ends_with_parent_component_backslash, br"hi\ho\..", ParentComponent, @"ParentComponent");

    mktest!(nul_byte, b"a\0b", NulByte, @"NulByte");

    /// Reproducer for GHSA-p3hw-mv63-rf9w: a crafted submodule name can place a harmless `..`
    /// first and a real `../` traversal later, so validators that only inspect the first match
    /// accept a name that still escapes `.git/modules`.
    #[test]
    fn traversal_after_a_benign_double_dot_is_rejected() {
        let err = gix_validate::submodule::name(b"a..b/../../../.git/".as_bstr())
            .expect_err("the submodule name escapes its parent");
        insta::assert_debug_snapshot!(err, "parent traversal remains invalid after a benign pair of dots", @"ParentComponent");
        assert!(matches!(err, gix_validate::submodule::name::Error::ParentComponent));
    }

    /// Reproducer for GHSA-p3hw-mv63-rf9w: the same first-match bypass also applies to
    /// Windows-style separators.
    #[test]
    fn backslash_traversal_after_a_benign_double_dot_is_rejected() {
        let err = gix_validate::submodule::name(br"a..b\..\..\..\.git\".as_bstr())
            .expect_err("the submodule name escapes its parent");
        insta::assert_debug_snapshot!(err, "parent traversal remains invalid after a benign pair of dots", @"ParentComponent");
        assert!(matches!(err, gix_validate::submodule::name::Error::ParentComponent));
    }
}

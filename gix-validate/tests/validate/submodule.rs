use gix_validate::submodule::name::Error;

#[test]
fn valid() {
    fn validate(name: &str) -> Result<(), Error> {
        gix_validate::submodule::name(name.into()).map(|_| ())
    }

    for valid_name in ["a/./b/..[", "..a/./b/", r"..a\./b\", "你好"] {
        validate(valid_name).unwrap_or_else(|err| panic!("{valid_name} should be valid: {err:?}"));
    }
}

mod invalid {
    use bstr::ByteSlice;

    macro_rules! mktest {
        ($name:ident, $input:literal, $expected:ident) => {
            #[test]
            fn $name() {
                match gix_validate::submodule::name($input.as_bstr()) {
                    Err(gix_validate::submodule::name::Error::$expected) => {}
                    got => panic!("Wanted {}, got {:?}", stringify!($expected), got),
                }
            }
        };
    }

    mktest!(empty, b"", Empty);
    mktest!(starts_with_parent_component, b"../", ParentComponent);
    mktest!(parent_component_in_middle, b"hi/../ho", ParentComponent);
    mktest!(ends_with_parent_component, b"hi/ho/..", ParentComponent);
    mktest!(only_parent_component, b"..", ParentComponent);
    mktest!(starts_with_parent_component_backslash, br"..\", ParentComponent);
    mktest!(parent_component_in_middle_backslash, br"hi\..\ho", ParentComponent);
    mktest!(ends_with_parent_component_backslash, br"hi\ho\..", ParentComponent);

    /// Reproducer for GHSA-p3hw-mv63-rf9w: a crafted submodule name can place a harmless `..`
    /// first and a real `../` traversal later, so validators that only inspect the first match
    /// accept a name that still escapes `.git/modules`.
    #[test]
    fn traversal_after_a_benign_double_dot_is_rejected() {
        match gix_validate::submodule::name(b"a..b/../../../.git/".as_bstr()) {
            Err(gix_validate::submodule::name::Error::ParentComponent) => {}
            got => panic!("Wanted ParentComponent, got {got:?}"),
        }
    }

    /// Reproducer for GHSA-p3hw-mv63-rf9w: the same first-match bypass also applies to
    /// Windows-style separators.
    #[test]
    fn backslash_traversal_after_a_benign_double_dot_is_rejected() {
        match gix_validate::submodule::name(br"a..b\..\..\..\.git\".as_bstr()) {
            Err(gix_validate::submodule::name::Error::ParentComponent) => {}
            got => panic!("Wanted ParentComponent, got {got:?}"),
        }
    }
}

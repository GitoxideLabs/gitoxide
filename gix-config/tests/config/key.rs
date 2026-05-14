use gix_config::AsKey;

#[test]
fn valid_and_invalid() {
    assert_eq!("".try_as_key(), None);
    assert_eq!("foo".try_as_key(), None);

    let key = "foo.bar".as_key();
    assert_eq!(key.section_name, "foo");
    assert_eq!(key.subsection_name, None);
    assert_eq!(key.value_name, "bar");

    let key = "foo.bar.baz".as_key();
    assert_eq!(key.section_name, "foo");
    assert_eq!(key.subsection_name.unwrap(), "bar");
    assert_eq!(key.value_name, "baz");

    let key = "foo.quux.bar.baz".as_key();
    assert_eq!(key.section_name, "foo");
    assert_eq!(key.subsection_name.unwrap(), "quux.bar");
    assert_eq!(key.value_name, "baz");

    let key = r"includeIf.gitdir/i:C:\bare.git.path".as_key();
    assert_eq!(key.subsection_name, Some(r"gitdir/i:C:\bare.git".into()));
}

mod _ref {
    use gix_config::KeyRef;

    #[test]
    fn missing_dot_is_invalid() {
        assert_eq!(KeyRef::parse_unvalidated("hello".into()), None);
    }

    #[test]
    fn section_name_and_key() {
        assert_eq!(
            KeyRef::parse_unvalidated("core.bare".into()),
            Some(KeyRef {
                section_name: "core",
                subsection_name: None,
                value_name: "bare"
            })
        );
    }

    #[test]
    fn section_name_subsection_and_key() {
        assert_eq!(
            KeyRef::parse_unvalidated("remote.origin.url".into()),
            Some(KeyRef {
                section_name: "remote",
                subsection_name: Some("origin".into()),
                value_name: "url"
            })
        );

        assert_eq!(
            KeyRef::parse_unvalidated(r"includeIf.gitdir/i:C:\bare.git.path".into()),
            Some(KeyRef {
                section_name: "includeIf",
                subsection_name: Some(r"gitdir/i:C:\bare.git".into()),
                value_name: "path"
            })
        );
    }
}

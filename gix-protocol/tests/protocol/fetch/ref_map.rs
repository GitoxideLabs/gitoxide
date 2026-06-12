mod from_refs {
    use gix_protocol::fetch::{RefMap, refmap};
    use gix_transport::client::Capabilities;

    fn caps_with(after_nul: &[u8]) -> Capabilities {
        let mut bytes: Vec<u8> = b"7814e8a05a59c0cf5fb186661d1551c75d1299b5 HEAD\0".to_vec();
        bytes.extend_from_slice(after_nul);
        Capabilities::from_bytes(&bytes).expect("valid capabilities line").0
    }

    fn ctx() -> refmap::init::Context {
        refmap::init::Context {
            fetch_refspecs: Vec::new(),
            extra_refspecs: Vec::new(),
        }
    }

    /// An `object-format=sha1` capability resolves to `gix_hash::Kind::Sha1` on the resulting RefMap.
    #[test]
    fn sha1_capability_is_honored() {
        let caps = caps_with(b"symref=HEAD:refs/heads/main object-format=sha1 agent=git/2.54.0");
        let map = RefMap::from_refs(Vec::new(), &caps, ctx()).expect("known format");
        assert_eq!(map.object_hash, gix_hash::Kind::Sha1);
    }

    /// An `object-format=sha256` capability resolves to `gix_hash::Kind::Sha256`, so Sha256 servers
    /// can be talked to without falling back to Sha1.
    #[cfg(feature = "sha256")]
    #[test]
    fn sha256_capability_is_honored() {
        let caps = caps_with(b"symref=HEAD:refs/heads/main object-format=sha256 agent=git/2.54.0");
        let map = RefMap::from_refs(Vec::new(), &caps, ctx()).expect("known format");
        assert_eq!(map.object_hash, gix_hash::Kind::Sha256);
    }

    /// Any `object-format` value we don't recognize must surface as an UnknownObjectFormat error
    /// rather than silently defaulting to Sha1, so callers can refuse to fetch from unsupported servers.
    #[test]
    fn unknown_object_format_errors() {
        let caps = caps_with(b"symref=HEAD:refs/heads/main object-format=sha999 agent=git/2.54.0");
        let err = RefMap::from_refs(Vec::new(), &caps, ctx()).expect_err("unknown format must error");
        assert!(matches!(err, refmap::init::Error::UnknownObjectFormat { ref format } if format == "sha999"));
    }

    /// Servers that omit `object-format` are implicitly Sha1, so the RefMap should reflect that.
    #[cfg(feature = "sha1")]
    #[test]
    fn missing_object_format_defaults_to_sha1() {
        let caps = caps_with(b"symref=HEAD:refs/heads/main agent=git/2.54.0");
        let map = RefMap::from_refs(Vec::new(), &caps, ctx()).expect("implicit sha1");
        assert_eq!(map.object_hash, gix_hash::Kind::Sha1);
    }
}

mod is_missing_required_mapping {
    use gix_protocol::{
        fetch::{
            RefMap,
            refmap::{Mapping, Source, SpecIndex},
        },
        handshake::Ref,
    };

    fn fetch_spec(spec: &str) -> gix_refspec::RefSpec {
        gix_refspec::parse(spec.into(), gix_refspec::parse::Operation::Fetch)
            .expect("valid")
            .to_owned()
    }

    /// If the server advertised refs but none of them matched at all, callers should treat this as a
    /// missing required mapping instead of silently proceeding with an empty result.
    ///
    /// This covers the case where an exact fetch refspec like `refs/heads/main` was expected to match
    /// something on the remote, but the only advertised ref was unrelated.
    #[test]
    fn is_true_if_remote_refs_exist_but_nothing_mapped() {
        let map = RefMap {
            refspecs: vec![fetch_spec("refs/heads/main")],
            remote_refs: vec![Ref::Direct {
                full_ref_name: "refs/heads/other".into(),
                object: gix_hash::ObjectId::null(gix_hash::Kind::Sha1),
            }],
            object_hash: gix_hash::Kind::Sha1,
            ..Default::default()
        };

        assert!(map.is_missing_required_mapping());
    }

    /// Exact explicit refspecs like `HEAD` still require an explicit mapping, even if unrelated implicit
    /// mappings exist due to extra refspecs such as tag following.
    ///
    /// Returning `true` here ensures callers don't mistake an implicit-only result for a successful match
    /// of the user's explicit fetch request.
    #[test]
    fn is_true_if_only_implicit_mappings_exist_for_exact_refspecs() {
        let map = RefMap {
            mappings: vec![Mapping {
                remote: Source::ObjectId(gix_hash::ObjectId::null(gix_hash::Kind::Sha1)),
                local: Some("refs/remotes/origin/main".into()),
                spec_index: SpecIndex::Implicit(0),
            }],
            refspecs: vec![fetch_spec("HEAD")],
            extra_refspecs: vec![fetch_spec("refs/tags/*:refs/tags/*")],
            object_hash: gix_hash::Kind::Sha1,
            ..Default::default()
        };

        assert!(map.is_missing_required_mapping());
    }

    /// Wildcard refspecs are allowed to match nothing without being considered an error.
    ///
    /// They express interest in a namespace rather than in one required ref, so an empty result here means
    /// "nothing matched" instead of "a required mapping is missing".
    #[test]
    fn is_false_if_wildcards_are_the_only_unmatched_explicit_refspecs() {
        let map = RefMap {
            refspecs: vec![fetch_spec("refs/heads/*:refs/remotes/origin/*")],
            object_hash: gix_hash::Kind::Sha1,
            ..Default::default()
        };

        assert!(!map.is_missing_required_mapping());
    }

    /// Once at least one explicit refspec produced a mapping, the helper must report success.
    ///
    /// This is the common case of an exact refspec matching as intended, so callers should continue with
    /// negotiation or ref updates instead of raising a no-mapping error.
    #[test]
    fn is_false_if_an_explicit_mapping_exists() {
        let map = RefMap {
            mappings: vec![Mapping {
                remote: Source::ObjectId(gix_hash::ObjectId::null(gix_hash::Kind::Sha1)),
                local: Some("refs/remotes/origin/main".into()),
                spec_index: SpecIndex::ExplicitInRemote(0),
            }],
            refspecs: vec![fetch_spec("refs/heads/main")],
            object_hash: gix_hash::Kind::Sha1,
            ..Default::default()
        };

        assert!(!map.is_missing_required_mapping());
    }
}

mod from_refs {
    use gix_protocol::fetch::{RefMap, refmap};
    use gix_transport::client::Capabilities;

    #[test]
    fn sha1_capability_is_honored() {
        let caps = caps_with(b"symref=HEAD:refs/heads/main object-format=sha1 agent=git/2.54.0");
        let map = RefMap::from_refs(Vec::new(), &caps, ctx()).expect("known format");
        assert_eq!(
            map.object_hash,
            gix_hash::Kind::Sha1,
            "An `object-format=sha1` capability resolves to `gix_hash::Kind::Sha1` on the resulting RefMap"
        );
    }

    #[cfg(feature = "sha256")]
    #[test]
    fn sha256_capability_is_honored() {
        let caps = caps_with(b"symref=HEAD:refs/heads/main object-format=sha256 agent=git/2.54.0");
        let map = RefMap::from_refs(Vec::new(), &caps, ctx()).expect("known format");
        assert_eq!(
            map.object_hash,
            gix_hash::Kind::Sha256,
            "An `object-format=sha256` capability resolves to `gix_hash::Kind::Sha256`, so Sha256 servers
            can be talked to without falling back to Sha1"
        );
    }

    #[test]
    fn unknown_object_format_errors() {
        let caps = caps_with(b"symref=HEAD:refs/heads/main object-format=sha999 agent=git/2.54.0");
        let err = RefMap::from_refs(Vec::new(), &caps, ctx()).expect_err("unknown format must error");
        insta::assert_debug_snapshot!(err, "unknown formats are unsupported", @r#"The object format used by the remote is unsupported: sha999, input="sha999""#);
        assert!(err.is_unsupported(), "unknown formats require another implementation");
        let metadata = err.metadata().next().expect("the unsupported format is retained");
        assert_eq!(
            metadata.get("input"),
            Some(&gix_error::MetadataValue::from(b"sha999".as_slice())),
            "the unsupported format is retained for callers"
        );
    }

    #[test]
    fn malformed_object_format_retains_its_source() {
        let caps = caps_with(b"object-format=\xff");
        let err = RefMap::from_refs(Vec::new(), &caps, ctx()).expect_err("the format is not UTF-8");
        assert!(err.is_corrupted(), "malformed peer bytes are corruption");
        assert!(
            err.downcast_any_ref::<bstr::Utf8Error>().is_some(),
            "the native decoding failure remains available"
        );
    }

    #[test]
    fn missing_object_format_defaults_to_sha1() {
        let caps = caps_with(b"symref=HEAD:refs/heads/main agent=git/2.54.0");
        let map = RefMap::from_refs(Vec::new(), &caps, ctx()).expect("implicit sha1");
        assert_eq!(
            map.object_hash,
            gix_hash::Kind::Sha1,
            "Servers that omit `object-format` are implicitly Sha1, so the RefMap should reflect that"
        );
    }

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

    #[test]
    fn a_missing_exact_source_is_required_even_when_another_spec_matches() -> gix_testtools::TestResult {
        let capabilities = gix_transport::client::Capabilities::from_bytes(b"\0object-format=sha1")?.0;
        let map = RefMap::from_refs(
            vec![Ref::Direct {
                full_ref_name: "refs/heads/main".into(),
                object: gix_hash::ObjectId::null(gix_hash::Kind::Sha1),
            }],
            &capabilities,
            gix_protocol::fetch::refmap::init::Context {
                fetch_refspecs: vec![fetch_spec("refs/heads/main"), fetch_spec("refs/heads/absent")],
                extra_refspecs: Vec::new(),
            },
        )?;
        assert!(
            map.is_missing_required_mapping(),
            "every exact requested source must exist before fetching"
        );
        assert_eq!(
            map.missing_source_refs().collect::<Vec<_>>(),
            ["refs/heads/absent"],
            "the missing request is named"
        );
        Ok(())
    }

    #[test]
    fn unmatched_wildcards_allow_unrelated_advertisements() -> gix_testtools::TestResult {
        let capabilities = gix_transport::client::Capabilities::from_bytes(b"\0object-format=sha1")?.0;
        let map = RefMap::from_refs(
            vec![Ref::Direct {
                full_ref_name: "refs/tags/v1".into(),
                object: gix_hash::ObjectId::null(gix_hash::Kind::Sha1),
            }],
            &capabilities,
            gix_protocol::fetch::refmap::init::Context {
                fetch_refspecs: vec![fetch_spec("refs/heads/*:refs/remotes/origin/*")],
                extra_refspecs: Vec::new(),
            },
        )?;
        assert!(
            !map.is_missing_required_mapping(),
            "a wildcard namespace can be empty even when other namespaces exist"
        );
        Ok(())
    }

    #[test]
    fn duplicate_partial_and_excluded_sources_are_still_advertised() -> gix_testtools::TestResult {
        let capabilities = gix_transport::client::Capabilities::from_bytes(b"\0object-format=sha1")?.0;
        let map = RefMap::from_refs(
            vec![Ref::Direct {
                full_ref_name: "refs/heads/main".into(),
                object: gix_hash::ObjectId::null(gix_hash::Kind::Sha1),
            }],
            &capabilities,
            gix_protocol::fetch::refmap::init::Context {
                fetch_refspecs: vec![
                    fetch_spec("refs/heads/main"),
                    fetch_spec("main"),
                    fetch_spec("^refs/heads/main"),
                ],
                extra_refspecs: Vec::new(),
            },
        )?;
        assert!(
            map.mappings.is_empty(),
            "the negative refspec removes all selected mappings"
        );
        assert!(
            !map.is_missing_required_mapping(),
            "selection excludes an existing source, so it isn't missing"
        );
        Ok(())
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

mod remote_head_candidates {
    use gix_protocol::{fetch::RefMap, handshake::Ref};

    fn direct(name: &str, object: gix_hash::ObjectId) -> Ref {
        Ref::Direct {
            full_ref_name: name.into(),
            object,
        }
    }

    #[test]
    fn advertised_head_candidates_distinguish_symbolic_detached_and_unborn_heads() {
        let commit_id = gix_hash::ObjectId::from_hex(b"1111111111111111111111111111111111111111").expect("valid id");
        let other_id = gix_hash::ObjectId::from_hex(b"2222222222222222222222222222222222222222").expect("valid id");
        let branches = [direct("refs/heads/\"", commit_id), direct("refs/heads/other", other_id)];
        let cases = [
            (None, Vec::new()),
            (Some(direct("HEAD", commit_id)), vec!["refs/heads/\""]),
            (
                Some(Ref::Symbolic {
                    full_ref_name: "HEAD".into(),
                    target: "refs/heads/other".into(),
                    tag: None,
                    object: other_id,
                }),
                vec!["refs/heads/other"],
            ),
            (
                Some(Ref::Symbolic {
                    full_ref_name: "HEAD".into(),
                    target: "refs/heads/hidden".into(),
                    tag: None,
                    object: commit_id,
                }),
                Vec::new(),
            ),
            (
                Some(Ref::Unborn {
                    full_ref_name: "HEAD".into(),
                    target: "refs/heads/missing".into(),
                }),
                Vec::new(),
            ),
        ];
        for (head, expected) in cases {
            let map = RefMap {
                remote_refs: head
                    .into_iter()
                    .chain(branches.clone())
                    .chain([direct("refs/tags/tag", commit_id)])
                    .collect(),
                ..Default::default()
            };
            let actual: Vec<_> = map
                .remote_head_candidates()
                .map(|reference| reference.unpack().0)
                .collect();
            let expected: Vec<&bstr::BStr> = expected.iter().map(|name| (*name).into()).collect();
            assert_eq!(
                actual, expected,
                "only advertised branches can identify the remote HEAD"
            );
        }

        let map = RefMap {
            remote_refs: vec![
                direct("HEAD", commit_id),
                direct("refs/heads/one", commit_id),
                direct("refs/heads/two", commit_id),
            ],
            ..Default::default()
        };
        assert_eq!(
            map.remote_head_candidates().count(),
            2,
            "ambiguous detached HEADs retain all candidates"
        );
    }
}

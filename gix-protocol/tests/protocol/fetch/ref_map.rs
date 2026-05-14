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

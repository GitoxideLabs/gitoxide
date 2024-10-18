mod v1 {
    fn capabilities(input: &str) -> gix_transport::client::Capabilities {
        gix_transport::client::Capabilities::from_bytes(format!("\0{input}").as_bytes())
            .expect("valid input capabilities")
            .0
    }

    const GITHUB_CAPABILITIES: &str = "multi_ack thin-pack side-band ofs-delta shallow deepen-since deepen-not deepen-relative no-progress include-tag allow-tip-sha1-in-want allow-reachable-sha1-in-want no-done symref=HEAD:refs/heads/main filter agent=git/github-gdf51a71f0236";
    mod fetch {
        mod default_features {
            use super::super::{capabilities, GITHUB_CAPABILITIES};
            use gix_protocol::Command;

            #[test]
            fn it_chooses_the_best_multi_ack_and_sideband() {
                assert_eq!(
                    Command::Fetch.default_features(
                        gix_transport::Protocol::V1,
                        &capabilities("multi_ack side-band side-band-64k multi_ack_detailed")
                    ),
                    &[("side-band-64k", None), ("multi_ack_detailed", None),]
                );
            }

            #[test]
            fn it_chooses_all_supported_non_stacking_capabilities_and_leaves_no_progress() {
                assert_eq!(
                    Command::Fetch.default_features(gix_transport::Protocol::V1, &capabilities(GITHUB_CAPABILITIES)),
                    &[
                        ("multi_ack", None),
                        ("thin-pack", None),
                        ("side-band", None),
                        ("ofs-delta", None),
                        ("shallow", None),
                        ("deepen-since", None),
                        ("deepen-not", None),
                        ("deepen-relative", None),
                        ("include-tag", None),
                        ("allow-tip-sha1-in-want", None),
                        ("allow-reachable-sha1-in-want", None),
                        ("no-done", None),
                        ("filter", None),
                    ],
                    "we don't enforce no-progress"
                );
            }
        }
    }
}

mod v2 {
    use gix_transport::client::Capabilities;

    fn capabilities(command: &str, input: &str) -> Capabilities {
        Capabilities::from_lines(format!("version 2\n{command}={input}").into())
            .expect("valid input for V2 capabilities")
    }

    mod fetch {
        mod default_features {
            use super::super::capabilities;
            use gix_protocol::Command;

            #[test]
            fn all_features() {
                assert_eq!(
                    Command::Fetch.default_features(
                        gix_transport::Protocol::V2,
                        &capabilities("fetch", "shallow filter ref-in-want sideband-all packfile-uris")
                    ),
                    ["shallow", "filter", "ref-in-want", "sideband-all", "packfile-uris"]
                        .iter()
                        .map(|s| (*s, None))
                        .collect::<Vec<_>>()
                );
            }
        }

        mod initial_arguments {
            use bstr::ByteSlice;

            use super::super::capabilities;
            use gix_protocol::Command;

            #[test]
            fn for_all_features() {
                assert_eq!(
                    Command::Fetch.initial_v2_arguments(&Command::Fetch.default_features(
                        gix_transport::Protocol::V2,
                        &capabilities("fetch", "shallow filter sideband-all packfile-uris")
                    )),
                    ["thin-pack", "ofs-delta", "sideband-all"]
                        .iter()
                        .map(|s| s.as_bytes().as_bstr().to_owned())
                        .collect::<Vec<_>>(),
                    "packfile-uris isn't really supported that well and we don't support it either yet"
                );
            }
        }
    }

    mod ls_refs {
        mod default_features {
            use super::super::capabilities;
            use gix_protocol::Command;

            #[test]
            fn default_as_there_are_no_features() {
                assert_eq!(
                    Command::LsRefs.default_features(
                        gix_transport::Protocol::V2,
                        &capabilities("something-else", "does not matter as there are none")
                    ),
                    &[]
                );
            }
        }

        mod validate {
            use bstr::ByteSlice;

            use super::super::capabilities;
            use gix_protocol::Command;

            #[test]
            fn ref_prefixes_can_always_be_used() {
                assert!(Command::LsRefs
                    .validate_argument_prefixes(
                        gix_transport::Protocol::V2,
                        &capabilities("something else", "do-not-matter"),
                        &[b"ref-prefix hello/".as_bstr().into()],
                        &[],
                    )
                    .is_ok());
            }

            #[test]
            fn unknown_argument() {
                assert_eq!(
                    Command::LsRefs
                        .validate_argument_prefixes(
                            gix_transport::Protocol::V2,
                            &capabilities("other", "do-not-matter"),
                            &[b"definitely-nothing-we-know".as_bstr().into()],
                            &[],
                        )
                        .unwrap_err()
                        .to_string(),
                    "ls-refs: argument definitely-nothing-we-know is not known or allowed"
                );
            }

            #[test]
            fn unknown_feature() {
                assert_eq!(
                    Command::LsRefs
                        .validate_argument_prefixes(
                            gix_transport::Protocol::V2,
                            &capabilities("other", "do-not-matter"),
                            &[],
                            &[("some-feature-that-does-not-exist", None)],
                        )
                        .unwrap_err()
                        .to_string(),
                    "ls-refs: capability some-feature-that-does-not-exist is not supported"
                );
            }
        }
    }
}

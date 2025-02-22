use gix_testtools::once_cell::sync::Lazy;

static BASELINE: Lazy<baseline::Baseline> = Lazy::new(|| baseline::parse().unwrap());

pub mod baseline {
    use std::{borrow::Borrow, collections::HashMap};

    use bstr::{BString, ByteSlice, ByteVec};
    use gix_hash::ObjectId;
    use gix_refspec::{
        match_group::{validate::Fix, SourceRef},
        parse::Operation,
        MatchGroup,
    };
    use gix_testtools::once_cell::sync::Lazy;

    use crate::matching::BASELINE;

    #[derive(Debug)]
    pub struct Ref {
        pub name: BString,
        pub target: ObjectId,
        /// Set if `target` is an annotated tag, this being the object it points to.
        pub object: Option<ObjectId>,
    }

    impl Ref {
        pub fn to_item(&self) -> gix_refspec::match_group::Item<'_> {
            gix_refspec::match_group::Item {
                full_ref_name: self.name.borrow(),
                target: &self.target,
                object: self.object.as_deref(),
            }
        }
    }

    static INPUT: Lazy<Vec<Ref>> = Lazy::new(|| parse_input().unwrap());

    pub type Baseline = HashMap<Vec<BString>, Result<Vec<Mapping>, BString>>;

    #[derive(Debug)]
    pub struct Mapping {
        pub remote: BString,
        /// `None` if there is no destination/tracking branch
        pub local: Option<BString>,
    }

    pub fn input() -> impl ExactSizeIterator<Item = gix_refspec::match_group::Item<'static>> + Clone {
        INPUT.iter().map(Ref::to_item)
    }

    pub fn of_objects_with_destinations_are_written_into_given_local_branches<'a, 'b>(
        specs: impl IntoIterator<Item = &'a str> + Clone,
        expected: impl IntoIterator<Item = &'b str>,
    ) {
        agrees_and_applies_fixes(specs, Vec::new(), expected);
    }

    pub fn agrees_and_applies_fixes<'a, 'b>(
        specs: impl IntoIterator<Item = &'a str> + Clone,
        fixes: impl IntoIterator<Item = Fix>,
        expected: impl IntoIterator<Item = &'b str>,
    ) {
        check_fetch_remote(
            specs,
            Mode::Custom {
                expected: expected
                    .into_iter()
                    .map(|s| {
                        let spec = gix_refspec::parse(s.into(), Operation::Fetch).expect("valid spec");
                        Mapping {
                            remote: spec.source().unwrap().into(),
                            local: spec.destination().map(ToOwned::to_owned),
                        }
                    })
                    .collect(),
                fixes: fixes.into_iter().collect(),
            },
        );
    }

    pub fn of_objects_always_matches_if_the_server_has_the_object<'a, 'b>(
        specs: impl IntoIterator<Item = &'a str> + Clone,
    ) {
        check_fetch_remote(specs, Mode::Normal { validate_err: None });
    }

    pub fn agrees_with_fetch_specs<'a>(specs: impl IntoIterator<Item = &'a str> + Clone) {
        check_fetch_remote(specs, Mode::Normal { validate_err: None });
    }

    pub fn agrees_with_fetch_specs_validation_error<'a>(
        specs: impl IntoIterator<Item = &'a str> + Clone,
        validate_err: impl Into<String>,
    ) {
        check_fetch_remote(
            specs,
            Mode::Normal {
                validate_err: Some(validate_err.into()),
            },
        );
    }

    pub fn invalid_specs_fail_to_parse_where_git_shows_surprising_behaviour<'a>(
        specs: impl IntoIterator<Item = &'a str>,
        err: gix_refspec::parse::Error,
    ) {
        let err = err.to_string();
        let mut saw_err = false;
        for spec in specs {
            match gix_refspec::parse(spec.into(), Operation::Fetch) {
                Ok(_) => {}
                Err(e) if e.to_string() == err => {
                    saw_err = true;
                }
                Err(err) => panic!("Unexpected parse error: {err:?}"),
            }
        }
        assert!(saw_err, "Failed to see error when parsing specs: {err:?}");
    }

    /// Here we checked by hand which refs are actually written with a particular refspec
    pub fn agrees_but_observable_refs_are_vague<'a, 'b>(
        specs: impl IntoIterator<Item = &'a str> + Clone,
        expected: impl IntoIterator<Item = &'b str>,
    ) {
        of_objects_with_destinations_are_written_into_given_local_branches(specs, expected);
    }

    enum Mode {
        Normal { validate_err: Option<String> },
        Custom { expected: Vec<Mapping>, fixes: Vec<Fix> },
    }

    fn check_fetch_remote<'a>(specs: impl IntoIterator<Item = &'a str> + Clone, mode: Mode) {
        let match_group = MatchGroup::from_fetch_specs(
            specs
                .clone()
                .into_iter()
                .map(|spec| gix_refspec::parse(spec.into(), Operation::Fetch).unwrap()),
        );

        let key: Vec<_> = specs.into_iter().map(BString::from).collect();
        let expected = BASELINE
            .get(&key)
            .unwrap_or_else(|| panic!("BUG: Need {key:?} added to the baseline"))
            .as_ref();

        let actual = match_group.match_lhs(input()).validated();
        let (actual, expected) = match &mode {
            Mode::Normal { validate_err } => match validate_err {
                Some(err_message) => {
                    assert_eq!(actual.unwrap_err().to_string(), *err_message);
                    return;
                }
                None => {
                    let (actual, fixed) = actual.unwrap();
                    assert_eq!(
                        fixed,
                        Vec::<gix_refspec::match_group::validate::Fix>::new(),
                        "we don't expect any issues to be fixed here"
                    );
                    (actual.mappings, expected.expect("no error"))
                }
            },
            Mode::Custom {
                expected,
                fixes: expected_fixes,
            } => {
                let (actual, actual_fixes) = actual.unwrap();
                assert_eq!(&actual_fixes, expected_fixes,);
                (actual.mappings, expected)
            }
        };
        assert_eq!(
            actual.len(),
            expected.len(),
            "got a different amount of mappings: {actual:?} != {expected:?}"
        );

        for (idx, (actual, expected)) in actual.iter().zip(expected).enumerate() {
            assert_eq!(
                source_to_bstring(&actual.lhs),
                expected.remote,
                "{idx}: remote mismatch"
            );
            if let Some(expected) = expected.local.as_ref() {
                match actual.rhs.as_ref() {
                    None => panic!("{idx}: Expected local ref to be {expected}, got none"),
                    Some(actual) => assert_eq!(actual.as_ref(), expected, "{idx}: mismatched local ref"),
                }
            }
        }
    }

    fn source_to_bstring(source: &SourceRef) -> BString {
        match source {
            SourceRef::FullName(name) => name.as_ref().into(),
            SourceRef::ObjectId(id) => id.to_string().into(),
        }
    }

    fn parse_input() -> crate::Result<Vec<Ref>> {
        let dir = gix_testtools::scripted_fixture_read_only("match_baseline.sh")?;
        let refs_buf = std::fs::read(dir.join("clone").join("remote-refs.list"))?;
        let mut out = Vec::new();
        for line in refs_buf.lines() {
            if line.starts_with(b"From ") {
                continue;
            }
            let mut tokens = line.splitn(2, |b| *b == b'\t');
            let target = ObjectId::from_hex(tokens.next().expect("hex-sha"))?;
            let name = tokens.next().expect("name");
            if !name.ends_with(b"^{}") {
                out.push(Ref {
                    name: name.into(),
                    target,
                    object: None,
                });
            } else {
                out.last_mut().unwrap().object = Some(target);
            }
        }
        Ok(out)
    }

    pub(crate) fn parse() -> crate::Result<Baseline> {
        let dir = gix_testtools::scripted_fixture_read_only("match_baseline.sh")?;
        let buf = std::fs::read(dir.join("clone").join("baseline.git"))?;

        let mut map = HashMap::new();
        let mut mappings = Vec::new();
        let mut fatal = None;
        for line in buf.lines() {
            if line.starts_with(b"From ") {
                continue;
            }
            match line.strip_prefix(b"specs: ") {
                Some(specs) => {
                    let key: Vec<_> = specs.split(|b| *b == b' ').map(BString::from).collect();
                    let value = match fatal.take() {
                        Some(message) => Err(message),
                        None => Ok(std::mem::take(&mut mappings)),
                    };
                    map.insert(key, value);
                }
                None => match line.strip_prefix(b"fatal: ") {
                    Some(message) => {
                        fatal = Some(message.into());
                    }
                    None => match line.strip_prefix(b"error: * Ignoring funny ref") {
                        Some(_) => continue,
                        None => {
                            let past_note = line
                                .splitn(2, |b| *b == b']')
                                .nth(1)
                                .or_else(|| line.strip_prefix(b" * branch "))
                                .or_else(|| line.strip_prefix(b" * tag "))
                                .unwrap_or_else(|| panic!("line unhandled: {:?}", line.as_bstr()));
                            let mut tokens = past_note.split(|b| *b == b' ').filter(|t| !t.is_empty());

                            let mut lhs = tokens.next().unwrap().trim();
                            if lhs.as_bstr() == "->" {
                                lhs = "HEAD".as_bytes();
                            } else {
                                tokens.next();
                            }
                            let rhs = tokens.next().unwrap().trim();
                            let local = (rhs != b"FETCH_HEAD").then(|| full_tracking_ref(rhs.into()));
                            if !(lhs.as_bstr() == "HEAD" && local.is_none()) {
                                mappings.push(Mapping {
                                    remote: full_remote_ref(lhs.into()),
                                    local,
                                });
                            }
                        }
                    },
                },
            }
        }

        Ok(map)
    }

    fn looks_like_tag(name: &BString) -> bool {
        name.starts_with(b"v0.") || name.starts_with(b"annotated-v0.")
    }

    fn full_remote_ref(mut name: BString) -> BString {
        if !name.contains(&b'/') || name.starts_with(b"sub/") || name.starts_with(b"suub/") {
            if looks_like_tag(&name) {
                name.insert_str(0, b"refs/tags/");
            } else if let Ok(_id) = gix_hash::ObjectId::from_hex(name.as_ref()) {
                // keep as is
            } else if name != "HEAD" {
                name.insert_str(0, b"refs/heads/");
            }
        }
        name
    }

    fn full_tracking_ref(mut name: BString) -> BString {
        if name.starts_with_str(b"origin/") || name.starts_with_str("new-origin/") {
            name.insert_str(0, b"refs/remotes/");
        } else if looks_like_tag(&name) {
            name.insert_str(0, b"refs/tags/");
        }
        name
    }
}

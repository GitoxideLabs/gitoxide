use std::collections::BTreeMap;

use bstr::{BStr, ByteSlice};
use gix_attributes::{
    AssignmentRef, NameRef, StateRef,
    search::{AttributeId, Outcome},
};
use gix_glob::pattern::Case;
use gix_testtools::size_ok;

mod specials {
    use std::path::Path;

    use gix_attributes::{
        Search,
        search::{MetadataCollection, Outcome},
    };
    use gix_glob::pattern::Case;

    #[test]
    fn dir_slash_never_matches_but_dir_slah_double_star_does() {
        assert!(!searchi("dir/", "dir/a", None));
    }

    #[test]
    fn dir_slash_double_star_matches_recursively() {
        assert!(searchi("dir/**", "dir/a", None));
        assert!(!search("Dir/**", "dir/a", None), "case doesn't match");
    }

    #[test]
    fn global_and_local_prefixes_respect_case_sensitivity() {
        assert!(searchi("a/b/d/*", "a/B/D/g", None), "fold: this works globally…");
        assert!(searchi("D/*", "a/B/D/g", Some("a/b")), "fold: …and it works locally");
        assert!(!search("a/b/d/*", "a/B/D/g", None), "sensitive: won't match globally");
        assert!(
            !search("D/*", "a/B/D/g", Some("a/b")),
            "sensitive: …and also does not match locally!"
        );
    }

    fn search_case(pattern: &str, path: &str, rela_containing_dir: Option<&str>, case: Case) -> bool {
        let mut search = Search::default();
        let mut collection = MetadataCollection::default();
        search.add_patterns_buffer(
            format!("{pattern} test").as_bytes(),
            rela_containing_dir.map_or_else(|| Path::new("<memory>").into(), |d| Path::new(d).join("filename")),
            rela_containing_dir.map(|_| Path::new("")),
            &mut collection,
            true,
        );
        let mut out = Outcome::default();
        out.initialize(&collection);
        search.pattern_matching_relative_path(path.into(), case, None, &mut out)
    }

    fn searchi(pattern: &str, path: &str, rela_containing_dir: Option<&str>) -> bool {
        search_case(pattern, path, rela_containing_dir, Case::Fold)
    }
    fn search(pattern: &str, path: &str, rela_containing_dir: Option<&str>) -> bool {
        search_case(pattern, path, rela_containing_dir, Case::Sensitive)
    }
}

#[test]
fn baseline() -> crate::Result {
    let mut buf = Vec::new();
    // Due to the way our setup differs from gits dynamic stack (which involves trying to read files from disk
    // by path) we can only test one case baseline, so we require multiple platforms (or filesystems) to run this.
    let case = if gix_fs::Capabilities::probe("../.git".as_ref()).ignore_case {
        Case::Fold
    } else {
        Case::Sensitive
    };
    let (mut group, mut collection, base, input) = baseline::user_attributes("basics")?;

    // Note that we have to hard-code these files for a lack of dynamic stack.
    // This isn't a problem as non-matching prefixes will simply be ignored.
    for (file, use_base) in [
        (".gitattributes", false),
        ("a/.gitattributes", true),
        ("a/b/.gitattributes", true),
    ] {
        let is_global = !use_base;
        group.add_patterns_file(
            base.join(file),
            false,
            use_base.then_some(base.as_path()),
            &mut buf,
            &mut collection,
            is_global, /* allow macros */
        )?;
    }
    assert_eq!(
        group.num_pattern_lists(),
        1 + 4,
        "should have loaded all files, and the builtins"
    );

    let mut actual = gix_attributes::search::Outcome::default();
    actual.initialize(&collection);
    for (rela_path, expected) in (baseline::Expectations { lines: input.lines() }) {
        actual.reset();
        let has_match = group.pattern_matching_relative_path(rela_path, case, None, &mut actual);
        assert_references(&actual);
        let actual: Vec<_> = actual
            .iter()
            .filter_map(|m| (!m.assignment.state.is_unspecified()).then_some(m.assignment))
            .collect();
        assert_eq!(actual, expected, "we have the same matches: {rela_path:?}");
        assert_ne!(has_match, actual.is_empty());
    }

    Ok(())
}

fn assert_references(out: &Outcome) {
    for m in out.iter() {
        if let Some(source) = m.kind.source_id() {
            let sm = out
                .match_by_id(source)
                .expect("sources are always available in the outcome");
            assert_ne!(
                sm.assignment.name, m.assignment.name,
                "it's impossible to resolve to ourselves"
            );
        }
    }
}

#[test]
fn all_attributes_are_listed_in_declaration_order() -> crate::Result {
    let (mut group, mut collection, base, input) = baseline::user_attributes("lookup-order")?;

    let mut buf = Vec::new();
    group.add_patterns_file(
        base.join(".gitattributes"),
        false,
        None,
        &mut buf,
        &mut collection,
        true, /* use macros */
    )?;

    let mut out = Outcome::default();
    out.initialize(&collection);
    let mut alt = Outcome::default();
    alt.initialize(&collection);

    let mut orders = collection
        .iter()
        .map(|attr| {
            (
                attr.0,
                attr.1.id,
                attr.1
                    .macro_attributes
                    .iter()
                    .map(|attr| (attr.id, attr.inner.name.as_ref()))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    orders.sort_by_key(|t| t.1);
    assert_eq!(
        orders,
        [
            (
                "binary",
                AttributeId(0),
                assignments([("diff", 1), ("merge", 2), ("text", 3)]).collect()
            ),
            ("diff", AttributeId(1), vec![]),
            ("merge", AttributeId(2), vec![]),
            ("text", AttributeId(3), vec![]),
            ("my-text", AttributeId(4), assignments(Some(("text", 3))).collect()),
            (
                "my-binary",
                AttributeId(5),
                assignments([("binary", 0), ("macro-overridden", 11), ("recursive", 9)]).collect()
            ),
            ("location", AttributeId(6), vec![]),
            (
                "b-cycle",
                AttributeId(7),
                assignments([("a-cycle", 8), ("my-text", 4)]).collect()
            ),
            (
                "a-cycle",
                AttributeId(8),
                assignments([("b-cycle", 7), ("my-binary", 5)]).collect()
            ),
            (
                "recursive",
                AttributeId(9),
                assignments(Some(("recursively-assigned-attr", 10))).collect()
            ),
            ("recursively-assigned-attr", AttributeId(10), vec![]),
            ("macro-overridden", AttributeId(11), vec![]),
            ("other", AttributeId(12), vec![])
        ],
        "binary is built-in, macros come first then their attributes (or macros)\
         , macros can be overridden, and it's exactly in declaration order"
    );

    for (rela_path, expected) in (baseline::Expectations { lines: input.lines() }) {
        out.reset();
        group.pattern_matching_relative_path(rela_path, Case::Sensitive, None, &mut out);
        assert_references(&out);
        let actual: Vec<_> = out.iter().map(|m| m.assignment).collect();
        assert_eq!(
            by_name(actual.clone()),
            by_name(expected),
            "{rela_path}: the order of everything matches perfectly"
        );
        out.copy_into(&collection, &mut alt);
        let alt_actual: Vec<_> = alt.iter().map(|m| m.assignment).collect();
        assert_eq!(alt_actual, actual);
    }
    assert_eq!(
        out.iter().count(),
        collection.iter().count(),
        "we manage to fill in all attributes in this baseline at least"
    );
    Ok(())
}

#[test]
fn given_attributes_are_made_available_in_given_order() -> crate::Result {
    let (mut group, mut collection, base, input) =
        baseline::user_attributes_named_baseline("lookup-order", "baseline.selected")?;

    let mut buf = Vec::new();
    group.add_patterns_file(
        base.join(".gitattributes"),
        false,
        None,
        &mut buf,
        &mut collection,
        true, /* use macros */
    )?;

    let mut out = Outcome::default();
    out.initialize_with_selection(&collection, ["my-binary", "recursive", "unspecified"]);
    let mut alt = Outcome::default();
    alt.initialize_with_selection(&collection, ["my-binary", "unspecified"]);

    for (rela_path, expected) in (baseline::Expectations { lines: input.lines() }) {
        out.reset();
        group.pattern_matching_relative_path(rela_path, Case::Sensitive, None, &mut out);
        assert_references(&out);
        let actual: Vec<_> = out.iter_selected().map(|m| m.assignment).collect();
        assert_eq!(
            actual, expected,
            "{rela_path}: the order of everything matches perfectly"
        );
        out.copy_into(&collection, &mut alt);
        let alt_actual: Vec<_> = alt.iter_selected().map(|m| m.assignment).collect();
        assert_eq!(alt_actual[0], actual[0]);
        assert_eq!(alt_actual[1], actual[2]);
    }
    assert_eq!(
        out.iter().count(),
        6,
        "the search stops early, leaving many attributes unspecified"
    );
    Ok(())
}

#[test]
fn size_of_outcome() {
    let actual = std::mem::size_of::<Outcome>();
    let expected = 840;
    assert!(
        size_ok(actual, expected),
        "it's quite big, shouldn't change without us noticing: {actual} <~ {expected}"
    );
}

fn by_name(assignments: Vec<AssignmentRef>) -> BTreeMap<NameRef, StateRef> {
    assignments.into_iter().map(|a| (a.name, a.state)).collect()
}

fn assignments<'a>(
    input: impl IntoIterator<Item = (&'a str, usize)> + 'a,
) -> impl Iterator<Item = (AttributeId, gix_attributes::NameRef<'a>)> + 'a {
    input.into_iter().map(|(name, order)| {
        (
            AttributeId(order),
            gix_attributes::NameRef::try_from(BStr::new(name)).expect("valid name"),
        )
    })
}

mod baseline {
    use std::path::PathBuf;

    use bstr::{BStr, ByteSlice};
    use gix_attributes::{AssignmentRef, StateRef, search::MetadataCollection};

    /// Read user-attributes and baseline in one go.
    pub fn user_attributes_named_baseline(
        name: &str,
        baseline: &str,
    ) -> crate::Result<(gix_attributes::Search, MetadataCollection, PathBuf, Vec<u8>)> {
        let dir = gix_testtools::scripted_fixture_read_only("make_attributes_baseline.sh")?;
        let base = dir.join(name);
        let input = std::fs::read(base.join(baseline))?;

        let mut buf = Vec::new();
        let mut collection = MetadataCollection::default();
        let group = gix_attributes::Search::new_globals([base.join("user.attributes")], &mut buf, &mut collection)?;

        Ok((group, collection, base, input))
    }

    /// Read user-attributes and baseline in one go.
    pub fn user_attributes(
        name: &str,
    ) -> crate::Result<(gix_attributes::Search, MetadataCollection, PathBuf, Vec<u8>)> {
        user_attributes_named_baseline(name, "baseline")
    }

    pub struct Expectations<'a> {
        pub lines: bstr::Lines<'a>,
    }

    impl<'a> Iterator for Expectations<'a> {
        type Item = (
            &'a BStr,
            // Names might refer to attributes or macros
            Vec<AssignmentRef<'a>>,
        );

        fn next(&mut self) -> Option<Self::Item> {
            let path = self.lines.next()?;
            let mut assignments = Vec::new();
            loop {
                let line = self.lines.next()?;
                if line.is_empty() {
                    return Some((path.as_bstr(), assignments));
                }

                let mut prev = None;
                let mut tokens = line.splitn(3, |b| {
                    let is_match = *b == b' ' && prev.take() == Some(b':');
                    prev = Some(*b);
                    is_match
                });

                if let Some(((_path, attr), info)) = tokens.next().zip(tokens.next()).zip(tokens.next()) {
                    let state = match info {
                        b"set" => StateRef::Set,
                        b"unset" => StateRef::Unset,
                        b"unspecified" => StateRef::Unspecified,
                        _ => StateRef::from_bytes(info),
                    };
                    let attr = attr.trim_end_with(|b| b == ':');
                    assignments.push(AssignmentRef {
                        name: gix_attributes::NameRef::try_from(attr.as_bstr()).expect("valid attributes"),
                        state,
                    });
                } else {
                    unreachable!("invalid line format: {line:?}", line = line.as_bstr())
                }
            }
        }
    }
}

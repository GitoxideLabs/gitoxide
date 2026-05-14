use bstr::ByteSlice;
use gix_attributes::search::Outcome;
use gix_worktree::stack::state;

use crate::stack::probe_case;

#[test]
fn baseline() -> crate::Result {
    // Due to the way our setup differs from gits dynamic stack (which involves trying to read files from disk
    // by path) we can only test one case baseline, so we require multiple platforms (or filesystems) to run this.
    let case = probe_case()?;
    let dir = crate::scripted_fixture_read_only("make_attributes_baseline.sh")?;
    let base = dir.join("basics");
    let git_dir = base.join(".git");

    let mut buf = Vec::new();
    let mut collection = gix_attributes::search::MetadataCollection::default();
    let state = gix_worktree::stack::State::for_checkout(
        false,
        gix_worktree::validate::path::component::Options {
            protect_windows: false,
            protect_ntfs: false,
            ..Default::default()
        },
        state::Attributes::new(
            gix_attributes::Search::new_globals([base.join("user.attributes")], &mut buf, &mut collection)?,
            Some(git_dir.join("info").join("attributes")),
            gix_worktree::stack::state::attributes::Source::WorktreeThenIdMapping,
            collection,
        ),
    );

    let mut cache = gix_worktree::Stack::new(&base, state, case, buf, vec![]);

    let mut actual = cache.attribute_matches();
    let input = std::fs::read(base.join("baseline"))?;
    for (rela_path, expected) in (baseline::Expectations { lines: input.lines() }) {
        let entry = cache.at_entry(rela_path, None, &gix_object::find::Never)?;
        let has_match = entry.matching_attributes(&mut actual);

        assert_eq!(
            has_match,
            !expected.is_empty(),
            "matches are reported when git reports them, too"
        );
        assert_references(&actual);
        assert_eq!(actual.iter_selected().count(), 0, "no selection made yet");
        let actual: Vec<_> = actual
            .iter()
            .filter_map(|m| (!m.assignment.state.is_unspecified()).then_some(m.assignment))
            .collect();
        assert_eq!(actual, expected, "we have the same matches: {rela_path:?}");
        assert_eq!(has_match, !actual.is_empty());
    }

    let mut actual = cache.selected_attribute_matches(["info", "test"]);
    let input = std::fs::read(base.join("baseline.selected"))?;
    for (rela_path, expected) in (baseline::Expectations { lines: input.lines() }) {
        let entry = cache.at_entry(rela_path, None, &gix_object::find::Never)?;
        let has_match = entry.matching_attributes(&mut actual);

        assert_eq!(
            has_match,
            !expected.is_empty(),
            "matches are reported when git reports them, too"
        );
        assert_references(&actual);
        let actual: Vec<_> = actual.iter_selected().map(|m| m.assignment).collect();
        assert_eq!(actual, expected, "we have the same matches: {rela_path:?}");
        assert_eq!(has_match, !actual.is_empty());
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

mod baseline {
    use bstr::{BStr, ByteSlice};
    use gix_attributes::{AssignmentRef, StateRef};

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

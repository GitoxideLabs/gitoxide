use bstr::ByteSlice;
use gix_hash::ObjectId;
use gix_refspec::{MatchGroup, match_group::Item, parse::Operation};
use gix_testtools::TestResult;

fn agrees_with_git(group: &str, expected_cases: usize) -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("reverse_baseline.sh")?;
    let commit_id = std::fs::read(fixture.join("commit-id"))?;
    let commit_id = ObjectId::from_hex(commit_id.trim().as_bytes())?;
    let mut count = 0;
    for case in std::fs::read_dir(fixture.join(group))? {
        let case = case?.path();
        let spec = std::fs::read(case.join("refspec"))?;
        let tracking_ref = std::fs::read(case.join("tracking-ref"))?;
        let expected = std::fs::read(case.join("baseline.git"))?;
        let parsed = gix_refspec::parse(spec.trim().as_bstr(), Operation::Fetch)?;
        let outcome = MatchGroup::from_fetch_specs([parsed]).match_rhs(std::iter::once(Item {
            full_ref_name: tracking_ref.trim().as_bstr(),
            target: Some(&commit_id),
            object: Some(&commit_id),
        }));
        let sources: Vec<_> = outcome
            .mappings
            .iter()
            .map(|mapping| {
                assert_eq!(
                    mapping.rhs.as_deref(),
                    Some(tracking_ref.trim().as_bstr()),
                    "reverse mapping retains the queried destination"
                );
                mapping.lhs.to_string()
            })
            .collect();
        let expected: Vec<_> = expected.lines().map(|line| line.to_str()).collect::<Result<_, _>>()?;
        assert_eq!(
            sources,
            expected,
            "Git reverse-maps {} to {} as recorded in {}",
            spec.trim().as_bstr(),
            tracking_ref.trim().as_bstr(),
            case.display()
        );
        count += 1;
    }
    assert_eq!(count, expected_cases, "all cases in `{group}` have been checked");
    Ok(())
}

#[test]
fn exact_sources_preserve_their_parsed_spelling() -> TestResult {
    agrees_with_git("exact-sources", 5)
}

#[test]
fn hexadecimal_sources_preserve_bytes_without_matching_the_target() -> TestResult {
    agrees_with_git("hexadecimal-sources", 2)
}

#[test]
fn glob_sources_substitute_only_the_captured_bytes() -> TestResult {
    agrees_with_git("glob-sources", 6)
}

#[test]
fn exact_destinations_match_names_not_abbreviations_or_object_ids() -> TestResult {
    agrees_with_git("exact-destinations", 7)
}

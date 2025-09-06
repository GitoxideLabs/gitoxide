use gix::bstr::BString;
use std::num::NonZero;

#[test]
fn blame_simple() -> crate::Result {
    let repo = crate::named_repo("make_blame_repo.sh")?;

    let suspect = repo.head_id()?;
    let outcome = repo.blame_file("simple.txt".into(), suspect, Default::default())?;

    assert_eq!(outcome.entries.len(), 4);

    Ok(())
}

#[test]
fn blame_simple_with_options() -> crate::Result {
    let repo = crate::named_repo("make_blame_repo.sh")?;

    let options = gix::blame::Options {
        range: gix::blame::BlameRanges::from_range(1..=2),
        ..Default::default()
    };

    let suspect = repo.head_id()?;
    let outcome = repo.blame_file("simple.txt".into(), suspect, options)?;

    assert_eq!(outcome.entries.len(), 2);

    let entries_with_lines: Vec<_> = outcome.entries_with_lines().collect();

    assert!(matches!(
        entries_with_lines.as_slice(),
        &[
            (
                gix::blame::BlameEntry {
                    start_in_blamed_file: 0,
                    start_in_source_file: 0,
                    source_file_name: None,
                    ..
                },
                _,
            ),
            (
                gix::blame::BlameEntry {
                    start_in_blamed_file: 1,
                    start_in_source_file: 1,
                    source_file_name: None,
                    ..
                },
                _,
            )
        ]
    ));

    assert_eq!(entries_with_lines[0].0.len, NonZero::new(1).unwrap());
    assert_eq!(entries_with_lines[1].0.len, NonZero::new(1).unwrap());

    assert_eq!(entries_with_lines[0].1, vec![BString::new("line 1\n".into())]);
    assert_eq!(entries_with_lines[1].1, vec![BString::new("line 2\n".into())]);

    Ok(())
}

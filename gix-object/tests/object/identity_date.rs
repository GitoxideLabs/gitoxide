use gix_object::{CommitRef, CommitRefIter, TagRef, TagRefIter};

#[test]
fn trailing_fields_preserve_the_first_date_and_timezone() -> gix_testtools::TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_trailing_signature_dates.sh")?;
    let hash_kind = crate::fixture_hash_kind();
    for offset in ["+0200", "-0530"] {
        let commit_bytes = std::fs::read(root.join(format!("commit{offset}")))?;
        let commit = CommitRef::from_bytes(&commit_bytes, hash_kind)?;
        let commit_dates = std::fs::read_to_string(root.join(format!("commit{offset}.dates")))?;
        let (author_date, committer_date) = commit_dates
            .trim_end()
            .split_once('\n')
            .expect("Git outputs the author and committer dates on separate lines");
        for (signature, expected) in [
            (commit.author()?, author_date),
            (commit.committer()?, committer_date),
            (
                CommitRefIter::from_bytes(&commit_bytes, hash_kind).author()?,
                author_date,
            ),
            (
                CommitRefIter::from_bytes(&commit_bytes, hash_kind).committer()?,
                committer_date,
            ),
        ] {
            assert_eq!(
                signature.time()?.format(gix_date::time::Format::Raw)?,
                expected,
                "decoded and iterated commit signatures use Git's first date and timezone"
            );
        }
        assert_eq!(
            commit.time()?.format(gix_date::time::Format::Raw)?,
            committer_date,
            "the commit time convenience accessor agrees with Git"
        );

        let tag_bytes = std::fs::read(root.join(format!("tag{offset}")))?;
        let tag = TagRef::from_bytes(&tag_bytes, hash_kind)?;
        let tag_date = std::fs::read_to_string(root.join(format!("tag{offset}.date")))?;
        for tagger in [tag.tagger()?, TagRefIter::from_bytes(&tag_bytes, hash_kind).tagger()?] {
            assert_eq!(
                tagger
                    .expect("the fixture contains a tagger")
                    .time()?
                    .format(gix_date::time::Format::Raw)?,
                tag_date.trim_end(),
                "decoded and iterated taggers use Git's first date and timezone"
            );
        }
    }
    Ok(())
}

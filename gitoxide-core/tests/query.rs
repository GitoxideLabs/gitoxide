use gix_testtools::TestResult;

#[test]
fn rewrites_compute_line_counts_only_for_edited_text() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = gix_testtools::scripted_fixture_writable("query-rewrites.sh")?;
    let engine = gitoxide_core::query::prepare(
        fixture.path(),
        gix::progress::Discard,
        std::io::sink(),
        gitoxide_core::query::Options {
            object_cache_size_mb: 1,
            find_copies_harder: false,
            threads: Some(1),
        },
    )?;
    drop(engine);

    let con = rusqlite::Connection::open(fixture.path().join(".git/ein.query"))?;
    let mut statement = con.prepare(
        "SELECT dest.file_path, source.file_path, mode, has_diff,
                lines_added, lines_removed, lines_before, lines_after
         FROM commit_file
         JOIN files AS dest ON dest.file_id = commit_file.file_id
         LEFT JOIN files AS source ON source.file_id = commit_file.source_file_id
         WHERE dest.file_path LIKE 'renamed-%' OR dest.file_path LIKE 'copied-%'
         ORDER BY dest.file_path",
    )?;
    let actual = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, u8>(2)?,
                row.get::<_, bool>(3)?,
                row.get::<_, usize>(4)?,
                row.get::<_, usize>(5)?,
                row.get::<_, usize>(6)?,
                row.get::<_, usize>(7)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(
        actual,
        [
            ("copied-edited".into(), None, 1, true, 21, 0, 0, 21),
            ("copied-exact".into(), Some("source".into()), 5, false, 0, 0, 0, 0),
            ("renamed-binary".into(), Some("binary".into()), 4, false, 0, 0, 0, 0),
            ("renamed-exact".into(), Some("exact".into()), 4, false, 0, 0, 0, 0),
            ("renamed-large".into(), Some("large".into()), 4, false, 0, 0, 0, 0),
            ("renamed-text".into(), Some("text".into()), 4, true, 1, 0, 20, 21),
        ],
        "query stores lazy text line counts, skips exact/binary/large diffs, and keeps copies exact-only"
    );
    Ok(())
}

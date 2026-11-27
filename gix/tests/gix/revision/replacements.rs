use gix::revision::walk::Sorting;
use gix_testtools::TestResult;

fn ids(walk: gix::revision::Walk<'_>) -> gix::Result<Vec<String>> {
    walk.map(|commit| commit.map(|commit| commit.id.to_string())).collect()
}

fn history(repo: &gix::Repository) -> gix::Result<Vec<String>> {
    let head_commit_id = repo.head_id()?;
    let expected = ids(repo.rev_walk([head_commit_id]).all()?)?;
    assert_eq!(
        ids(repo.rev_walk([head_commit_id]).sorting(Sorting::TopoOrder).all()?)?,
        expected,
        "both sorting modes follow the same replacement ancestry"
    );
    Ok(expected)
}

fn baseline(path: impl AsRef<std::path::Path>) -> std::io::Result<Vec<String>> {
    Ok(std::fs::read_to_string(path)?.lines().map(str::to_owned).collect())
}

#[test]
fn replacements_match_git_with_default_opening_and_raw_commit_graphs() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = gix_testtools::scripted_fixture_read_only("make_replacement_refs.sh")?;
    for name in ["repo", "bare"] {
        let path = fixture.join(name);
        let original = baseline(path.join("original.baseline"))?;
        let connected = baseline(path.join("connected.baseline"))?;
        assert_eq!(connected.len(), 4, "Git connects both original segments");

        let before = gix::open(fixture.join(format!("{name}-original")))?;
        assert_eq!(original.len(), 2, "the raw history ends at the squash root");
        assert!(before.commit_graph_if_enabled()?.is_some(), "the raw graph is present");
        assert_eq!(history(&before)?, original, "there are no replacements yet");

        for repo in [gix::open(&path)?, gix::discover(&path)?] {
            assert_eq!(history(&repo)?, connected, "default opening agrees with Git");
            assert!(
                repo.commit_graph_if_enabled()?.is_none(),
                "raw graphs are not opened automatically with replacements"
            );
            let tip_commit_id = repo.head_id()?;
            for sorting in [Sorting::BreadthFirst, Sorting::TopoOrder, Sorting::DateOrder] {
                let walk = || repo.rev_walk([tip_commit_id]).sorting(sorting);
                assert_eq!(
                    ids(walk().use_commit_graph(true).all()?)?,
                    connected,
                    "enabling graph use never opens a raw graph automatically with replacements"
                );
                for (override_graph, expected) in [(None, &original), (Some(false), &connected)] {
                    assert_eq!(
                        ids(walk()
                            .with_commit_graph(Some(repo.commit_graph()?))
                            .use_commit_graph(override_graph)
                            .all()?)?,
                        *expected,
                        "a supplied graph takes precedence over replacements unless disabled ({override_graph:?})"
                    );
                }
            }
            let mut graph = repo.revision_graph(None)?.with_owned_cache(Some(repo.commit_graph()?));
            let archive_id = repo.rev_parse_single("archive")?;
            assert_eq!(
                repo.merge_base_with_graph(tip_commit_id, archive_id, &mut graph)?,
                None,
                "lower-level repository graphs also trust a supplied graph over replacements"
            );
            let walk = || repo.rev_walk([tip_commit_id]).with_hidden([archive_id]);
            let mut cache = repo.rev_walk_cache();
            assert_eq!(
                ids(walk().with_cache(&mut cache).all()?)?,
                ids(walk().all()?)?,
                "hidden walks with a cache follow replacement ancestry"
            );
        }

        for enabled in [false, true] {
            let value = format!("core.useReplaceRefs={enabled}");
            let repo = gix::open_opts(&path, gix::open::Options::isolated().cli_overrides([value.as_str()]))?;
            let expected = baseline(path.join(format!("configured-{enabled}.baseline")))?;
            assert_eq!(
                history(&repo)?,
                expected,
                "explicit replacement configuration agrees with Git"
            );
            assert_eq!(
                repo.commit_graph_if_enabled()?.is_some(),
                !enabled,
                "raw history can still use its commit-graph"
            );
        }
        let mut raw = gix::open(&path)?;
        raw.objects.ignore_replacements = true;
        assert_eq!(
            history(&raw)?,
            original,
            "per-handle disablement preserves original ancestry"
        );
        assert!(
            raw.commit_graph_if_enabled()?.is_some(),
            "raw handles can use the cache"
        );
        for value in ["1", "0", "", "false"] {
            let _environment = gix_testtools::Env::new().set("GIT_NO_REPLACE_OBJECTS", value);
            assert_eq!(
                history(&gix::open(&path)?)?,
                original,
                "the Git environment switch also disables replacements"
            );
        }
        let without_graph = gix::open(fixture.join(format!("{name}-without-graph")))?;
        assert!(
            without_graph.commit_graph().is_err(),
            "the graph-free variant has no commit-graph file"
        );
        assert_eq!(
            history(&without_graph)?,
            connected,
            "replacements also work without a graph file"
        );
    }
    Ok(())
}

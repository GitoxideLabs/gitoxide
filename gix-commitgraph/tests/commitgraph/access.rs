use crate::{check_common, graph_and_expected, graph_and_expected_named};

#[test]
fn legacy_graph_has_no_generation_numbers() -> gix_testtools::TestResult {
    let repo_dir = gix_testtools::scripted_fixture_read_only("single_commit.sh")?;
    let mut data = std::fs::read(repo_dir.join(".git/objects/info/commit-graph"))?;
    let hash_len = gix_testtools::object_hash().len_in_bytes();
    let generation_offset = crate::chunk_range(&data, *b"CDAT")?.start + hash_len + 8;
    let timestamp_bits = u32::from_be_bytes(data[generation_offset..][..4].try_into()?) & 3;
    data[generation_offset..][..4].copy_from_slice(&timestamp_bits.to_be_bytes());
    crate::update_checksum(&mut data)?;
    let graph = gix_commitgraph::Graph::new(vec![crate::mapped_file(&data, "commit-graph")?])?;
    assert!(
        !graph.generation_numbers_available(),
        "a legacy graph stores zero instead of computed generation numbers"
    );
    Ok(())
}

#[test]
fn single_parent() {
    let (cg, refs) = graph_and_expected("single_parent.sh", &["parent", "child"]);
    check_common(&cg, &refs);

    assert_eq!(cg.commit_at(refs["parent"].pos()).generation(), 1);
    assert_eq!(cg.commit_at(refs["child"].pos()).generation(), 2);
}

#[test]
fn single_commit_huge_dates_generation_v2_also_do_not_allow_huge_dates() {
    let (cg, refs) = graph_and_expected_named("single_commit_huge_dates.sh", "v2", &["HEAD"]);
    let info = &refs["HEAD"];
    let actual = cg.commit_by_id(info.id).expect("present");
    assert_eq!(
        actual.committer_timestamp(),
        1,
        "overflow happened, can't represent huge dates"
    );
    assert_eq!(
        info.time.seconds, 68719476737,
        "this is the value we would want to see, but it's not possible in V2 either, as that is just about generations"
    );
    assert_eq!(actual.generation(), 1, "generations are fine though");
}

#[test]
fn single_commit_huge_dates_overflow_v1() {
    let (cg, refs) = graph_and_expected_named("single_commit_huge_dates.sh", "v1", &["HEAD"]);
    let info = &refs["HEAD"];
    let actual = cg.commit_by_id(info.id).expect("present");
    assert_eq!(actual.committer_timestamp(), 1, "overflow happened");
    assert_eq!(
        info.time.seconds, 68719476737,
        "this is the value we would want to see, but it's not possible in V1"
    );
    assert_eq!(actual.generation(), 1, "generations are fine though");
}

#[test]
fn single_commit_future_64bit_dates_work() {
    let (cg, refs) = graph_and_expected_named("single_commit_huge_dates.sh", "max-date", &["HEAD"]);
    let info = &refs["HEAD"];
    let actual = cg.commit_by_id(info.id).expect("present");
    assert_eq!(
        actual.committer_timestamp(),
        info.time.seconds.try_into().expect("timestamps in bound"),
        "this is close the highest representable value in the graph, like year 2500, so we are good for longer than I should care about"
    );
    assert_eq!(actual.generation(), 1);
}

#[test]
fn generation_numbers_overflow_is_handled_in_chained_graph() {
    let names = ["extra", "old-2", "future-2", "old-1", "future-1"];
    let (cg, mut refs) = graph_and_expected("generation_number_overflow.sh", &names);
    for (r, expected) in names
        .iter()
        .map(|n| refs.remove(n.to_owned()).expect("present"))
        .zip((1..=5).rev())
    {
        assert_eq!(
            cg.commit_by_id(r.id).expect("present").generation(),
            expected,
            "actually, this test seems to have valid generation numbers from the get-go. How to repro the actual issue?"
        );
    }
}

#[test]
fn corrected_commit_dates_include_overflow_offsets_in_every_layer() -> gix_testtools::TestResult {
    let names = ["future-1", "old-1", "future-2", "old-2", "extra"];
    let (graph, refs) = graph_and_expected("generation_number_overflow.sh", &names);
    assert!(
        graph.corrected_commit_dates_available(),
        "every split layer has generation version 2"
    );
    for (name, expected) in names.into_iter().zip(4_147_483_646..=4_147_483_650) {
        assert_eq!(
            graph
                .commit_by_id(refs[name].id())
                .expect("fixture ref is in graph")
                .corrected_commit_date()?,
            Some(expected),
            "{name}: corrected dates increase even when raw timestamps go backwards"
        );
    }

    let (legacy, refs) = graph_and_expected_named("single_commit_huge_dates.sh", "v1", &["HEAD"]);
    assert!(
        !legacy.corrected_commit_dates_available(),
        "version 1 contains only topological generations"
    );
    assert_eq!(
        legacy
            .commit_by_id(refs["HEAD"].id())
            .expect("HEAD is in graph")
            .corrected_commit_date()?,
        None
    );
    Ok(())
}

#[test]
fn malformed_corrected_date_chunks_and_offsets_are_corruption() -> gix_testtools::TestResult {
    let root = gix_testtools::scripted_fixture_read_only("generation_number_overflow.sh")?;
    let data = std::fs::read_dir(root.join(".git/objects/info/commit-graphs"))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .filter(|path| path.extension().is_some_and(|extension| extension == "graph"))
        .map(std::fs::read)
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .find(|data| crate::chunk_range(data, *b"GDO2").is_ok())
        .expect("the fixture contains overflowing offsets");
    let table_end = 8 + usize::from(data[6]) * gix_chunk::file::Index::ENTRY_SIZE;
    for kind in [*b"GDA2", *b"GDO2"] {
        let mut truncated = data.clone();
        let range = crate::chunk_range(&data, kind)?;
        truncated.remove(range.end - 1);
        for entry in truncated[8..table_end + gix_chunk::file::Index::ENTRY_SIZE]
            .as_chunks_mut::<{ gix_chunk::file::Index::ENTRY_SIZE }>()
            .0
            .iter_mut()
        {
            let offset = u64::from_be_bytes(entry[4..].try_into()?);
            if offset >= range.end as u64 {
                entry[4..].copy_from_slice(&(offset - 1).to_be_bytes());
            }
        }
        let err =
            crate::mapped_file(&truncated, "commit-graph").expect_err("generation chunks have fixed-width records");
        assert!(err.is_corrupted(), "partial generation records are corruption: {err}");
        assert!(
            err.to_string().contains("generation"),
            "the malformed generation chunk was rejected: {err}"
        );
    }

    let generation_data = crate::chunk_range(&data, *b"GDA2")?;
    let overflow = crate::chunk_range(&data, *b"GDO2")?;
    let file = crate::mapped_file(&data, "commit-graph")?;
    let position = file
        .iter_commits()
        .position(|commit| commit.committer_timestamp() > 0)
        .expect("a positive date makes addition overflow observable");
    let offset = generation_data.start + position * 4;
    for case in ["missing", "out-of-bounds", "overflow", "zero"] {
        let mut malformed = data.clone();
        malformed[offset..offset + 4].copy_from_slice(&0x8000_0000u32.to_be_bytes());
        match case {
            "missing" => {
                let entry = malformed[8..table_end]
                    .as_chunks_mut::<{ gix_chunk::file::Index::ENTRY_SIZE }>()
                    .0
                    .iter_mut()
                    .find(|entry| &entry[..4] == b"GDO2")
                    .expect("overflow chunk exists");
                entry[..4].copy_from_slice(b"MISS");
            }
            "out-of-bounds" => malformed[offset..offset + 4].copy_from_slice(&u32::MAX.to_be_bytes()),
            "overflow" => malformed[overflow.start..overflow.start + 8].copy_from_slice(&u64::MAX.to_be_bytes()),
            "zero" => {
                malformed[offset..offset + 4].fill(0);
                let hash_len = file.object_hash().len_in_bytes();
                let timestamp = crate::chunk_range(&data, *b"CDAT")?.start + position * (hash_len + 16) + hash_len + 8;
                let generation = u32::from_be_bytes(malformed[timestamp..timestamp + 4].try_into()?) & !3;
                malformed[timestamp..timestamp + 4].copy_from_slice(&generation.to_be_bytes());
                malformed[timestamp + 4..timestamp + 8].fill(0);
            }
            _ => unreachable!(),
        }
        let graph = crate::mapped_file(&malformed, "commit-graph")?;
        let err = graph
            .commit_at(gix_commitgraph::file::Position(position.try_into()?))
            .corrected_commit_date()
            .expect_err("malformed corrected dates are rejected on access");
        assert!(
            err.is_corrupted(),
            "{case}: malformed generation offsets are corruption: {err}"
        );
    }
    Ok(())
}

#[test]
fn octopus_merges() {
    let (cg, refs) = graph_and_expected(
        "octopus_merges.sh",
        &[
            "root",
            "parent1",
            "parent2",
            "parent3",
            "parent4",
            "three_parents",
            "four_parents",
        ],
    );
    check_common(&cg, &refs);

    assert_eq!(cg.commit_at(refs["root"].pos()).generation(), 1);
    assert_eq!(cg.commit_at(refs["parent1"].pos()).generation(), 2);
    assert_eq!(cg.commit_at(refs["parent2"].pos()).generation(), 2);
    assert_eq!(cg.commit_at(refs["parent3"].pos()).generation(), 2);
    assert_eq!(cg.commit_at(refs["parent4"].pos()).generation(), 2);
    assert_eq!(cg.commit_at(refs["three_parents"].pos()).generation(), 3);
    assert_eq!(cg.commit_at(refs["four_parents"].pos()).generation(), 3);
}

#[test]
fn single_commit() {
    let (cg, refs) = graph_and_expected("single_commit.sh", &["commit"]);
    check_common(&cg, &refs);

    assert_eq!(cg.commit_at(refs["commit"].pos()).generation(), 1);
}

#[test]
fn two_parents() {
    let (cg, refs) = graph_and_expected("two_parents.sh", &["parent1", "parent2", "child"]);
    check_common(&cg, &refs);

    assert_eq!(cg.commit_at(refs["parent1"].pos()).generation(), 1);
    assert_eq!(cg.commit_at(refs["parent2"].pos()).generation(), 1);
    assert_eq!(cg.commit_at(refs["child"].pos()).generation(), 2);
}

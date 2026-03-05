use crate::{check_common, graph_and_expected, graph_and_expected_named};
use gix_testtools::scripted_fixture_writable;
use std::{fs, path::Path};

fn should_skip_path_v2_unsupported() -> bool {
    // Git learned to read and write changed-path Bloom filter version 2 in 2.46.
    // Older binaries ignore the config in these fixtures and keep producing v1 data.
    // Use the raw version here instead of `should_skip_as_git_version_is_smaller_than()`,
    // which intentionally never skips on CI.
    *gix_testtools::GIT_VERSION < (2, 46, 0)
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

#[test]
fn changed_paths_v2_settings_are_read() {
    if should_skip_path_v2_unsupported() {
        return;
    }
    let (cg, _refs) = graph_and_expected("changed_paths_v2.sh", &["HEAD"]);
    let settings = cg
        .bloom_filter_settings()
        .expect("changed-path Bloom settings are available");
    assert_eq!(settings.hash_version, 2, "fixture explicitly requests v2 filters");
    assert_eq!(settings.bits_per_entry, 10, "git default bits per entry");
    assert_eq!(settings.num_hashes, 7, "git default hash count");
}

#[test]
fn incompatible_split_chain_prefers_top_layer_bloom_settings() {
    if should_skip_path_v2_unsupported() {
        return;
    }
    let (cg, _refs) = graph_and_expected("split_chain_changed_paths_mismatch.sh", &["c1", "c2"]);
    let settings = cg
        .bloom_filter_settings()
        .expect("top layer has changed-path Bloom settings");
    assert_eq!(settings.hash_version, 2, "top layer uses v2 and should remain usable");
}

#[test]
fn split_chain_uses_base_bloom_when_top_has_none() {
    if should_skip_path_v2_unsupported() {
        return;
    }
    let (cg, _refs) = graph_and_expected("split_chain_top_without_bloom.sh", &["c1", "c2"]);
    let settings = cg
        .bloom_filter_settings()
        .expect("base layer changed-path settings remain usable");
    assert_eq!(settings.hash_version, 2);
}

#[test]
fn bloom_is_disabled_if_bidx_chunk_is_missing() {
    let tmp = scripted_fixture_writable("changed_paths_v2.sh").expect("fixture available");
    mutate_commit_graph(tmp.path(), |data| {
        let entries = parse_chunk_table(data);
        let bidx = find_chunk_index(&entries, *b"BIDX").expect("BIDX present in fixture");
        set_chunk_id(data, bidx, *b"XIDX");
    });
    let graph = gix_commitgraph::Graph::from_info_dir(&info_dir(tmp.path())).expect("graph remains readable");
    assert!(graph.bloom_filter_settings().is_none(), "missing BIDX disables Bloom");
}

#[test]
fn bloom_is_disabled_if_bdat_chunk_is_missing() {
    let tmp = scripted_fixture_writable("changed_paths_v2.sh").expect("fixture available");
    mutate_commit_graph(tmp.path(), |data| {
        let entries = parse_chunk_table(data);
        let bdat = find_chunk_index(&entries, *b"BDAT").expect("BDAT present in fixture");
        set_chunk_id(data, bdat, *b"XDAT");
    });
    let graph = gix_commitgraph::Graph::from_info_dir(&info_dir(tmp.path())).expect("graph remains readable");
    assert!(graph.bloom_filter_settings().is_none(), "missing BDAT disables Bloom");
}

#[test]
fn bloom_is_disabled_if_bidx_is_too_small() {
    let tmp = scripted_fixture_writable("changed_paths_v2.sh").expect("fixture available");
    mutate_commit_graph(tmp.path(), |data| {
        let entries = parse_chunk_table(data);
        let bidx = find_chunk_index(&entries, *b"BIDX").expect("BIDX present in fixture");
        let bidx_offset = entries[bidx].offset;
        set_chunk_offset(data, bidx + 1, bidx_offset + 4);
    });
    let graph = gix_commitgraph::Graph::from_info_dir(&info_dir(tmp.path())).expect("graph remains readable");
    assert!(graph.bloom_filter_settings().is_none(), "too-small BIDX disables Bloom");
}

#[test]
fn bloom_is_disabled_if_bdat_is_too_small() {
    let tmp = scripted_fixture_writable("changed_paths_v2.sh").expect("fixture available");
    mutate_commit_graph(tmp.path(), |data| {
        let entries = parse_chunk_table(data);
        let bdat = find_chunk_index(&entries, *b"BDAT").expect("BDAT present in fixture");
        let next_offset = entries[bdat + 1].offset;
        set_chunk_offset(data, bdat, next_offset - 4);
    });
    let graph = gix_commitgraph::Graph::from_info_dir(&info_dir(tmp.path())).expect("graph remains readable");
    assert!(graph.bloom_filter_settings().is_none(), "too-small BDAT disables Bloom");
}

#[test]
fn bloom_is_disabled_if_bidx_offsets_are_invalid() {
    let tmp = scripted_fixture_writable("changed_paths_v2.sh").expect("fixture available");
    mutate_commit_graph(tmp.path(), |data| {
        let entries = parse_chunk_table(data);
        let bidx = find_chunk_index(&entries, *b"BIDX").expect("BIDX present in fixture");
        let start = entries[bidx].offset as usize;
        data[start..start + 4].copy_from_slice(&u32::MAX.to_be_bytes());
        data[start + 4..start + 8].copy_from_slice(&1u32.to_be_bytes());
    });
    let graph = gix_commitgraph::Graph::from_info_dir(&info_dir(tmp.path())).expect("graph remains readable");
    assert!(
        graph.bloom_filter_settings().is_none(),
        "out-of-range and decreasing BIDX offsets disable Bloom"
    );
}

#[derive(Clone, Copy)]
struct ChunkTableEntry {
    id: [u8; 4],
    offset: u64,
}

fn info_dir(repo_path: &Path) -> std::path::PathBuf {
    repo_path.join(".git").join("objects").join("info")
}

#[allow(clippy::permissions_set_readonly_false)]
fn mutate_commit_graph(repo_path: &Path, mutate: impl FnOnce(&mut [u8])) {
    let graph_path = info_dir(repo_path).join("commit-graph");
    let mut permissions = fs::metadata(&graph_path).expect("commit-graph metadata").permissions();
    permissions.set_readonly(false);
    fs::set_permissions(&graph_path, permissions).expect("set commit-graph writable");
    let mut data = fs::read(&graph_path).expect("read commit-graph fixture");
    mutate(&mut data);
    fs::write(graph_path, data).expect("rewrite mutated commit-graph");
}

fn parse_chunk_table(data: &[u8]) -> Vec<ChunkTableEntry> {
    let chunk_count = usize::from(data[6]);
    let mut out = Vec::with_capacity(chunk_count + 1);
    let table_start = 8;
    for idx in 0..=chunk_count {
        let entry_offset = table_start + idx * 12;
        let id = data[entry_offset..entry_offset + 4]
            .try_into()
            .expect("chunk id has 4 bytes");
        let offset = u64::from_be_bytes(
            data[entry_offset + 4..entry_offset + 12]
                .try_into()
                .expect("chunk offset has 8 bytes"),
        );
        out.push(ChunkTableEntry { id, offset });
    }
    out
}

fn find_chunk_index(entries: &[ChunkTableEntry], id: [u8; 4]) -> Option<usize> {
    entries.iter().position(|entry| entry.id == id)
}

fn set_chunk_id(data: &mut [u8], chunk_index: usize, id: [u8; 4]) {
    let entry_offset = 8 + chunk_index * 12;
    data[entry_offset..entry_offset + 4].copy_from_slice(&id);
}

fn set_chunk_offset(data: &mut [u8], chunk_index: usize, offset: u64) {
    let entry_offset = 8 + chunk_index * 12 + 4;
    data[entry_offset..entry_offset + 8].copy_from_slice(&offset.to_be_bytes());
}

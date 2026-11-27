use gix_commitgraph::{Generation, file};
use gix_error::TestResult;

#[test]
fn display_names_special_values() {
    for (generation, expected) in [
        (Generation::UNCOMPUTED, "uncomputed"),
        (Generation::MAX, "capped"),
        (Generation::INFINITY, "unavailable"),
    ] {
        assert_eq!(generation.to_string(), expected, "label for {generation:?}");
    }
}

#[test]
fn only_computed_unsaturated_generations_are_reliable() -> TestResult {
    let mut data = crate::fixture_commit_graph_bytes("single_commit.sh")?;
    let hash_len = gix_testtools::object_hash().len_in_bytes();
    let offset = crate::chunk_range(&data, *b"CDAT")?.start + hash_len + 8;
    let timestamp_bits = u32::from_be_bytes(data[offset..][..4].try_into()?) & 3;
    for (stored, expected) in [(1u32, "1"), (0x3fff_fffe, "1073741822")] {
        data[offset..][..4].copy_from_slice(&((stored << 2) | timestamp_bits).to_be_bytes());
        crate::update_checksum(&mut data)?;
        let generation = crate::mapped_file(&data, "commit-graph")?
            .commit_at(file::Position(0))
            .generation();
        assert_eq!(
            generation.to_string(),
            expected,
            "computed generations display as numbers"
        );
        assert_eq!(
            Some(generation).filter(Generation::is_reliable),
            Some(generation),
            "keep generation {generation}"
        );
    }
    for generation in [Generation::UNCOMPUTED, Generation::MAX, Generation::INFINITY] {
        assert_eq!(
            Some(generation).filter(Generation::is_reliable),
            None,
            "reject generation {generation}"
        );
    }
    Ok(())
}

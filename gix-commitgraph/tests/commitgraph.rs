use std::{
    collections::{HashMap, HashSet},
    hash::BuildHasher,
    io::{BufRead, Cursor},
    path::Path,
};

use gix_commitgraph::{Graph, Position as GraphPosition};
use gix_error::{ErrorExt, Message, Result, ResultExt, TestResult};
use gix_testtools::scripted_fixture_read_only;

mod access;

#[test]
fn missing_path_is_not_found() -> gix_testtools::Result {
    let dir = gix_testtools::tempfile::tempdir()?;
    let err = gix_commitgraph::at(dir.path().join("missing"))
        .err()
        .expect("a missing path cannot contain a commit-graph");
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&(err), &[(&(dir.path()).to_string_lossy(), "<tmp>")]), "callers can distinguish a missing optional cache from other failures", @"
    Could not access commit-graph path at \"<tmp>/missing\"

    Caused by:
        0: NotFound
    ");
    assert!(
        err.is_not_found(),
        "callers can distinguish a missing optional cache from other failures"
    );
    Ok(())
}

#[test]
fn checksum_mismatches_retain_their_classification() -> gix_testtools::Result {
    let repo = gix_testtools::scripted_fixture_writable("single_commit.sh")?;
    let mut data = std::fs::read(repo.path().join(".git/objects/info/commit-graph"))?;
    *data.last_mut().expect("the graph has a checksum trailer") ^= 1;
    // Git can make its graph read-only; corrupt a separate file.
    let path = repo.path().join("corrupt-commit-graph");
    std::fs::write(&path, data)?;

    let graph = gix_commitgraph::File::at(path)?;
    let err = graph.verify_checksum().expect_err("the checksum no longer matches");
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&(err), &[]), "a checksum mismatch is corruption", @"
    commit-graph checksum does not match

    Caused by:
        0: Hash was Oid(1), but should have been Oid(2)
    ");
    assert!(err.is_corrupted(), "a checksum mismatch is corruption");
    Ok(())
}

pub fn check_common(cg: &Graph, expected: &HashMap<String, RefInfo, impl BuildHasher>) {
    cg.verify_integrity(|_| Ok::<_, Message>(())).expect("graph is valid");
    assert_eq!(
        usize::try_from(cg.num_commits()).expect("an architecture able to hold 32 bits of integer"),
        expected.len()
    );
    for ref_info in expected.values() {
        assert_eq!(cg.id_at(ref_info.pos()), ref_info.id(), "id_at({})", ref_info.pos());
        assert_eq!(
            cg.lookup(ref_info.id()),
            Some(ref_info.pos()),
            "lookup({})",
            ref_info.id()
        );

        let expected_parents: Vec<_> = ref_info
            .parent_ids()
            .map(|id| {
                expected
                    .values()
                    .find(|item| item.id() == id)
                    .expect("find RefInfo by id")
            })
            .collect();

        let commit = cg.commit_at(ref_info.pos());
        assert_eq!(commit.id(), ref_info.id());
        assert_eq!(
            commit.committer_timestamp(),
            ref_info.time.seconds.try_into().expect("timestamp in bounds")
        );
        assert_eq!(commit.root_tree_id(), ref_info.root_tree_id());
        assert_eq!(
            commit.parent1().expect("failed to access commit's parent1"),
            expected_parents.iter().map(|x| x.pos()).next()
        );
        assert_eq!(
            commit
                .iter_parents()
                .collect::<std::result::Result<Vec<_>, _>>()
                .expect("failed to access commit's parents"),
            expected_parents.iter().map(|x| x.pos()).collect::<Vec<_>>()
        );
    }

    assert_eq!(
        cg.iter_ids().collect::<HashSet<_>>(),
        expected.values().map(RefInfo::id).collect::<HashSet<_>>()
    );
}

pub fn graph_and_expected(
    script_path: &str,
    refs: &[&'static str],
) -> (gix_commitgraph::Graph, HashMap<String, RefInfo>) {
    graph_and_expected_named(script_path, "", refs)
}

pub fn graph_and_expected_named(
    script_path: &str,
    name: &str,
    refs: &[&'static str],
) -> (gix_commitgraph::Graph, HashMap<String, RefInfo>) {
    let repo_dir = scripted_fixture_read_only(script_path)
        .expect("script succeeds all the time")
        .join(name);
    let expected = inspect_refs(&repo_dir, refs);
    let cg =
        Graph::from_info_dir(&repo_dir.join(".git").join("objects").join("info")).expect("graph present and valid");
    let object_hash = cg.object_hash();
    let any_ref = expected.values().next().expect("at least one ref");
    assert_eq!(
        object_hash,
        any_ref.id().kind(),
        "graph hash kind should match fixture object IDs"
    );

    (cg, expected)
}

pub struct RefInfo {
    id: gix_hash::ObjectId,
    pub time: gix_date::Time,
    parent_ids: Vec<gix_hash::ObjectId>,
    pos: GraphPosition,
    root_tree_id: gix_hash::ObjectId,
}

impl RefInfo {
    pub fn id(&self) -> &gix_hash::oid {
        &self.id
    }

    pub fn pos(&self) -> GraphPosition {
        self.pos
    }

    pub fn parent_ids(&self) -> impl Iterator<Item = &gix_hash::oid> {
        self.parent_ids.iter().map(AsRef::as_ref)
    }

    pub fn root_tree_id(&self) -> &gix_hash::oid {
        &self.root_tree_id
    }
}

fn inspect_refs(repo_dir: impl AsRef<Path>, refs: &[&'static str]) -> HashMap<String, RefInfo> {
    let output = gix_testtools::git_command(repo_dir)
        .arg("show")
        .arg("--no-patch")
        .arg("--pretty=format:%S %H %T %ct %P")
        .args(refs)
        .arg("--")
        .output()
        .expect("failed to execute `git show`");
    // Output format: <refname> <id> <tree_id> <parent_ids>
    let mut infos: Vec<_> = Cursor::new(output.stdout)
        .lines()
        .map(|x| x.expect("failed to read `git show` output"))
        .map(|x| {
            let parts = x.trim_end().split(' ').collect::<Vec<_>>();
            (
                parts[0].to_string(),
                gix_hash::ObjectId::from_hex(parts[1].as_bytes()).expect("40 bytes hex"),
                gix_hash::ObjectId::from_hex(parts[2].as_bytes()).expect("40 bytes hex"),
                gix_date::Time::new(parts[3].parse().expect("valid stamp"), 0),
                parts[4..]
                    .iter()
                    .map(|x| gix_hash::ObjectId::from_hex(x.as_bytes()).expect("40 bytes hex"))
                    .collect(),
            )
        })
        .collect();
    infos.sort_by_key(|x| x.1);

    let get_pos = |id: &gix_hash::oid| -> GraphPosition {
        let pos: u32 = infos
            .binary_search_by_key(&id, |x| &x.1)
            .expect("sorted_ids to contain id")
            .try_into()
            .expect("graph position to fit in u32");
        GraphPosition(pos)
    };

    infos
        .iter()
        .cloned()
        .map(|(name, id, root_tree_id, time, parent_ids)| {
            (
                name,
                RefInfo {
                    id,
                    parent_ids,
                    root_tree_id,
                    time,
                    pos: get_pos(&id),
                },
            )
        })
        .collect()
}

#[test]
fn malformed_file_data_is_corruption() -> TestResult {
    let data = fixture_commit_graph_bytes("single_commit.sh")?;
    let mut bad_signature = data.clone();
    bad_signature[0] ^= 1;
    let mut bad_count = data.clone();
    let fan = chunk_range(&data, *b"OIDF")?;
    bad_count[fan.end - 4..fan.end].copy_from_slice(&0u32.to_be_bytes());

    for (data, diagnostic) in [
        (b"CGPH".to_vec(), "Commit-graph file too small even for an empty graph"),
        (
            bad_signature,
            "Commit-graph file does not start with expected signature",
        ),
        (bad_count, "chunk contains 0 commits"),
        (
            data[..data.len() - 1].to_vec(),
            "Expected commit-graph trailer to contain",
        ),
    ] {
        let err = mapped_file(&data, "commit-graph").expect_err("the graph data is malformed");
        assert!(err.is_corrupted(), "malformed stored data is corruption: {err}");
        assert!(err.to_string().contains(diagnostic), "the intended check failed: {err}");
    }
    Ok(())
}

#[test]
fn chunk_errors_keep_their_causes() -> TestResult {
    let data = fixture_commit_graph_bytes("single_commit.sh")?;
    let mut no_chunks = data.clone();
    no_chunks[6] = 0;
    let err = mapped_file(&no_chunks, "commit-graph").expect_err("a graph needs its required chunks");
    assert!(err.is_corrupted(), "invalid stored chunk indices are corruption");
    assert!(
        err.is_validation(),
        "the chunk decoder's input-validation cause survives"
    );
    assert_eq!(
        err.probable_cause().to_string(),
        "Empty chunk indices are not allowed as the point of chunked files is to have chunks.",
        "the decoder's diagnostic remains the cause"
    );

    for kind in [*b"CDAT", *b"OIDF", *b"OIDL"] {
        let mut missing_chunk = data.clone();
        let table_end = 8 + usize::from(data[6]) * gix_chunk::file::Index::ENTRY_SIZE;
        let entry = missing_chunk[8..table_end]
            .as_chunks_mut::<{ gix_chunk::file::Index::ENTRY_SIZE }>()
            .0
            .iter_mut()
            .find(|entry| entry[..4] == kind)
            .expect("the fixture contains each required chunk");
        entry[..4].copy_from_slice(b"MISS");
        let err = mapped_file(&missing_chunk, "commit-graph").expect_err("a required chunk is missing");
        assert!(err.is_corrupted(), "a missing required chunk is corruption");
        assert_eq!(
            err.probable_cause().to_string(),
            format!(
                "Chunk named '{}' was not found in chunk file index",
                std::str::from_utf8(&kind)?
            ),
            "the chunk lookup's original diagnostic survives"
        );
    }
    Ok(())
}

#[test]
fn unsupported_headers_are_classified() -> TestResult {
    let data = fixture_commit_graph_bytes("single_commit.sh")?;
    for (offset, value, diagnostic) in [
        (4, 2, "Unsupported commit-graph file version: 2"),
        (5, 255, "Commit-graph file uses unsupported hash version: 255"),
    ] {
        let mut unsupported = data.clone();
        unsupported[offset] = value;
        let err = mapped_file(&unsupported, "commit-graph").expect_err("the header requests an unsupported format");
        assert_eq!(
            err.to_string(),
            diagnostic,
            "unsupported-format diagnostics are unchanged"
        );
        assert!(err.is_unsupported(), "a different format implementation is required");
        assert!(
            !err.is_corrupted() && !err.is_validation(),
            "unsupported does not mean malformed or invalid input"
        );
    }
    Ok(())
}

#[test]
fn initialization_io_errors_keep_their_sources() -> TestResult {
    let dir = gix_testtools::tempfile::tempdir()?;
    let missing = dir.path().join("missing");
    for err in [
        gix_commitgraph::File::at(&missing).expect_err("the graph file does not exist"),
        Graph::at(&missing).err().expect("the graph path does not exist"),
        Graph::from_commit_graphs_dir(dir.path())
            .err()
            .expect("the chain file does not exist"),
    ] {
        assert!(
            err.is_not_found(),
            "missing optional graph data remains distinguishable"
        );
        assert!(!err.is_corrupted(), "an I/O failure does not establish corruption");
        assert!(
            !err.is_validation(),
            "an I/O failure is not a constructor contract violation"
        );
        assert_eq!(
            err.downcast_any_ref::<std::io::Error>()
                .expect("the native I/O source is preserved")
                .kind(),
            std::io::ErrorKind::NotFound,
            "the original I/O kind survives context"
        );
    }
    Ok(())
}

#[test]
fn empty_graph_inputs_differ_from_empty_stored_chains() -> TestResult {
    let err = Graph::new(Vec::new())
        .err()
        .expect("a graph requires at least one file");
    assert!(
        err.is_validation(),
        "an empty caller-supplied list violates the constructor contract"
    );

    let dir = gix_testtools::tempfile::tempdir()?;
    std::fs::write(dir.path().join("commit-graph-chain"), b"")?;
    let err = Graph::from_commit_graphs_dir(dir.path())
        .err()
        .expect("an empty stored chain cannot describe a graph");
    assert!(err.is_corrupted(), "an empty stored chain is malformed data");
    assert!(err.is_validation(), "disk assembly preserves its constructor failure");
    assert_eq!(
        err.probable_cause().to_string(),
        "Commit-graph must contain at least one file",
        "the constructor diagnostic remains the cause"
    );
    Ok(())
}

#[test]
fn verification_errors_classify_data_not_uncomputed_generations() -> TestResult {
    let data = fixture_commit_graph_bytes("single_commit.sh")?;
    let file = mapped_file(&data, "commit-graph")?;
    let hash_len = file.object_hash().len_in_bytes();
    let commit_data = chunk_range(&data, *b"CDAT")?.start;
    let generation_offset = commit_data + hash_len + 8;
    let timestamp_bits = u32::from_be_bytes(data[generation_offset..generation_offset + 4].try_into()?) & 3;

    for (offset, replacement, diagnostic, is_corrupted) in [
        (commit_data, vec![0; hash_len], "invalid root tree ID", true),
        (
            commit_data + hash_len,
            0x8000_0000u32.to_be_bytes().to_vec(),
            "first parent is an extra edge index",
            true,
        ),
        (
            commit_data + hash_len,
            file.num_commits().to_be_bytes().to_vec(),
            "parent position 1 that is out of range",
            true,
        ),
        (
            generation_offset,
            ((2u32 << 2) | timestamp_bits).to_be_bytes().to_vec(),
            "generation should be 1 but is 2",
            true,
        ),
        (
            generation_offset,
            timestamp_bits.to_be_bytes().to_vec(),
            "it or a parent has an uncomputed generation",
            false,
        ),
    ] {
        let mut invalid = data.clone();
        invalid[offset..offset + replacement.len()].copy_from_slice(&replacement);
        update_checksum(&mut invalid)?;
        let graph = Graph::new(vec![mapped_file(&invalid, "commit-graph")?])?;
        let err = graph
            .verify_integrity(|_| Ok::<_, Message>(()))
            .expect_err("verification still rejects the altered commit");
        assert_eq!(
            err.is_corrupted(),
            is_corrupted,
            "uncomputed generations are not corruption: {err}"
        );
        assert_eq!(
            err.is_unsupported(),
            !is_corrupted,
            "uncomputed generations require another verification strategy: {err}"
        );
        assert!(
            err.probable_cause().to_string().contains(diagnostic),
            "verification reaches the intended check rather than a checksum mismatch: {err}"
        );
    }
    Ok(())
}

#[test]
fn uncomputed_parent_generations_are_not_corruption() -> TestResult {
    let mut data = fixture_commit_graph_bytes("single_parent.sh")?;
    let hash_len = mapped_file(&data, "commit-graph")?.object_hash().len_in_bytes();
    let commit_data = chunk_range(&data, *b"CDAT")?.start;
    let entry_size = hash_len + 16;
    // Reject unknown parent generations even when treating zero as a number would make the check pass.
    for child_generation in [1u32, 2u32] {
        // Visit the child first so the failure depends on its parent's uncomputed generation.
        for (index, parent, generation) in [(0, 1u32, child_generation), (1, 0x7000_0000u32, 0u32)] {
            let parents_offset = commit_data + index * entry_size + hash_len;
            data[parents_offset..parents_offset + 4].copy_from_slice(&parent.to_be_bytes());
            data[parents_offset + 4..parents_offset + 8].copy_from_slice(&0x7000_0000u32.to_be_bytes());
            let generation_offset = parents_offset + 8;
            let timestamp_bits = u32::from_be_bytes(data[generation_offset..generation_offset + 4].try_into()?) & 3;
            data[generation_offset..generation_offset + 4]
                .copy_from_slice(&((generation << 2) | timestamp_bits).to_be_bytes());
        }
        update_checksum(&mut data)?;
        let graph = Graph::new(vec![mapped_file(&data, "commit-graph")?])?;
        let err = graph
            .verify_integrity(|_| Ok::<_, Message>(()))
            .expect_err("uncomputed parent generations remain unsupported by verification");
        assert!(
            err.is_unsupported() && !err.is_validation(),
            "an uncomputed parent generation requires another verification strategy"
        );
        assert!(
            !err.is_corrupted(),
            "an uncomputed parent generation does not establish corruption"
        );
        assert!(
            err.probable_cause()
                .to_string()
                .contains("it or a parent has an uncomputed generation"),
            "the child's generation cannot be verified without computed parent generations"
        );
    }
    Ok(())
}

#[test]
fn filename_errors_keep_hash_decode_causes() -> TestResult {
    let data = fixture_commit_graph_bytes("single_commit.sh")?;
    let file = mapped_file(&data, "commit-graph")?;
    for (name, invalid_hex) in [
        ("graph-invalid.graph".to_owned(), true),
        (format!("graph-{}.graph", file.object_hash().null()), false),
    ] {
        let file = mapped_file(&data, &name)?;
        let err = file
            .traverse(|_| Ok(()))
            .expect_err("the filename does not match the graph checksum");
        assert!(
            err.is_corrupted(),
            "a split-chain filename must agree with its contents"
        );
        assert_eq!(
            err.is_validation(),
            invalid_hex,
            "only malformed hex contributes a decoding cause"
        );
        assert_eq!(
            err.iter_errors()
                .next()
                .expect("there is a filename diagnostic")
                .to_string(),
            format!("commit-graph filename should be graph-{}.graph", file.checksum()),
            "the existing filename diagnostic is preserved"
        );
        if invalid_hex {
            assert_eq!(
                err.probable_cause().to_string(),
                "A hash sized 7 hexadecimal characters is invalid",
                "the original hash decoder error is retained"
            );
        }
    }
    Ok(())
}

#[test]
fn split_chain_inconsistency_is_corruption() -> TestResult {
    let repo = gix_testtools::scripted_fixture_writable("generation_number_overflow.sh")?;
    let dir = repo.path().join(".git/objects/info/commit-graphs");
    let chain = std::fs::read_to_string(dir.join("commit-graph-chain"))?;
    let mut files = Vec::new();
    for (index, hash) in chain.lines().enumerate() {
        let mut data = std::fs::read(dir.join(format!("graph-{hash}.graph")))?;
        if index == 1 {
            let base = chunk_range(&data, *b"BASE")?;
            data[base.start] ^= 1;
            update_checksum(&mut data)?;
        }
        files.push(mapped_file(&data, "commit-graph")?);
    }
    let graph = Graph::new(files)?;
    let err = graph
        .verify_integrity(|_| Ok::<_, Message>(()))
        .expect_err("the second file refers to a different base graph");
    assert!(err.is_corrupted(), "inconsistent stored BASE references are corruption");
    assert!(
        err.to_string().contains("base graph at index 0"),
        "the altered BASE reference is detected"
    );
    Ok(())
}

#[test]
fn processor_errors_keep_their_classes_and_sources() -> TestResult {
    let data = fixture_commit_graph_bytes("single_commit.sh")?;
    let file = mapped_file(&data, "commit-graph")?;
    let graph = Graph::new(vec![mapped_file(&data, "commit-graph")?])?;
    for err in [
        file.traverse(|_| Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied).raise()))
            .expect_err("the processor failed"),
        graph
            .verify_integrity(|_| Err::<(), _>(std::io::Error::from(std::io::ErrorKind::PermissionDenied)))
            .expect_err("the processor failed"),
    ] {
        assert!(
            !err.is_corrupted(),
            "processor I/O failures do not imply graph corruption"
        );
        assert_eq!(
            err.downcast_any_ref::<std::io::Error>()
                .expect("the processor's native error is retained")
                .kind(),
            std::io::ErrorKind::PermissionDenied,
            "processor context preserves the I/O error kind"
        );
    }
    for err in [
        file.traverse(|_| Err(gix_error::message("processor rejected input").validation_error()))
            .expect_err("the processor rejected its input"),
        graph
            .verify_integrity(|_| Err::<(), _>(gix_error::validation("processor rejected input")))
            .expect_err("the processor rejected its input"),
    ] {
        assert!(err.is_validation(), "the processor's explicit classification survives");
        assert!(
            !err.is_corrupted(),
            "processor validation failures do not imply graph corruption"
        );
        assert_eq!(
            err.probable_cause().to_string(),
            "processor rejected input",
            "the processor remains the cause"
        );
    }
    Ok(())
}

fn fixture_commit_graph_bytes(script: &str) -> TestResult<Vec<u8>> {
    let repo = gix_testtools::scripted_fixture_writable(script)?;
    Ok(std::fs::read(repo.path().join(".git/objects/info/commit-graph"))?)
}

fn mapped_file(data: &[u8], name: &str) -> Result<gix_commitgraph::File> {
    let mut mapping = memmap2::MmapMut::map_anon(data.len()).or_error()?;
    mapping.copy_from_slice(data);
    gix_commitgraph::File::new(mapping.make_read_only().or_error()?, name.into())
}

fn chunk_range(data: &[u8], kind: gix_chunk::Id) -> Result<std::ops::Range<usize>> {
    gix_chunk::file::Index::from_bytes(data, 8, u32::from(data[6]))?.usize_offset_by_id(kind)
}

fn update_checksum(data: &mut [u8]) -> Result {
    let object_hash = gix_hash::Kind::try_from(data[5]).expect("the fixture uses a supported hash kind");
    let trailer_offset = data.len() - object_hash.len_in_bytes();
    let (contents, trailer) = data.split_at_mut(trailer_offset);
    let mut hasher = gix_hash::hasher(object_hash);
    hasher.update(contents);
    trailer.copy_from_slice(hasher.try_finalize()?.as_slice());
    Ok(())
}

use gix_testtools::TestResult;

pub struct V2 {
    pub response: Vec<u8>,
    pub commit_id_hex: String,
    pub object_count: usize,
    pub hash: gix_hash::Kind,
    pub ls_refs_request: Vec<u8>,
    pub fetch_request: Vec<u8>,
}

fn packet(out: &mut Vec<u8>, line: &str) {
    out.extend_from_slice(format!("{:04x}{line}\n", line.len() + 5).as_bytes());
}

fn request(command: &str, hash: &str, arguments: &[&str]) -> Vec<u8> {
    let mut out = Vec::new();
    packet(&mut out, &format!("command={command}"));
    packet(&mut out, "agent=git/transport-test");
    packet(&mut out, &format!("object-format={hash}"));
    out.extend_from_slice(b"0001");
    for argument in arguments {
        packet(&mut out, argument);
    }
    out.extend_from_slice(b"0000");
    out
}

/// Load Git-generated protocol v2 responses and repository metadata for the currently selected fixture hash.
///
/// Set `GIX_TEST_FIXTURE_HASH` to `sha1` or `sha256` to select the hash; when unset,
/// [`gix_testtools::object_hash()`] uses [`gix_hash::Kind::default()`].
///
/// `make_transport_baseline.sh` records the capability advertisement, `ls-refs` response, and fetched pack.
/// Expected client requests are constructed independently here to check the transport's outgoing bytes.
pub fn v2() -> TestResult<V2> {
    let dir =
        gix_testtools::scripted_fixture_read_only("make_transport_baseline.sh")?.join("repo/.git/transport-baseline");
    let commit_id = std::fs::read_to_string(dir.join("commit-id"))?.trim().to_owned();
    let object_count = std::fs::read_to_string(dir.join("object-count"))?.trim().parse()?;
    let hash = gix_testtools::object_hash();
    let hash_name = hash.to_string();
    let ls_refs_request = request(
        "ls-refs",
        &hash_name,
        &[
            "peel",
            "symrefs",
            "ref-prefix HEAD",
            "ref-prefix refs/heads/",
            "ref-prefix refs/tags",
        ],
    );
    let want = format!("want {commit_id}");
    let fetch_request = request("fetch", &hash_name, &["thin-pack", "ofs-delta", &want, "done"]);
    Ok(V2 {
        response: std::fs::read(dir.join("v2.response"))?,
        commit_id_hex: commit_id,
        object_count,
        hash,
        ls_refs_request,
        fetch_request,
    })
}

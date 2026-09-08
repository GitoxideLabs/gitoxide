use std::{
    collections::{HashMap, HashSet},
    hash::BuildHasher,
    io::{BufRead, Cursor},
    path::Path,
    process::Command,
};

use gix_commitgraph::{Graph, Position as GraphPosition};
use gix_testtools::scripted_fixture_read_only;

mod access;

#[test]
fn missing_path_keeps_io_error() -> gix_testtools::Result {
    let dir = gix_testtools::tempfile::tempdir()?;
    let err = gix_commitgraph::at(dir.path().join("missing"))
        .err()
        .expect("a missing path cannot contain a commit-graph");
    assert_eq!(
        err.downcast_any_ref::<std::io::Error>()
            .expect("the filesystem error is preserved")
            .kind(),
        std::io::ErrorKind::NotFound,
        "callers can distinguish a missing optional cache from other failures"
    );
    Ok(())
}

pub fn check_common(cg: &Graph, expected: &HashMap<String, RefInfo, impl BuildHasher>) {
    cg.verify_integrity(|_| Ok::<_, gix_error::Message>(()))
        .expect("graph is valid");
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
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_dir.as_ref())
        .arg("show")
        .arg("--no-patch")
        .arg("--pretty=format:%S %H %T %ct %P")
        .args(refs)
        .arg("--")
        .env_remove("GIT_DIR")
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

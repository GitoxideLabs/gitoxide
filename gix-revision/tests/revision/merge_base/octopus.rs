use gix_error::TestResult;

use crate::{hex_to_id, odb_at};

#[test]
fn three_sequential_commits() -> TestResult {
    let odb = octopus_odb_at("three-sequential-commits")?;
    let mut graph = gix_revision::Graph::new(&odb, None);
    let first_commit = hex_to_id("e5d0542bd38431f105a8de8e982b3579647feb9f");
    let mut heads = vec![
        hex_to_id("4fbed377d3eab982d4a465cafaf34b64207da847"),
        hex_to_id("8bc2f99c9aacf07568a2bbfe1269f6e543f22d6b"),
        first_commit,
    ];
    let mut heap = permutohedron::Heap::new(&mut heads);
    while let Some(heads) = heap.next_permutation() {
        let actual = gix_revision::merge_base::octopus(*heads.first().expect("three heads"), &heads[1..], &mut graph)?
            .expect("a merge base");
        assert_eq!(actual, first_commit);
    }
    Ok(())
}

#[test]
fn three_parallel_commits() -> TestResult {
    let odb = octopus_odb_at("three-parallel-commits")?;
    let mut graph = gix_revision::Graph::new(&odb, None);
    let base = hex_to_id("3ca3e3dd12585fabbef311d524a5e54678090528");
    let mut heads = vec![
        hex_to_id("4ce66b336dff547fdeb6cd86e04c617c8d998ff5"),
        hex_to_id("6291f6d7da04208dc4ccbbdf9fda98ac9ae67bc0"),
        hex_to_id("c507d5413da00c32e5de1ea433030e8e4716bc60"),
    ];
    let mut heap = permutohedron::Heap::new(&mut heads);
    while let Some(heads) = heap.next_permutation() {
        let actual = gix_revision::merge_base::octopus(*heads.first().expect("three heads"), &heads[1..], &mut graph)?
            .expect("a merge base");
        assert_eq!(actual, base);
    }
    Ok(())
}

#[test]
fn three_forked_commits() -> TestResult {
    let odb = octopus_odb_at("three-forked-commits")?;
    let mut graph = gix_revision::Graph::new(&odb, None);
    let base = hex_to_id("3ca3e3dd12585fabbef311d524a5e54678090528");
    let mut heads = vec![
        hex_to_id("413d38a3fe7453c68cb7314739d7775f68ab89f5"),
        hex_to_id("d4d01a9b6f6fcb23d57cd560229cd9680ec9bd6e"),
        hex_to_id("c507d5413da00c32e5de1ea433030e8e4716bc60"),
    ];
    let mut heap = permutohedron::Heap::new(&mut heads);
    while let Some(heads) = heap.next_permutation() {
        let actual = gix_revision::merge_base::octopus(*heads.first().expect("three heads"), &heads[1..], &mut graph)?
            .expect("a merge base");
        assert_eq!(actual, base);
    }
    Ok(())
}

fn octopus_odb_at(name: &str) -> TestResult<gix_odb::Handle> {
    let root = gix_testtools::scripted_fixture_read_only("merge_base_octopus_repos.sh")?;
    Ok(odb_at(root.join(name).join(".git/objects"))?)
}

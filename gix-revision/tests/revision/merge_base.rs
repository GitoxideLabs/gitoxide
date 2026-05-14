use gix_revision::merge_base;

#[test]
fn validate() -> crate::Result {
    let root = gix_testtools::scripted_fixture_read_only("make_merge_base_repos.sh")?;
    let mut count = 0;
    let odb = gix_odb::at(root.join(".git/objects"))?;
    for baseline_path in baseline::expectation_paths(&root)? {
        count += 1;
        for use_commitgraph in [false, true] {
            let cache = use_commitgraph
                .then(|| gix_commitgraph::Graph::from_info_dir(&odb.store_ref().path().join("info")).unwrap());
            for expected in baseline::parse_expectations(&baseline_path)? {
                let mut graph = gix_revision::Graph::new(&odb, cache.as_ref());
                let actual = merge_base(expected.first, &expected.others, &mut graph)?;
                assert_eq!(
                    actual,
                    expected.bases,
                    "sample {file:?}:{input}",
                    file = baseline_path.with_extension("").file_name(),
                    input = expected.plain_input
                );
            }
            let mut graph = gix_revision::Graph::new(&odb, cache.as_ref());
            for expected in baseline::parse_expectations(&baseline_path)? {
                let actual = merge_base(expected.first, &expected.others, &mut graph)?;
                assert_eq!(
                    actual,
                    expected.bases,
                    "sample (reused graph) {file:?}:{input}",
                    file = baseline_path.with_extension("").file_name(),
                    input = expected.plain_input
                );
            }
        }
    }
    assert_ne!(count, 0, "there must be at least one baseline");
    Ok(())
}

mod octopus {
    use crate::hex_to_id;

    #[test]
    fn three_sequential_commits() -> crate::Result {
        let odb = odb_at("three-sequential-commits")?;
        let mut graph = gix_revision::Graph::new(&odb, None);
        let first_commit = hex_to_id("e5d0542bd38431f105a8de8e982b3579647feb9f");
        let mut heads = vec![
            hex_to_id("4fbed377d3eab982d4a465cafaf34b64207da847"),
            hex_to_id("8bc2f99c9aacf07568a2bbfe1269f6e543f22d6b"),
            first_commit,
        ];
        let mut heap = permutohedron::Heap::new(&mut heads);
        while let Some(heads) = heap.next_permutation() {
            let actual = gix_revision::merge_base::octopus(*heads.first().unwrap(), &heads[1..], &mut graph)?
                .expect("a merge base");
            assert_eq!(actual, first_commit);
        }
        Ok(())
    }

    #[test]
    fn three_parallel_commits() -> crate::Result {
        let odb = odb_at("three-parallel-commits")?;
        let mut graph = gix_revision::Graph::new(&odb, None);
        let base = hex_to_id("3ca3e3dd12585fabbef311d524a5e54678090528");
        let mut heads = vec![
            hex_to_id("4ce66b336dff547fdeb6cd86e04c617c8d998ff5"),
            hex_to_id("6291f6d7da04208dc4ccbbdf9fda98ac9ae67bc0"),
            hex_to_id("c507d5413da00c32e5de1ea433030e8e4716bc60"),
        ];
        let mut heap = permutohedron::Heap::new(&mut heads);
        while let Some(heads) = heap.next_permutation() {
            let actual = gix_revision::merge_base::octopus(*heads.first().unwrap(), &heads[1..], &mut graph)?
                .expect("a merge base");
            assert_eq!(actual, base);
        }
        Ok(())
    }

    #[test]
    fn three_forked_commits() -> crate::Result {
        let odb = odb_at("three-forked-commits")?;
        let mut graph = gix_revision::Graph::new(&odb, None);
        let base = hex_to_id("3ca3e3dd12585fabbef311d524a5e54678090528");
        let mut heads = vec![
            hex_to_id("413d38a3fe7453c68cb7314739d7775f68ab89f5"),
            hex_to_id("d4d01a9b6f6fcb23d57cd560229cd9680ec9bd6e"),
            hex_to_id("c507d5413da00c32e5de1ea433030e8e4716bc60"),
        ];
        let mut heap = permutohedron::Heap::new(&mut heads);
        while let Some(heads) = heap.next_permutation() {
            let actual = gix_revision::merge_base::octopus(*heads.first().unwrap(), &heads[1..], &mut graph)?
                .expect("a merge base");
            assert_eq!(actual, base);
        }
        Ok(())
    }

    fn odb_at(name: &str) -> crate::Result<gix_odb::Handle> {
        let root = gix_testtools::scripted_fixture_read_only("merge_base_octopus_repos.sh")?;
        Ok(gix_odb::at(root.join(name).join(".git/objects"))?)
    }
}

mod baseline {
    use std::{
        ffi::OsStr,
        path::{Path, PathBuf},
    };

    use bstr::ByteSlice;
    use gix_hash::ObjectId;
    use nonempty::NonEmpty;

    /// The expectation as produced by Git itself
    #[derive(Debug)]
    pub struct Expectation {
        pub plain_input: String,
        pub first: ObjectId,
        pub others: Vec<ObjectId>,
        pub bases: Option<NonEmpty<ObjectId>>,
    }

    pub fn parse_expectations(baseline: &Path) -> std::io::Result<Vec<Expectation>> {
        let lines = std::fs::read(baseline)?;
        let mut lines = lines.lines();
        let mut out = Vec::new();
        while let Some(plain_input) = lines.next() {
            let plain_input = plain_input.to_str_lossy().into_owned();
            let mut input = lines
                .next()
                .expect("second line is resolved input objects")
                .split(|b| *b == b' ');
            let first = ObjectId::from_hex(input.next().expect("at least one object")).unwrap();
            let others = input.map(|hex_id| ObjectId::from_hex(hex_id).unwrap()).collect();
            let bases: Vec<_> = lines
                .by_ref()
                .take_while(|l| !l.is_empty())
                .map(|hex_id| ObjectId::from_hex(hex_id).unwrap())
                .collect();
            out.push(Expectation {
                plain_input,
                first,
                others,
                bases: NonEmpty::from_vec(bases),
            });
        }
        Ok(out)
    }

    pub fn expectation_paths(root: &Path) -> std::io::Result<Vec<PathBuf>> {
        let mut out: Vec<_> = std::fs::read_dir(root)?
            .map(Result::unwrap)
            .filter_map(|e| (e.path().extension() == Some(OsStr::new("baseline"))).then(|| e.path()))
            .collect();
        out.sort();
        Ok(out)
    }
}

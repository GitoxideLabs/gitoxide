use gix_testtools::{TestResult, size_ok};

mod window_size {
    use gix_negotiate::window_size;

    #[test]
    fn initial_value_without_previous_window_size() {
        assert_eq!(window_size(false, None), 16);
        assert_eq!(window_size(true, None), 16);
    }

    #[test]
    fn transport_is_stateless() {
        let mut ws = window_size(true, None);
        for expected in [32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 18022, 19824] {
            ws = window_size(true, Some(ws));
            assert_eq!(ws, expected);
        }
    }

    #[test]
    fn transport_is_not_stateless() {
        let mut ws = window_size(false, None);
        for expected in [32, 64, 96] {
            ws = window_size(false, Some(ws));
            assert_eq!(ws, expected);
        }

        let mut ws = 4;
        for expected in [8, 16, 32, 64, 96] {
            ws = window_size(false, Some(ws));
            assert_eq!(ws, expected);
        }
    }
}

mod baseline;

#[test]
fn missing_parents_are_skipped_like_shallow_boundaries() -> TestResult {
    use gix_negotiate::Algorithm;
    use gix_object::FindExt;
    struct MissingParent<'a> {
        odb: &'a gix_odb::Handle,
        missing_commit_id: gix_hash::ObjectId,
    }
    impl gix_object::Find for MissingParent<'_> {
        fn try_find<'a>(
            &self,
            commit_id: &gix_hash::oid,
            buf: &'a mut Vec<u8>,
        ) -> gix_error::Result<Option<gix_object::Data<'a>>> {
            if commit_id == self.missing_commit_id {
                Ok(None)
            } else {
                gix_object::Find::try_find(self.odb, commit_id, buf)
            }
        }
    }

    let git_dir = gix_testtools::scripted_fixture_read_only("make_repos.sh")?.join("no_parents/client/.git");
    let object_hash = gix_testtools::object_hash();
    let odb = gix_odb::at(git_dir.join("objects"), object_hash)?;
    let refs = gix_ref::file::Store::at_opts(git_dir, object_hash, Default::default());
    let tip_commit_id = refs.find("c7")?.target.into_id();
    let missing_commit_id = odb
        .find_commit(&tip_commit_id, &mut Vec::new())?
        .parents()
        .next()
        .expect("c7 has a parent");
    for algorithm in [Algorithm::Consecutive, Algorithm::Skipping, Algorithm::Noop] {
        let mut graph = gix_revwalk::Graph::new(
            MissingParent {
                odb: &odb,
                missing_commit_id,
            },
            None,
        );
        let mut negotiator = algorithm.into_negotiator();
        negotiator.add_tip(tip_commit_id, &mut graph)?;
        let haves = std::iter::from_fn(|| negotiator.next_have(&mut graph)).collect::<Result<Vec<_>, _>>()?;
        let expected: &[_] = match algorithm {
            Algorithm::Consecutive | Algorithm::Skipping => &[tip_commit_id],
            Algorithm::Noop => &[],
        };
        assert_eq!(
            haves, expected,
            "{algorithm}: a missing parent is not traversed; only walking negotiators send the tip"
        );
    }
    Ok(())
}

#[test]
fn size_of_entry() {
    let actual = std::mem::size_of::<gix_revwalk::graph::Commit<gix_negotiate::Metadata>>();
    let sha1 = 56;
    let sha256_extra = 16;
    let expected = sha1 + sha256_extra;
    assert!(
        size_ok(actual, expected),
        "we may keep a lot of these, so let's not let them grow unnoticed: {actual} <~ {expected}"
    );
}

use gix_diff::Rewrites;
use gix_odb::Write;

#[test]
fn run_baseline() -> crate::Result {
    let root = gix_testtools::scripted_fixture_read_only("tree-baseline.sh")?;
    let cases = std::fs::read_to_string(root.join("baseline.cases"))?;
    for baseline::Expectation {
        root,
        odb,
        our_commit_id,
        their_commit_id,
        merge_info,
        case_name,
    } in baseline::Expectations::new(&root, &cases)
    {
        let mut graph = gix_revwalk::Graph::new(&odb, None);
        let mut blob_merge = baseline::new_platform(&root, None);
        let mut diff_resource_cache = baseline::new_diff_resource_cache(&root);
        let options = gix_merge::commit::Options {
            allow_missing_merge_base: false,
            tree_merge: gix_merge::tree::Options {
                rewrites: Some(Rewrites {
                    copies: None,
                    percentage: Some(0.5),
                    limit: 0,
                }),
            },
            blob_merge: Default::default(),
        };
        let mut actual = gix_merge::commit(
            our_commit_id,
            their_commit_id,
            gix_merge::blob::builtin_driver::text::Labels {
                ancestor: None,
                current: Some("ours".into()),
                other: Some("theirs".into()),
            },
            &mut graph,
            &mut diff_resource_cache,
            &mut blob_merge,
            &odb,
            options,
        )?;

        match merge_info {
            Ok(expected_tree_id) => {
                let actual_id = actual.tree.write(|tree| odb.write(tree))?;
                assert_eq!(actual_id, expected_tree_id, "{case_name}: merged tree mismatch");
            }
            Err(_conflicts) => {
                todo!("compare conflicts")
            }
        }
    }

    Ok(())
}

mod baseline {
    use gix_worktree::stack::state::attributes;
    use std::path::{Path, PathBuf};

    pub struct Conflict;

    pub struct Expectation {
        pub root: PathBuf,
        pub odb: gix_odb::memory::Proxy<gix_odb::Handle>,
        pub our_commit_id: gix_hash::ObjectId,
        pub their_commit_id: gix_hash::ObjectId,
        pub merge_info: Result<gix_hash::ObjectId, Conflict>,
        pub case_name: String,
    }

    pub struct Expectations<'a> {
        root: &'a Path,
        lines: std::str::Lines<'a>,
    }

    impl<'a> Expectations<'a> {
        pub fn new(root: &'a Path, cases: &'a str) -> Self {
            Expectations {
                root,
                lines: cases.lines(),
            }
        }
    }

    impl Iterator for Expectations<'_> {
        type Item = Expectation;

        fn next(&mut self) -> Option<Self::Item> {
            let line = self.lines.next()?;
            let mut tokens = line.split(' ');
            let (Some(subdir), Some(our_commit_id), Some(their_commit_id), Some(merge_info_filename)) =
                (tokens.next(), tokens.next(), tokens.next(), tokens.next())
            else {
                unreachable!("invalid line: {line:?}")
            };
            assert_eq!(tokens.next(), None, "unexpected trailing tokens in line {line:?}");

            let subdir_path = self.root.join(subdir);
            let odb = gix_odb::at(subdir_path.join(".git/objects")).expect("object dir exists");
            let objects = gix_odb::memory::Proxy::new(odb, gix_hash::Kind::Sha1);
            let our_commit_id = gix_hash::ObjectId::from_hex(our_commit_id.as_bytes()).unwrap();
            let their_commit_id = gix_hash::ObjectId::from_hex(their_commit_id.as_bytes()).unwrap();
            let merge_info = parse_merge_info(std::fs::read_to_string(subdir_path.join(merge_info_filename)).unwrap());
            Some(Expectation {
                root: subdir_path,
                odb: objects,
                our_commit_id,
                their_commit_id,
                merge_info,
                case_name: format!(
                    "{subdir}-{}",
                    merge_info_filename
                        .split('.')
                        .next()
                        .expect("extension after single dot")
                ),
            })
        }
    }

    fn parse_merge_info(content: String) -> Result<gix_hash::ObjectId, Conflict> {
        let mut lines = content.split('\0').filter(|t| !t.is_empty());
        let tree_id = gix_hash::ObjectId::from_hex(lines.next().unwrap().as_bytes()).unwrap();
        assert_eq!(lines.next(), None, "TODO: implement multi-line answer");
        Ok(tree_id)
    }

    pub fn new_platform(
        root: &Path,
        drivers: impl IntoIterator<Item = gix_merge::blob::Driver>,
    ) -> gix_merge::blob::Platform {
        let attributes = gix_worktree::Stack::new(
            root,
            gix_worktree::stack::State::AttributesStack(gix_worktree::stack::state::Attributes::new(
                Default::default(),
                None,
                attributes::Source::WorktreeThenIdMapping,
                Default::default(),
            )),
            gix_worktree::glob::pattern::Case::Sensitive,
            Vec::new(),
            Vec::new(),
        );
        let filter =
            gix_merge::blob::Pipeline::new(Default::default(), gix_filter::Pipeline::default(), Default::default());
        gix_merge::blob::Platform::new(
            filter,
            gix_merge::blob::pipeline::Mode::ToGit,
            attributes,
            drivers.into_iter().collect(),
            Default::default(),
        )
    }

    pub fn new_diff_resource_cache(root: &Path) -> gix_diff::blob::Platform {
        gix_diff::blob::Platform::new(
            Default::default(),
            gix_diff::blob::Pipeline::new(Default::default(), Default::default(), Vec::new(), Default::default()),
            Default::default(),
            gix_worktree::Stack::new(
                root,
                gix_worktree::stack::State::AttributesStack(gix_worktree::stack::state::Attributes::default()),
                Default::default(),
                Vec::new(),
                Vec::new(),
            ),
        )
    }
}

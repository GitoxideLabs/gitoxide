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
        our_side_name,
        their_commit_id,
        their_side_name,
        merge_info,
        case_name,
    } in baseline::Expectations::new(&root, &cases)
    {
        let mut graph = gix_revwalk::Graph::new(&odb, None);
        let mut blob_merge = baseline::new_platform(&root, None);
        let mut diff_resource_cache = baseline::new_diff_resource_cache(&root);
        let options = gix_merge::commit::Options {
            allow_missing_merge_base: false,
            use_first_merge_base: false,
            tree_merge: gix_merge::tree::Options {
                rewrites: Some(Rewrites {
                    copies: None,
                    percentage: Some(0.5),
                    limit: 0,
                }),
                blob_merge: Default::default(),
                blob_merge_command_ctx: Default::default(),
                fail_on_conflict: false,
            },
        };
        let mut actual = gix_merge::commit(
            our_commit_id,
            their_commit_id,
            gix_merge::blob::builtin_driver::text::Labels {
                ancestor: None,
                current: Some(our_side_name.as_str().into()),
                other: Some(their_side_name.as_str().into()),
            },
            &mut graph,
            &mut diff_resource_cache,
            &mut blob_merge,
            &odb,
            |content| odb.write_buf(gix_object::Kind::Blob, content),
            options,
        )?;

        let actual_id = actual.tree.write(|tree| odb.write(tree))?;
        assert_eq!(
            actual_id, merge_info.merged_tree,
            "{case_name}: merged tree mismatch {:?} {:?}",
            actual.conflicts, merge_info.information
        );
    }

    Ok(())
}

// TODO: make sure everything is read eventually, even if only to improve debug messages in case of failure.
#[allow(dead_code)]
mod baseline {
    use gix_object::tree::EntryMode;
    use gix_worktree::stack::state::attributes;
    use std::path::{Path, PathBuf};

    /// An entry in the conflict
    #[derive(Debug)]
    pub struct Entry {
        /// The relative path in the repository
        pub location: String,
        /// The content id.
        pub id: gix_hash::ObjectId,
        /// The kind of entry.
        pub mode: EntryMode,
    }

    /// Keep track of all the sides of a conflict. Some might not be set to indicate removal, including the ancestor.
    #[derive(Default, Debug)]
    pub struct Conflict {
        pub ancestor: Option<Entry>,
        pub ours: Option<Entry>,
        pub theirs: Option<Entry>,
    }

    #[derive(Debug)]
    pub enum ConflictKind {
        /// The conflict was resolved by automatically merging the content.
        AutoMerging,
        /// The content could not be resolved so it's conflicting.
        ConflictContents,
        /// Directory in theirs in the way of our file
        ConflictDirectoryBlocksFile,
        /// Modified in ours but deleted in theirs
        ConflictModifyDelete,
        /// TODO
        FOo,
    }

    /// More loosely structured information about the `Conflict`.
    #[derive(Debug)]
    pub struct ConflictInfo {
        /// All the paths involved in the informational message
        pub paths: Vec<String>,
        /// The type of the conflict, further described in `message`.
        pub kind: ConflictKind,
        /// An arbitrary message formed from paths and kind
        pub message: String,
    }

    impl Conflict {
        fn any_location(&self) -> Option<&str> {
            self.ancestor
                .as_ref()
                .or(self.ours.as_ref())
                .or(self.theirs.as_ref())
                .map(|a| a.location.as_str())
        }
        fn storage_for(&mut self, side: Side, location: &str) -> Option<&mut Option<Entry>> {
            let current_location = self.any_location();
            let location_is_same = current_location.is_none() || current_location == Some(location);
            let side = match side {
                Side::Ancestor => &mut self.ancestor,
                Side::Ours => &mut self.ours,
                Side::Theirs => &mut self.theirs,
            };
            (!side.is_some() && location_is_same).then_some(side)
        }
    }

    pub struct MergeInfo {
        /// The hash of the merged tree - it may contain intermediate files if the merge didn't succeed entirely.
        pub merged_tree: gix_hash::ObjectId,
        /// If there were conflicts, this is the conflicting paths.
        pub conflicts: Option<Vec<Conflict>>,
        /// Structured details which to some extent can be compared to our own conflict information.
        pub information: Vec<ConflictInfo>,
    }

    pub struct Expectation {
        pub root: PathBuf,
        pub odb: gix_odb::memory::Proxy<gix_odb::Handle>,
        pub our_commit_id: gix_hash::ObjectId,
        pub our_side_name: String,
        pub their_commit_id: gix_hash::ObjectId,
        pub their_side_name: String,
        pub merge_info: MergeInfo,
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
            let (
                Some(subdir),
                Some(our_commit_id),
                Some(our_side_name),
                Some(their_commit_id),
                Some(their_side_name),
                Some(merge_info_filename),
            ) = (
                tokens.next(),
                tokens.next(),
                tokens.next(),
                tokens.next(),
                tokens.next(),
                tokens.next(),
            )
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
                our_side_name: our_side_name.to_owned(),
                their_commit_id,
                their_side_name: their_side_name.to_owned(),
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

    fn parse_merge_info(content: String) -> MergeInfo {
        let mut lines = content.split('\0').filter(|t| !t.is_empty()).peekable();
        let tree_id = gix_hash::ObjectId::from_hex(lines.next().unwrap().as_bytes()).unwrap();
        let mut out = MergeInfo {
            merged_tree: tree_id,
            conflicts: None,
            information: Vec::new(),
        };

        let mut conflicts = Vec::new();
        let mut conflict = Conflict::default();
        while let Some(line) = lines.peek() {
            let (entry, side) = match parse_conflict_file_info(line) {
                Some(t) => t,
                None => break,
            };
            lines.next();
            let field = match conflict.storage_for(side, &entry.location) {
                None => {
                    conflicts.push(conflict);
                    conflict = Conflict::default();
                    conflict
                        .storage_for(side, &entry.location)
                        .expect("always available for new side")
                }
                Some(field) => field,
            };
            *field = Some(entry);
        }

        while lines.peek().is_some() {
            out.information
                .push(parse_info(&mut lines).expect("if there are lines, it should be valid info"));
        }
        assert_eq!(lines.next(), None, "TODO: conflict messages");
        out.conflicts = (!conflicts.is_empty()).then_some(conflicts);
        out
    }

    #[derive(Copy, Clone)]
    enum Side {
        Ancestor,
        Ours,
        Theirs,
    }

    fn parse_conflict_file_info(line: &str) -> Option<(Entry, Side)> {
        let (info, mut path) = line.split_at(line.find('\t')?);
        path = &path[1..];
        let mut tokens = info.split(' ');
        let (oct_mode, hex_id, stage) = (
            tokens.next().expect("mode"),
            tokens.next().expect("id"),
            tokens.next().expect("stage"),
        );
        assert_eq!(
            tokens.next(),
            None,
            "info line not understood, expected three fields only"
        );
        Some((
            Entry {
                location: path.to_owned(),
                id: gix_hash::ObjectId::from_hex(hex_id.as_bytes()).unwrap(),
                mode: EntryMode(gix_utils::btoi::to_signed_with_radix::<usize>(oct_mode.as_bytes(), 8).unwrap() as u16),
            },
            match stage {
                "1" => Side::Ancestor,
                "2" => Side::Ours,
                "3" => Side::Theirs,
                invalid => panic!("{invalid} is an unexpected side"),
            },
        ))
    }

    fn parse_info<'a>(mut lines: impl Iterator<Item = &'a str>) -> Option<ConflictInfo> {
        let num_paths: usize = lines.next()?.parse().ok()?;
        let paths: Vec<_> = lines.by_ref().take(num_paths).map(ToOwned::to_owned).collect();
        let kind = match lines.next()? {
            "Auto-merging" => ConflictKind::AutoMerging,
            "CONFLICT (contents)" => ConflictKind::ConflictContents,
            "CONFLICT (file/directory)" => ConflictKind::ConflictDirectoryBlocksFile,
            "CONFLICT (modify/delete)" => ConflictKind::ConflictModifyDelete,
            "CONFLICT (directory rename suggested)" => ConflictKind::FOo,
            conflict_type => panic!("Unkonwn conflict type: {conflict_type}"),
        };
        let message = lines.next()?.to_owned();
        dbg!(&kind, &message);
        Some(ConflictInfo { paths, kind, message })
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

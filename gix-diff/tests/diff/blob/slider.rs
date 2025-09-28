use std::path::PathBuf;

use gix_diff::blob::{
    intern::TokenSource,
    unified_diff::{ConsumeHunk, ContextSize, DiffLineKind, HunkHeader},
    Algorithm, UnifiedDiff,
};
use gix_object::FindExt;
use gix_ref::{bstr::BString, file::ReferenceExt};

struct Baseline {
    hunks: (),
}

mod baseline {}

#[derive(Debug)]
struct DiffHunk {
    header: HunkHeader,
    lines: Vec<(DiffLineKind, BString)>,
}

struct DiffHunkRecorder {
    inner: Vec<DiffHunk>,
}

impl DiffHunkRecorder {
    fn new() -> Self {
        Self { inner: Vec::new() }
    }
}

impl ConsumeHunk for DiffHunkRecorder {
    type Out = Vec<DiffHunk>;

    fn consume_hunk(
        &mut self,
        header: HunkHeader,
        lines: &[(gix_diff::blob::unified_diff::DiffLineKind, &[u8])],
    ) -> std::io::Result<()> {
        let lines: Vec<_> = lines
            .iter()
            .map(|(kind, line)| (*kind, BString::new(line.to_vec())))
            .collect();

        let diff_hunk = DiffHunk { header, lines };

        self.inner.push(diff_hunk);

        Ok(())
    }

    fn finish(self) -> Self::Out {
        self.inner
    }
}

#[test]
fn sliders() -> gix_testtools::Result {
    let worktree_path = fixture_path()?;
    let git_dir = worktree_path.join(".git");
    let odb = gix_odb::at(git_dir.join("objects"))?;
    let refs = gix_ref::file::Store::at(git_dir.clone(), gix_ref::store::init::Options::default());
    // In `../tree.rs`, there is `head_of` which would obviate the need for using `gix_ref`.
    let mut head = refs.find("HEAD")?;
    let head_id = head.peel_to_id(&refs, &odb)?;

    let commits = gix_traverse::commit::Simple::new(Some(head_id), &odb)
        .map(Result::unwrap)
        .map(|commit| commit.id)
        .collect::<Vec<_>>();

    let index = gix_index::File::at(git_dir.join("index"), gix_hash::Kind::Sha1, false, Default::default())?;
    let stack = gix_worktree::Stack::from_state_and_ignore_case(
        worktree_path.clone(),
        false,
        gix_worktree::stack::State::AttributesAndIgnoreStack {
            attributes: Default::default(),
            ignore: Default::default(),
        },
        &index,
        index.path_backing(),
    );
    let capabilities = gix_fs::Capabilities::probe(&git_dir);
    let mut resource_cache = gix_diff::blob::Platform::new(
        Default::default(),
        gix_diff::blob::Pipeline::new(
            gix_diff::blob::pipeline::WorktreeRoots {
                old_root: None,
                new_root: None,
            },
            gix_filter::Pipeline::new(Default::default(), Default::default()),
            vec![],
            gix_diff::blob::pipeline::Options {
                large_file_threshold_bytes: 0,
                fs: capabilities,
            },
        ),
        gix_diff::blob::pipeline::Mode::ToGit,
        stack,
    );

    assert!(commits.len() > 0);
    assert!(commits.len().is_multiple_of(2));

    let mut buffer = Vec::new();
    let mut buffer2 = Vec::new();

    let mut iter = commits.chunks(2);

    while let Some(&[new, old]) = iter.next() {
        // TODO: We can extract the duplicate code.
        let old_commit = odb.find_commit(&old, &mut buffer)?.to_owned();
        let new_commit = odb.find_commit(&new, &mut buffer)?.to_owned();
        let file_path = BString::from(old_commit.message.trim_ascii_end());

        eprintln!("diffing {old} and {new}, file path {file_path}");

        let old_tree = old_commit.tree;
        let old_blob_id = odb
            .find_tree(&old_tree, &mut buffer2)?
            .entries
            .iter()
            .find(|entry| file_path.eq(entry.filename))
            .unwrap()
            .oid
            .to_owned();

        let new_tree = new_commit.tree;
        let new_blob_id = odb
            .find_tree(&new_tree, &mut buffer2)?
            .entries
            .iter()
            .find(|entry| entry.filename == file_path)
            .unwrap()
            .oid
            .to_owned();

        resource_cache.set_resource(
            old_blob_id,
            gix_object::tree::EntryKind::Blob,
            file_path.as_ref(),
            gix_diff::blob::ResourceKind::OldOrSource,
            &odb,
        )?;
        resource_cache.set_resource(
            new_blob_id,
            gix_object::tree::EntryKind::Blob,
            file_path.as_ref(),
            gix_diff::blob::ResourceKind::NewOrDestination,
            &odb,
        )?;

        let outcome = resource_cache.prepare_diff()?;

        //let old_string: gix_ref::bstr::BString = outcome.old.data.as_slice().unwrap().into();
        //let new_string: gix_ref::bstr::BString = outcome.new.data.as_slice().unwrap().into();
        //eprintln!("{old_string}");
        //eprintln!("{new_string}");

        let interner = gix_diff::blob::intern::InternedInput::new(
            tokens_for_diffing(outcome.old.data.as_slice().unwrap_or_default()),
            tokens_for_diffing(outcome.new.data.as_slice().unwrap_or_default()),
        );

        let actual = gix_diff::blob::diff(
            Algorithm::Myers,
            &interner,
            UnifiedDiff::new(&interner, DiffHunkRecorder::new(), ContextSize::symmetrical(3)),
        )?;

        eprintln!("{actual:#?}");
    }

    Ok(())
}

fn tokens_for_diffing(data: &[u8]) -> impl TokenSource<Token = &[u8]> {
    gix_diff::blob::sources::byte_lines(data)
}

fn fixture_path() -> gix_testtools::Result<PathBuf> {
    gix_testtools::scripted_fixture_read_only_standalone("make_diff_for_sliders_repo.sh")
}

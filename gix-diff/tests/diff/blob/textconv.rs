use std::path::Path;

use gix_diff::blob::{self, ResourceKind, pipeline::WorktreeRoots};
use gix_worktree::stack::state::attributes;
use imara_diff::{Algorithm::Histogram, Diff};

use crate::blob::pipeline::convert_to_diffable::default_options;

#[test]
fn binary_diff_with_textconv() -> gix_testtools::Result {
    let workdir = crate::scripted_fixture_read_only("make_blob_textconv_repo.sh")?;

    // There's no quick and easy way to get the blob ids using just plumbing crates. In other tests
    // in `gix-diff`, ids are hard-coded and passed to `hex_to_id`.
    let new_file_id = read_id(&workdir.join("new-file.id"))?;
    let changed_file_id = read_id(&workdir.join("changed-file.id"))?;

    let command = "tr '\\000' '\\n' <";
    let attributes = gix_worktree::Stack::new(
        &workdir,
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
    let driver = gix_diff::blob::Driver {
        name: "bin".into(),
        binary_to_text_command: Some(command.into()),
        ..Default::default()
    };
    let pipeline = gix_diff::blob::Pipeline::new(
        WorktreeRoots::default(),
        gix_filter::Pipeline::default(),
        vec![driver],
        default_options(),
    );

    let odb = gix_odb::at(workdir.join(".git/objects"), gix_testtools::object_hash())?;

    let mut resource_cache = gix_diff::blob::Platform::new(
        Default::default(),
        pipeline,
        gix_diff::blob::pipeline::Mode::ToWorktreeAndBinaryToText,
        attributes,
    );

    resource_cache.set_resource(
        new_file_id,
        gix_object::tree::EntryKind::Blob,
        "sample.bin".into(),
        ResourceKind::OldOrSource,
        &odb,
    )?;
    resource_cache.set_resource(
        changed_file_id,
        gix_object::tree::EntryKind::Blob,
        "sample.bin".into(),
        ResourceKind::NewOrDestination,
        &odb,
    )?;

    let out = resource_cache.prepare_diff()?;
    let input = out.interned_input();
    let diff = Diff::compute(Histogram, &input);

    let actual = blob::UnifiedDiff::new(
        &diff,
        &input,
        blob::unified_diff::ConsumeBinaryHunk::new(String::new(), "\n"),
        blob::unified_diff::ContextSize::symmetrical(3),
    )
    .consume()?;

    let baseline = std::fs::read(workdir.join("baseline.diff"))?;
    let expected = crate::blob::skip_header_and_fold_to_unidiff(&baseline);

    assert_eq!(actual, expected);

    Ok(())
}

fn read_id(path: &Path) -> crate::Result<gix_hash::ObjectId> {
    let hex = std::fs::read_to_string(path)?;

    Ok(gix_hash::ObjectId::from_hex(hex.trim().as_bytes())?)
}

use std::path::Path;

use gix_diff::blob::{self, ResourceKind, pipeline::WorktreeRoots};

use imara_diff::{Algorithm::Histogram, Diff};

use crate::blob::pipeline::convert_to_diffable::default_options;

#[test]
fn binary_diff_with_textconv() -> gix_testtools::Result {
    let workdir = gix_testtools::scripted_fixture_read_only_needs_archive("make_blob_textconv_repo.sh")?;

    let new_file_id = read_id(&workdir.join("new-file.id"))?;
    let changed_file_id = read_id(&workdir.join("changed-file.id"))?;

    let command = "tr '\\000' '\\n' <";
    let attributes = crate::blob::new_attributes_stack(&workdir);
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

    assert_eq!(
        actual, expected,
        "the unified diffs of textconv processed diffs matches perfectly, as sliders don't play a role here"
    );
    Ok(())
}

fn read_id(path: &Path) -> crate::Result<gix_hash::ObjectId> {
    let hex = std::fs::read_to_string(path)?;

    Ok(gix_hash::ObjectId::from_hex(hex.trim().as_bytes())?)
}

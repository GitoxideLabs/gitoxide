use bstr::ByteSlice;
use gix_attributes::glob::pattern::Case;
use gix_filter::eol;

mod convert_to_git;
mod convert_to_worktree;

#[test]
fn default() -> crate::Result {
    let mut filters = gix_filter::Pipeline::default();
    let out = filters.convert_to_worktree(
        b"hi",
        "file".into(),
        &mut |_, _| {},
        gix_filter::driver::apply::Delay::Allow,
    )?;
    assert_eq!(
        out.as_bytes().expect("unchanged").as_bstr(),
        "hi",
        "default-pipelines can be used like normal, they have not effect"
    );
    Ok(())
}

fn attribute_cache(name: &str) -> gix_testtools::Result<gix_worktree::Stack> {
    let dir = gix_testtools::scripted_fixture_read_only("pipeline_repos.sh")?.join(name);
    Ok(gix_worktree::Stack::new(
        dir,
        gix_worktree::stack::State::for_add(
            gix_worktree::stack::state::Attributes::new(
                Default::default(),
                None,
                gix_worktree::stack::state::attributes::Source::WorktreeThenIdMapping,
                Default::default(),
            ),
            gix_worktree::stack::state::Ignore::new(
                Default::default(),
                Default::default(),
                None,
                gix_worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped,
                Default::default(),
            ),
        ),
        Case::Sensitive,
        Vec::new(),
        Default::default(),
    ))
}

fn pipeline(
    name: &str,
    init: impl FnOnce() -> (
        Vec<gix_filter::Driver>,
        Vec<&'static encoding_rs::Encoding>,
        gix_filter::pipeline::CrlfRoundTripCheck,
        eol::Configuration,
    ),
) -> gix_testtools::Result<(gix_worktree::Stack, gix_filter::Pipeline)> {
    let cache = attribute_cache(name)?;
    let (drivers, encodings_with_roundtrip_check, crlf_roundtrip_check, eol_config) = init();
    let pipe = gix_filter::Pipeline::new(
        Default::default(),
        gix_filter::pipeline::Options {
            drivers,
            eol_config,
            encodings_with_roundtrip_check,
            crlf_roundtrip_check,
            object_hash: gix_testtools::hash_kind_from_env().unwrap_or_default(),
        },
    );
    Ok((cache, pipe))
}

use bstr::ByteSlice;
use gix_attributes::glob::pattern::Case;
use gix_filter::{eol, pipeline::convert::to_worktree};
use gix_testtools::TestResult;

mod convert_to_git;
mod convert_to_worktree;

#[test]
fn default_options() -> TestResult {
    let mut filters = gix_filter::Pipeline::new(Default::default(), gix_testtools::object_hash(), Default::default());
    let out = filters.convert_to_worktree(
        b"hi",
        "file".into(),
        &mut |_, _| {},
        to_worktree::Options {
            can_delay: gix_filter::driver::apply::Delay::Allow,
            unknown_encoding: to_worktree::UnknownEncoding::Fail,
        },
    )?;
    assert_eq!(
        out.as_bytes().expect("unchanged").as_bstr(),
        "hi",
        "default-pipelines can be used like normal, they have not effect"
    );
    Ok(())
}

fn fixture(name: &str) -> gix_testtools::Result<std::path::PathBuf> {
    Ok(gix_testtools::scripted_fixture_read_only("pipeline_repos.sh")?.join(name))
}

fn attribute_cache(name: &str) -> gix_testtools::Result<gix_worktree::Stack> {
    let dir = fixture(name)?;
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
        gix_testtools::object_hash(),
        gix_filter::pipeline::Options {
            drivers,
            eol_config,
            encodings_with_roundtrip_check,
            crlf_roundtrip_check,
        },
    );
    Ok((cache, pipe))
}

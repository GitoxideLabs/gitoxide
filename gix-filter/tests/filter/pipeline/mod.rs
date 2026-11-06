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

#[test]
fn encoding_attributes_preserve_utf8_bytes_and_latin1_controls() -> TestResult {
    use std::{io::Read, path::Path};

    for (fixture, worktree, utf8) in [
        ("utf8", b"\xff\xfe".as_slice(), b"\xff\xfe".as_slice()),
        ("latin1", b"\x80\x91\xff".as_slice(), "\u{80}\u{91}ÿ".as_bytes()),
    ] {
        let (mut cache, mut pipe) = pipeline(fixture, || {
            (Vec::new(), Vec::new(), Default::default(), Default::default())
        })?;
        let mut attributes = |path: &bstr::BStr, attrs: &mut gix_attributes::search::Outcome| {
            cache
                .at_entry(path, None, &gix_object::find::Never)
                .expect("fixture attributes can be read")
                .matching_attributes(attrs);
        };
        let mut actual = Vec::new();
        pipe.convert_to_git(worktree, Path::new("sample"), &mut attributes, &mut |_| Ok(None))?
            .read_to_end(&mut actual)?;
        assert_eq!(
            actual, utf8,
            "the encoding attribute uses Git's semantics for {fixture}"
        );
        actual.clear();
        pipe.convert_to_worktree(
            utf8,
            "sample".into(),
            &mut attributes,
            to_worktree::Options {
                can_delay: gix_filter::driver::apply::Delay::Forbid,
                unknown_encoding: to_worktree::UnknownEncoding::Fail,
            },
        )?
        .read_to_end(&mut actual)?;
        assert_eq!(actual, worktree, "checkout reverses the conversion for {fixture}");
    }
    Ok(())
}

#[test]
fn roundtrip_selection_matches_attribute_spelling_not_encoding_identity() -> TestResult {
    use gix_encoding::Encoding;
    use std::path::Path;

    let mut input = gix_encoding::convert(b"a", Encoding::Utf8, Encoding::Utf16)?;
    for unit in input.as_chunks_mut::<2>().0 {
        unit.swap(0, 1);
    }
    for (label, should_reject) in [("UTF-16", true), ("utf-16", true), ("UTF16", false), ("unknown", false)] {
        let (mut cache, mut pipe) = pipeline("utf16", || {
            (Vec::new(), vec![label.into()], Default::default(), Default::default())
        })?;
        let result = pipe.convert_to_git(
            input.as_slice(),
            Path::new("sample"),
            &mut |path, attrs| {
                cache
                    .at_entry(path, None, &gix_object::find::Never)
                    .expect("fixture attributes can be read")
                    .matching_attributes(attrs);
            },
            &mut |_| Ok(None),
        );
        assert_eq!(
            result.is_err(),
            should_reject,
            "{label}: Git compares original names ignoring case, and the opposite byte order cannot round-trip"
        );
    }
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
        Vec<bstr::BString>,
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

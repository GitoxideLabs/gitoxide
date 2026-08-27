use std::{io::Read, path::Path};

use bstr::ByteSlice;
use gix_filter::{eol, pipeline::CrlfRoundTripCheck};

use crate::{driver::apply::driver_with_process, pipeline::pipeline};

#[test]
fn no_driver_but_filter_with_autocrlf() -> gix_testtools::Result {
    let (_cache, mut pipe) = pipeline("no-filter", || {
        (
            vec![],
            Vec::new(),
            CrlfRoundTripCheck::Fail,
            eol::Configuration {
                auto_crlf: eol::AutoCrlf::Enabled,
                eol: None,
            },
        )
    })?;

    let mut out = pipe.convert_to_git(
        "hi\r\n".as_bytes(),
        Path::new("any.txt"),
        &mut |_path, _attrs| {},
        &mut no_object_in_index,
    )?;

    assert_eq!(
        out.as_bytes().expect("read converted to buffer").as_bstr(),
        "hi\n",
        "the read is read into memory if there is no driver"
    );
    let mut buf = Vec::new();
    out.read_to_end(&mut buf)?;
    assert_eq!(buf.as_bstr(), "hi\n", "we can consume the output");
    Ok(())
}

#[test]
fn all_stages_mean_streaming_is_impossible() -> gix_testtools::Result {
    let (mut cache, mut pipe) = pipeline("all-filters", || {
        (
            vec![driver_with_process()],
            Vec::new(),
            CrlfRoundTripCheck::Fail,
            Default::default(),
        )
    })?;

    let source_hash = match gix_testtools::object_hash() {
        gix_hash::Kind::Sha1 => "2188d1cdee2b93a80084b61af431a49d21bc7cc0",
        gix_hash::Kind::Sha256 => "66b8b3bf4f18bcb5f74e09b24ac62e10934e9453a1de9793edb9390dc2ab1d6b",
        _ => unimplemented!(),
    };
    let src = format!("➡a\r\n➡b\r\n➡$Id: {source_hash}$");
    let mut out = pipe.convert_to_git(
        src.as_bytes(),
        Path::new("any.txt"),
        &mut |path, attrs| {
            cache
                .at_entry(path, None, &gix_object::find::Never)
                .expect("cannot fail")
                .matching_attributes(attrs);
        },
        &mut no_object_in_index,
    )?;
    assert!(out.is_changed(), "filters were applied");
    assert!(out.as_read().is_none(), "non-driver filters operate in-memory");
    let buf = out.as_bytes().expect("in-memory operation");
    assert_eq!(buf.as_bstr(), "a\nb\n$Id$", "filters were successfully reversed");
    Ok(())
}

#[test]
fn only_driver_means_streaming_is_possible() -> gix_testtools::Result {
    let (mut cache, mut pipe) = pipeline("driver-only", || {
        (
            vec![driver_with_process()],
            Vec::new(),
            CrlfRoundTripCheck::Skip,
            Default::default(),
        )
    })?;

    let source_hash = match gix_testtools::object_hash() {
        gix_hash::Kind::Sha1 => "2188d1cdee2b93a80084b61af431a49d21bc7cc0",
        gix_hash::Kind::Sha256 => "66b8b3bf4f18bcb5f74e09b24ac62e10934e9453a1de9793edb9390dc2ab1d6b",
        _ => unimplemented!(),
    };
    let src = format!("➡a\r\n➡b\r\n➡$Id: {source_hash}$");
    let mut out = pipe.convert_to_git(
        src.as_bytes(),
        Path::new("subdir/doesnot/matter/any.txt"),
        &mut |path, attrs| {
            cache
                .at_entry(path, None, &gix_object::find::Never)
                .expect("cannot fail")
                .matching_attributes(attrs);
        },
        &mut no_object_in_index,
    )?;
    assert!(out.is_changed(), "filters were applied");
    assert!(out.as_read().is_some(), "filter-only can be streamed");
    let mut buf = Vec::new();
    out.read_to_end(&mut buf)?;
    assert_eq!(
        buf.as_bstr(),
        format!("a\r\nb\r\n$Id: {source_hash}$"),
        "one filter was reversed"
    );
    Ok(())
}

/// The whole point of a `process` filter is that it stays alive for the next file, so the `Pipeline` is
/// the only handle to it. That makes this the shape in which the defect reached users: `gix` owns its
/// pipelines privately - `Repository::status()` and checkout never hand one back - so nobody is in a
/// position to call [`shutdown()`][gix_filter::driver::State::shutdown()] on what they launched, and
/// dropping the pipeline has to be enough.
#[cfg(unix)]
#[test]
fn a_dropped_pipeline_reaps_the_filter_it_launched() -> gix_testtools::Result {
    let (mut cache, mut pipe) = pipeline("driver-only", || {
        (
            vec![driver_with_process()],
            Vec::new(),
            CrlfRoundTripCheck::Skip,
            Default::default(),
        )
    })?;

    let mut out = pipe.convert_to_git(
        "➡a\n".as_bytes(),
        Path::new("any.txt"),
        &mut |path, attrs| {
            cache
                .at_entry(path, None, &gix_object::find::Never)
                .expect("cannot fail")
                .matching_attributes(attrs);
        },
        &mut no_object_in_index,
    )?;
    let mut buf = Vec::new();
    out.read_to_end(&mut buf)?;
    assert_eq!(
        buf.as_bstr(),
        "a\n",
        "the `process` filter did its work, so it is definitely running"
    );
    drop(out);

    let pid = match pipe.driver_state_mut().maybe_launch_process(
        &driver_with_process(),
        gix_filter::driver::Operation::Clean,
        "any.txt".into(),
    )? {
        Some(gix_filter::driver::Process::MultiFile { client, .. }) => client.id(),
        _ => unreachable!("the driver declares a `process` filter, which is still running"),
    };
    drop(pipe);

    assert_eq!(
        crate::reap::observe(pid),
        crate::reap::Child::Reaped,
        "dropping the pipeline terminated its filter and waited for it"
    );
    Ok(())
}

#[test]
fn no_filter_means_reader_is_returned_unchanged() -> gix_testtools::Result {
    let (mut cache, mut pipe) = pipeline("no-filters", || {
        (vec![], Vec::new(), CrlfRoundTripCheck::Fail, Default::default())
    })?;

    let source_hash = match gix_testtools::object_hash() {
        gix_hash::Kind::Sha1 => "2188d1cdee2b93a80084b61af431a49d21bc7cc0",
        gix_hash::Kind::Sha256 => "66b8b3bf4f18bcb5f74e09b24ac62e10934e9453a1de9793edb9390dc2ab1d6b",
        _ => unimplemented!(),
    };
    let input = format!("➡a\r\n➡b\r\n➡$Id: {source_hash}$");
    let mut out = pipe.convert_to_git(
        input.as_bytes(),
        Path::new("other.txt"),
        &mut |path, attrs| {
            cache
                .at_entry(path, None, &gix_object::find::Never)
                .expect("cannot fail")
                .matching_attributes(attrs);
        },
        &mut no_call,
    )?;
    assert!(!out.is_changed(), "no filter was applied");
    let actual = out
        .as_read()
        .expect("input is unchanged, we get the original stream back");
    let mut buf = Vec::new();
    actual.read_to_end(&mut buf)?;
    assert_eq!(buf.as_bstr(), input, "input is unchanged");
    Ok(())
}

fn no_call(_buf: &mut Vec<u8>) -> Result<Option<()>, Box<dyn std::error::Error + Send + Sync>> {
    unreachable!("index function will not be called")
}

fn no_object_in_index(_buf: &mut Vec<u8>) -> Result<Option<()>, Box<dyn std::error::Error + Send + Sync>> {
    Ok(None)
}

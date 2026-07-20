use std::io::Read;

use bstr::ByteSlice;
use gix_filter::pipeline::CrlfRoundTripCheck;

use crate::{driver::apply::driver_with_process, pipeline::pipeline};

#[test]
fn all_stages() -> gix_testtools::Result {
    let (mut cache, mut pipe) = pipeline("all-filters", || {
        (
            vec![driver_with_process()],
            Vec::new(),
            CrlfRoundTripCheck::Skip,
            Default::default(),
        )
    })?;

    let mut out = pipe.convert_to_worktree(
        b"a\nb\n$Id$",
        "any.txt".into(),
        &mut |path, attrs| {
            cache
                .at_entry(path, None, &gix_object::find::Never)
                .expect("cannot fail")
                .matching_attributes(attrs);
        },
        gix_filter::driver::apply::Delay::Forbid,
    )?;
    assert!(out.is_changed(), "filters were applied");
    assert!(
        out.as_bytes().is_none(),
        "the last filter is a driver which is applied, yielding a stream"
    );
    assert!(out.as_read().is_some(), "process filter is last");
    let mut buf = Vec::new();
    out.read_to_end(&mut buf)?;
    let expected_hash = match gix_testtools::object_hash() {
        gix_hash::Kind::Sha1 => "2188d1cdee2b93a80084b61af431a49d21bc7cc0",
        gix_hash::Kind::Sha256 => "66b8b3bf4f18bcb5f74e09b24ac62e10934e9453a1de9793edb9390dc2ab1d6b",
        _ => unimplemented!(),
    };
    assert_eq!(
        buf.as_bstr(),
        format!("➡a\r\n➡b\r\n➡$Id: {expected_hash}$"),
        "the buffer shows that a lot of transformations were applied"
    );
    Ok(())
}

#[test]
fn all_stages_no_filter() -> gix_testtools::Result {
    let (mut cache, mut pipe) = pipeline("all-filters", || {
        (vec![], Vec::new(), CrlfRoundTripCheck::Skip, Default::default())
    })?;

    let mut out = pipe.convert_to_worktree(
        b"$Id$a\nb\n",
        "other.txt".into(),
        &mut |path, attrs| {
            cache
                .at_entry(path, None, &gix_object::find::Never)
                .expect("cannot fail")
                .matching_attributes(attrs);
        },
        gix_filter::driver::apply::Delay::Forbid,
    )?;
    assert!(out.is_changed(), "filters were applied");
    assert!(
        out.as_read().is_none(),
        "there is no filter process, so no chance for getting a stream"
    );
    let buf = out.as_bytes().expect("no filter process");
    let expected_hash = match gix_testtools::object_hash() {
        gix_hash::Kind::Sha1 => "a77d7acbc809ac8df987a769221c83137ba1b9f9",
        gix_hash::Kind::Sha256 => "5ac811252c70ca9761feaa6fe00a74fbf558378ff4fc2853e43b097b153bd7eb",
        _ => unimplemented!(),
    };
    assert_eq!(
        buf.as_bstr(),
        format!("$Id: {expected_hash}$a\r\nb\r\n"),
        "the buffer shows that a lot of transformations were applied"
    );
    Ok(())
}

#[test]
fn no_filter() -> gix_testtools::Result {
    let (mut cache, mut pipe) = pipeline("no-filters", || {
        (vec![], Vec::new(), CrlfRoundTripCheck::Skip, Default::default())
    })?;

    let input = b"$Id$a\nb\n";
    let out = pipe.convert_to_worktree(
        input,
        "other.txt".into(),
        &mut |path, attrs| {
            cache
                .at_entry(path, None, &gix_object::find::Never)
                .expect("cannot fail")
                .matching_attributes(attrs);
        },
        gix_filter::driver::apply::Delay::Forbid,
    )?;
    assert!(!out.is_changed(), "no filter was applied");
    let actual = out.as_bytes().expect("input is unchanged");
    assert_eq!(actual, input, "so the input is unchanged…");
    assert_eq!(actual.as_ptr(), input.as_ptr(), "…which means it's exactly the same");
    Ok(())
}

#[test]
fn unknown_encoding_is_ignored_while_other_filters_are_applied() -> gix_testtools::Result {
    let (mut cache, mut pipe) = pipeline("unknown-encoding", || {
        (
            vec![driver_with_process()],
            Vec::new(),
            CrlfRoundTripCheck::Skip,
            Default::default(),
        )
    })?;

    let mut out = pipe.convert_to_worktree(
        b"one\ntwo\n",
        "file".into(),
        &mut |path, attrs| {
            cache
                .at_entry(path, None, &gix_object::find::Never)
                .expect("attribute lookup succeeds")
                .matching_attributes(attrs);
        },
        gix_filter::driver::apply::Delay::Forbid,
    )?;
    let mut actual = Vec::new();
    out.read_to_end(&mut actual)?;
    assert_eq!(
        actual.as_bstr(),
        "➡one\r\n➡two\r\n",
        "unknown encodings only skip the encoding stage"
    );
    Ok(())
}

#[test]
fn unknown_encoding_can_be_reported_or_rejected() -> gix_testtools::Result {
    let (mut cache, mut pipe) = pipeline("unknown-encoding", || {
        (
            vec![driver_with_process()],
            Vec::new(),
            CrlfRoundTripCheck::Skip,
            Default::default(),
        )
    })?;
    let mut attributes = |path: &bstr::BStr, attrs: &mut gix_attributes::search::Outcome| {
        cache
            .at_entry(path, None, &gix_object::find::Never)
            .expect("attribute lookup succeeds")
            .matching_attributes(attrs);
    };

    let converted = pipe.convert_to_worktree_with_options(
        b"one\n",
        "file".into(),
        &mut attributes,
        gix_filter::pipeline::convert::to_worktree::Options {
            delay: gix_filter::driver::apply::Delay::Forbid,
            ..Default::default()
        },
    )?;
    let gix_filter::pipeline::convert::to_worktree::Outcome { output, ignored_error } = converted;
    drop(output);
    assert_eq!(
        ignored_error.expect("unknown encoding was ignored").to_string(),
        "The encoding named 'definitely-not-an-encoding' isn't available"
    );

    let (mut strict_cache, mut strict_pipe) = pipeline("unknown-encoding", || {
        (
            vec![driver_with_process()],
            Vec::new(),
            CrlfRoundTripCheck::Skip,
            Default::default(),
        )
    })?;
    let err = match strict_pipe.convert_to_worktree_with_options(
        b"one\n",
        "file".into(),
        &mut |path, attrs| {
            strict_cache
                .at_entry(path, None, &gix_object::find::Never)
                .expect("attribute lookup succeeds")
                .matching_attributes(attrs);
        },
        gix_filter::pipeline::convert::to_worktree::Options {
            delay: gix_filter::driver::apply::Delay::Forbid,
            unknown_encoding: gix_filter::pipeline::convert::to_worktree::UnknownEncoding::Fail,
        },
    ) {
        Ok(_) => panic!("strict handling rejects unknown encodings"),
        Err(err) => err,
    };
    assert!(
        matches!(
            err,
            gix_filter::pipeline::convert::to_worktree::Error::Configuration(
                gix_filter::pipeline::convert::configuration::Error::UnknownEncoding { .. }
            )
        ),
        "the configuration error remains available to strict callers"
    );
    Ok(())
}

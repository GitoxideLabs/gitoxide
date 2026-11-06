use std::{io::Read, path::Path};

use bstr::ByteSlice;
use gix_filter::{eol, pipeline::CrlfRoundTripCheck};

use crate::{
    driver::apply::driver_with_process,
    pipeline::{fixture, pipeline},
};

#[test]
fn no_driver_but_filter_with_autocrlf() -> gix_testtools::TestResult {
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
fn all_stages_mean_streaming_is_impossible() -> gix_testtools::TestResult {
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
fn only_driver_means_streaming_is_possible() -> gix_testtools::TestResult {
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

#[test]
fn no_filter_means_reader_is_returned_unchanged() -> gix_testtools::TestResult {
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

#[test]
fn plain_utf16_is_rejected() -> gix_testtools::TestResult {
    let dir = fixture("utf16")?;
    let (mut cache, mut pipe) = pipeline("utf16", || {
        (vec![], Vec::new(), CrlfRoundTripCheck::Skip, Default::default())
    })?;
    for file in ["bomless.UTF-16", "le-bom.UTF-16", "be-bom.UTF-16"] {
        let input = std::fs::read(dir.join(file))?;
        let err = pipe
            .convert_to_git(
                input.as_slice(),
                Path::new(file),
                &mut |path, attrs| {
                    cache
                        .at_entry(path, None, &gix_object::find::Never)
                        .expect("fixture attributes are readable")
                        .matching_attributes(attrs);
                },
                &mut no_call,
            )
            .err()
            .expect("plain UTF-16 cannot preserve Git's BOM-required semantics");
        assert!(err.is_validation(), "{file}: unsupported UTF-16 is a validation error");
        assert_eq!(
            err.error().to_string(),
            "The UTF-16 encoding 'UTF-16' is not supported; use UTF-16LE or UTF-16BE without a BOM",
            "{file}: reject the label instead of silently treating it as UTF-16LE"
        );
    }
    Ok(())
}

#[test]
fn explicit_utf16_rejects_both_boms_regardless_of_roundtrip_check() -> gix_testtools::TestResult {
    let dir = fixture("utf16")?;
    for encoding in [encoding_rs::UTF_16LE, encoding_rs::UTF_16BE] {
        for check_roundtrip in [false, true] {
            let (mut cache, mut pipe) = pipeline("utf16", || {
                (
                    vec![],
                    if check_roundtrip { vec![encoding] } else { Vec::new() },
                    CrlfRoundTripCheck::Skip,
                    Default::default(),
                )
            })?;
            for prefix in ["bomless", "le-bom", "be-bom"] {
                let file = format!("{prefix}.{}", encoding.name());
                let input = std::fs::read(dir.join(&file))?;
                let result = pipe.convert_to_git(
                    input.as_slice(),
                    Path::new(&file),
                    &mut |path, attrs| {
                        cache
                            .at_entry(path, None, &gix_object::find::Never)
                            .expect("fixture attributes are readable")
                            .matching_attributes(attrs);
                    },
                    &mut no_call,
                );
                if prefix == "bomless" {
                    assert_eq!(
                        result?.as_bytes().expect("encoding converts in memory"),
                        std::fs::read(dir.join(format!("{file}.git")))?,
                        "{file}, roundtrip={check_roundtrip}: BOMless input agrees with Git"
                    );
                } else {
                    let err = result.err().expect("either BOM is forbidden with explicit endianness");
                    assert!(err.is_validation(), "{file}: BOM rejection is a validation error");
                    assert!(
                        err.error().to_string().contains("BOM"),
                        "{file}, roundtrip={check_roundtrip}: reject the BOM before decoding or round-trip checking: {err:#}"
                    );
                }
            }
        }
    }
    Ok(())
}

fn no_call(_buf: &mut Vec<u8>) -> gix_error::Result<Option<()>> {
    unreachable!("index function will not be called")
}

fn no_object_in_index(_buf: &mut Vec<u8>) -> gix_error::Result<Option<()>> {
    Ok(None)
}

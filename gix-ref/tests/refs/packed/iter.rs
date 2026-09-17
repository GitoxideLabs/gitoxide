use gix_ref::packed;

use crate::file::{store_at, store_with_packed_refs};

const HASH_KIND: gix_hash::Kind = gix_hash::Kind::Sha1;

#[test]
fn empty() -> crate::Result {
    assert_eq!(
        packed::Iter::new(&[], HASH_KIND)?.count(),
        0,
        "empty buffers are fine and lead to no line returned"
    );
    Ok(())
}

#[test]
fn packed_refs_with_header() -> crate::Result {
    let dir = crate::scripted_fixture_read_only("make_packed_ref_repository.sh")?;
    let buf = std::fs::read(dir.join(".git").join("packed-refs"))?;
    let iter = packed::Iter::new(&buf, crate::fixture_hash_kind())?;
    assert_eq!(iter.count(), 11, "it finds the right amount of items");
    Ok(())
}

#[test]
fn iter_prefix() -> crate::Result {
    let packed = store_with_packed_refs()?.open_packed_buffer()?.expect("packed-refs");
    assert_eq!(
        packed
            .iter_prefixed("refs/heads/".into())?
            .collect::<Result<Vec<_>, _>>()?,
        vec!["refs/heads/A", "refs/heads/d1", "refs/heads/dt1", "refs/heads/main"]
    );

    assert_eq!(
        packed
            .iter_prefixed("refs/heads/d".into())?
            .collect::<Result<Vec<_>, _>>()?,
        vec!["refs/heads/d1", "refs/heads/dt1"],
        "partial prefixes are fine, they don't have to resemble or be a directory"
    );

    assert_eq!(
        packed
            .iter_prefixed("refs/remotes/".into())?
            .collect::<Result<Vec<_>, _>>()?,
        vec!["refs/remotes/origin/main", "refs/remotes/origin/multi-link-target3",]
    );

    let last_ref_in_file = "refs/tags/t1";
    assert_eq!(
        packed
            .iter_prefixed(last_ref_in_file.into())?
            .collect::<Result<Vec<_>, _>>()?,
        vec![last_ref_in_file],
        "prefixes which are a ref also work, this one is the last of the file"
    );
    let first_ref_in_file = "refs/d1";
    assert_eq!(
        packed
            .iter_prefixed(first_ref_in_file.into())?
            .collect::<Result<Vec<_>, _>>()?,
        vec![first_ref_in_file],
        "prefixes which are a ref also work, and this one at the beginning of the file"
    );
    Ok(())
}

#[test]
fn packed_refs_without_header() -> crate::Result {
    let packed_refs = b"916840c0e2f67d370291042cb5274a597f4fa9bc refs/tags/TEST-0.0.1
c4cebba92af964f2d126be90b8a6298c4cf84d45 refs/tags/gix-actor-v0.1.0
^13da90b54699a6b500ec5cd7d175f2cd5a1bed06
0b92c8a256ae06c189e3b9c30b646d62ac8f7d10 refs/tags/gix-actor-v0.1.1\n";
    assert_eq!(
        packed::Iter::new(packed_refs, HASH_KIND)?.collect::<Result<Vec<_>, _>>()?,
        vec![
            packed::Reference {
                name: "refs/tags/TEST-0.0.1".try_into()?,
                target: "916840c0e2f67d370291042cb5274a597f4fa9bc".into(),
                object: None
            },
            packed::Reference {
                name: "refs/tags/gix-actor-v0.1.0".try_into()?,
                target: "c4cebba92af964f2d126be90b8a6298c4cf84d45".into(),
                object: Some("13da90b54699a6b500ec5cd7d175f2cd5a1bed06".into())
            },
            packed::Reference {
                name: "refs/tags/gix-actor-v0.1.1".try_into()?,
                target: "0b92c8a256ae06c189e3b9c30b646d62ac8f7d10".into(),
                object: None
            }
        ]
    );
    Ok(())
}

#[test]
fn broken_ref_doesnt_end_the_iteration() -> crate::Result {
    let packed_refs = b"916840c0e2f67d370291042cb5274a597f4fa9bc refs/tags/TEST-0.0.1
buggy-hash refs/wrong
^buggy-hash-too
0b92c8a256ae06c189e3b9c30b646d62ac8f7d10 refs/tags/gix-actor-v0.1.1\n";
    let mut iter = packed::Iter::new(packed_refs, HASH_KIND)?;

    assert!(iter.next().expect("first ref").is_ok(), "first line is valid");
    for (line, input) in [(2_u64, b"buggy-hash refs/wrong".as_slice()), (3, b"^buggy-hash-too")] {
        let err = iter.next().expect("invalid line").expect_err("invalid reference");
        assert!(err.is_corrupted());
        assert_eq!(err.values["line"], gix_error::Value::from(line));
        assert_eq!(err.values["input"], gix_error::Value::from(input));
    }
    assert!(iter.next().expect("last ref").is_ok(), "last line is valid");
    assert!(iter.next().is_none(), "exhausted");
    Ok(())
}

#[test]
fn performance() -> crate::Result {
    let store = store_at("make_repository_with_lots_of_packed_refs.sh")?;
    let start = std::time::Instant::now();
    let actual = store
        .open_packed_buffer()?
        .expect("packed-refs present")
        .iter()?
        .count();
    assert_eq!(actual, 150003);
    let elapsed = start.elapsed().as_secs_f32();
    eprintln!(
        "Enumerated {} refs in {}s ({} refs/s)",
        actual,
        elapsed,
        actual as f32 / elapsed
    );
    Ok(())
}

#[test]
fn error_metadata_counts_peeled_lines_and_retains_unterminated_input() -> crate::Result {
    let input = format!("{0} refs/tags/one\n^{0}\nbroken", HASH_KIND.null());
    let mut iter = packed::Iter::new(input.as_bytes(), HASH_KIND)?;
    iter.next().expect("peeled tag")?;
    let err = iter
        .next()
        .expect("last line")
        .expect_err("malformed reference")
        .into_error();
    let details = err.metadata().next().expect("line details survive conversion");
    assert_eq!(details.values["line"], gix_error::Value::from(3_u64));
    assert_eq!(details.values["input"], gix_error::Value::from(b"broken".as_slice()));
    assert!(iter.next().is_none());
    Ok(())
}

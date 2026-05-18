use gix_ref::packed;
use gix_testtools::fixture_path;

use crate::{
    file::{store_at, store_with_packed_refs},
    hex_to_id,
    packed::write_packed_refs_with,
};

const HASH_KIND: gix_hash::Kind = gix_hash::Kind::Sha1;

#[test]
fn a_lock_file_would_not_be_a_valid_partial_name() {
    // doesn't really belong here but want to make sure refname validation works as expected.
    // let err: &gix_ref::PartialNameRef = "heads/hello.lock".try_into().expect_err("this should fail");
    let err = <&gix_ref::PartialNameRef as TryFrom<_>>::try_from("heads/hello.lock").expect_err("this should fail");
    assert_eq!(err.to_string(), "Reference name cannot end with '.lock'");
}

#[test]
fn capitalized_branch() -> crate::Result {
    let store = store_with_packed_refs()?;
    let packed_refs = store.open_packed_buffer()?.expect("packed-refs exist");

    assert_eq!(
        packed_refs.find("A")?.name.as_bstr(),
        "refs/heads/A",
        "fully capitalized refs aren't just considered pseudorefs"
    );
    Ok(())
}

#[test]
fn all_iterable_refs_can_be_found() -> crate::Result {
    let store = store_with_packed_refs()?;
    let packed_refs = store.open_packed_buffer()?.expect("packed-refs exist");

    for reference in packed_refs.iter()? {
        let reference = reference?;
        let found = packed_refs.try_find(reference.name)?.expect("reference exists");
        assert_eq!(reference, found, "both refs are exactly the same");
        let found = packed_refs.find(reference.name)?;
        assert_eq!(reference, found);
    }
    Ok(())
}

#[test]
fn binary_search_a_name_past_the_end_of_the_packed_refs_file() -> crate::Result {
    let packed_refs = packed::Buffer::open(
        fixture_path("packed-refs").join("triggers-out-of-bounds"),
        32,
        HASH_KIND,
    )?;
    assert!(packed_refs.try_find("v0.0.1")?.is_none());
    Ok(())
}

#[test]
fn find_packed_refs_with_peeled_items_and_full_or_partial_names() -> crate::Result {
    let packed_refs = b"# pack-refs with: peeled fully-peeled sorted
916840c0e2f67d370291042cb5274a597f4fa9bc refs/tags/TEST-0.0.1
c4cebba92af964f2d126be90b8a6298c4cf84d45 refs/tags/gix-actor-v0.1.0
^13da90b54699a6b500ec5cd7d175f2cd5a1bed06
0b92c8a256ae06c189e3b9c30b646d62ac8f7d10 refs/tags/gix-actor-v0.1.1\n";
    let (_keep, path) = write_packed_refs_with(packed_refs)?;

    let buf = packed::Buffer::open(path, 1024, HASH_KIND)?;
    let name = "refs/tags/TEST-0.0.1";
    assert_eq!(
        buf.try_find(name)?.expect("reference exists"),
        packed::Reference {
            name: name.try_into()?,
            target: "916840c0e2f67d370291042cb5274a597f4fa9bc".into(),
            object: None
        }
    );
    let name = "refs/tags/gix-actor-v0.1.0";
    assert_eq!(
        buf.try_find(name)?.expect("reference exists"),
        packed::Reference {
            name: name.try_into()?,
            target: "c4cebba92af964f2d126be90b8a6298c4cf84d45".into(),
            object: Some("13da90b54699a6b500ec5cd7d175f2cd5a1bed06".into())
        }
    );
    let name = "refs/tags/gix-actor-v0.1.1";
    assert_eq!(
        buf.try_find(name)?.expect("reference exists"),
        packed::Reference {
            name: name.try_into()?,
            target: "0b92c8a256ae06c189e3b9c30b646d62ac8f7d10".into(),
            object: None
        }
    );
    Ok(())
}

#[test]
fn partial_name_to_full_name_conversion_rules_are_applied() -> crate::Result {
    let store = store_at("make_packed_refs_for_lookup_rules.sh")?;
    let packed = store.open_packed_buffer()?.expect("packed-refs exists");

    assert_eq!(
        store.find_loose("origin")?.name.as_bstr(),
        "refs/remotes/origin/HEAD",
        "a special that only applies to loose refs"
    );
    assert!(
        packed.try_find("origin")?.is_none(),
        "packed refs don't have this special case as they don't store HEADs or symrefs"
    );
    assert_eq!(
        store.find_loose("HEAD")?.name.as_bstr(),
        "HEAD",
        "HEAD can be found in loose stores"
    );
    assert!(
        packed.try_find("HEAD")?.is_none(),
        "packed refs definitely don't contain HEAD"
    );
    assert_eq!(
        packed.try_find("head-or-tag")?.expect("present").name.as_bstr(),
        "refs/tags/head-or-tag",
        "it finds tags first"
    );
    assert_eq!(
        packed.try_find("heads/head-or-tag")?.expect("present").name.as_bstr(),
        "refs/heads/head-or-tag",
        "it finds heads when disambiguated"
    );
    assert_eq!(
        packed.try_find("main")?.expect("present").name.as_bstr(),
        "refs/heads/main",
        "it finds local heads before remote ones"
    );
    assert_eq!(
        packed.try_find("origin/main")?.expect("present").name.as_bstr(),
        "refs/remotes/origin/main",
        "it finds remote heads when disambiguated"
    );
    assert_eq!(
        packed.try_find("remotes/origin/main")?.expect("present").name.as_bstr(),
        "refs/remotes/origin/main",
        "more specification is possible, too"
    );
    let target = hex_to_id("b3109a7e51fc593f85b145a76c70ddd1d133fafd").to_string();
    let object = hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03").to_string();
    assert_eq!(
        packed.try_find("tag-object")?.expect("present"),
        packed::Reference {
            name: "refs/tags/tag-object".try_into()?,
            target: target.as_str().into(),
            object: Some(object.as_str().into())
        },
        "tag objects aren't special, but lets test a little more"
    );
    Ok(())
}

#[test]
fn invalid_refs_within_a_file_do_not_lead_to_incorrect_results() -> crate::Result {
    let broken_packed_refs = b"# pack-refs with: peeled fully-peeled sorted
916840c0e2f67d370291042cb5274a597f4fa9bc refs/tags/TEST-0.0.1
bogus refs/tags/gix-actor-v0.1.0
^13da90b54699a6b500ec5cd7d175f2cd5a1bed06
0b92c8a256ae06c189e3b9c30b646d62ac8f7d10 refs/tags/gix-actor-v0.1.1\n";
    let (_keep, path) = write_packed_refs_with(broken_packed_refs)?;

    let buf = packed::Buffer::open(path, 1024, HASH_KIND)?;

    let name = "refs/tags/gix-actor-v0.1.1";
    assert_eq!(
        buf.try_find(name)?.expect("reference exists"),
        packed::Reference {
            name: name.try_into()?,
            target: "0b92c8a256ae06c189e3b9c30b646d62ac8f7d10".into(),
            object: None
        }
    );

    for failing_name in &["refs/tags/TEST-0.0.1", "refs/tags/gix-actor-v0.1.0"] {
        assert_eq!(
            buf.try_find(*failing_name)
                .expect_err("it should detect an err")
                .to_string(),
            "The reference could not be parsed"
        );
    }
    Ok(())
}

/// Build a packed-refs body with `count` valid records whose names are
/// `refs/heads/auto-NNNN`. The body is sorted, satisfying the "sorted"
/// header so `packed::Buffer::open` keeps the original byte layout.
fn synthetic_packed_refs(count: usize) -> Vec<u8> {
    let mut out: Vec<u8> = b"# pack-refs with: peeled fully-peeled sorted\n".to_vec();
    for i in 0..count {
        let line = format!("{:040x} refs/heads/auto-{i:04}\n", i as u128 + 1);
        out.extend_from_slice(line.as_bytes());
    }
    out
}

/// Mixed corpus: `count` valid records plus a malformed line spliced in
/// the middle. Returned names are sorted so the file remains valid.
fn synthetic_packed_refs_with_corruption(count: usize) -> Vec<u8> {
    let mut out: Vec<u8> = b"# pack-refs with: peeled fully-peeled sorted\n".to_vec();
    for i in 0..count {
        let line = format!("{:040x} refs/heads/auto-{i:04}\n", i as u128 + 1);
        out.extend_from_slice(line.as_bytes());
        if i == count / 2 {
            // A line with the wrong shape (no space after the would-be hash) —
            // `decode::reference` will reject it while leaving the surrounding
            // records intact.
            out.extend_from_slice(b"bogus refs/heads/auto-malformed\n");
        }
    }
    out
}

#[test]
fn many_lookups_keep_returning_correct_results_across_the_index_threshold() -> crate::Result {
    // Cross the lazy-index build threshold and verify every lookup — both
    // the binary-search ones below it and the index-served ones above it —
    // returns the same record we'd iterate.
    let body = synthetic_packed_refs(64);
    let (_keep, path) = write_packed_refs_with(&body)?;
    let buf = packed::Buffer::open(path, 1024, HASH_KIND)?;

    let names: Vec<_> = (0..64).map(|i| format!("refs/heads/auto-{i:04}")).collect();
    for name in &names {
        let found = buf.try_find(name.as_str())?.expect("ref is present");
        assert_eq!(found.name.as_bstr(), name.as_str(), "name round-trips");
    }
    // A miss against a clean index reports `Ok(None)`, not `Err(Parse)`.
    assert!(
        buf.try_find("refs/heads/does-not-exist")?.is_none(),
        "missing refs return Ok(None) when no malformed records are present"
    );
    Ok(())
}

#[test]
fn index_path_surfaces_parse_failures_on_miss() -> crate::Result {
    // Build a body large enough that `try_find` will trip the index-build
    // threshold, then verify that a miss surfaces `Error::Parse` because a
    // malformed record was encountered while building the index — matching
    // the binary-search path's `encountered_parse_failure` semantics.
    let body = synthetic_packed_refs_with_corruption(32);
    let (_keep, path) = write_packed_refs_with(&body)?;
    let buf = packed::Buffer::open(path, 1024, HASH_KIND)?;

    // First do enough valid lookups to push past `INDEX_BUILD_AFTER_LOOKUPS`
    // and force the index to be built.
    for i in 0..16 {
        let name = format!("refs/heads/auto-{i:04}");
        let found = buf.try_find(name.as_str())?.expect("ref exists");
        assert_eq!(found.name.as_bstr(), name.as_str());
    }

    // Now ask for a name that doesn't exist. Because the malformed record
    // is in the file, the index records `encountered_parse_failure=true`,
    // and a miss surfaces as a parse error rather than `Ok(None)`.
    assert_eq!(
        buf.try_find("refs/heads/never-there")
            .expect_err("miss against a corrupt file must surface as Error::Parse")
            .to_string(),
        "The reference could not be parsed",
        "index lookup matches the binary-search path's behavior on miss when corruption exists",
    );
    Ok(())
}

/// Profiling helper: simulate a wide-refs fetch's `update_refs` loop and
/// break down where the time goes. Run with:
///
/// ```text
/// cargo test -p gix-ref --release --features sha1 --test refs \
///     -- --ignored --nocapture loose_stat_overhead_profile
/// ```
///
/// Reports total time for two strategies against a ~150k-packed-ref repo:
///   A. `store.try_find(name)` — the current `try_find_reference` path,
///      which `stat`s the loose-ref file before falling through to packed.
///   B. Pre-built `HashSet` of loose names → bypass the stat when the name
///      isn't in it, then `packed.try_find(name)` directly.
///
/// The delta (A − B) is the loose-stat overhead that a hypothetical
/// `file::Store`-level loose-name cache (or a caller-side enumeration like
/// the original #2605) would reclaim on top of this PR's packed-buffer
/// index. Useful for deciding whether such a follow-up is worth its
/// complexity, since the answer is workload-dependent (warm vs cold cache,
/// loose-ref count, filesystem characteristics).
#[test]
#[ignore = "profiling only; expensive and times vary across machines"]
fn loose_stat_overhead_profile() -> crate::Result {
    use std::collections::HashSet;
    use std::time::Instant;

    let store = store_at("make_repository_with_lots_of_packed_refs.sh")?;
    let packed = store.open_packed_buffer()?.expect("packed-refs present");

    // Collect all 150k packed ref names — these are what fetch's update_refs
    // would iterate over.
    let collect_start = Instant::now();
    let packed_names: Vec<_> = packed
        .iter()?
        .filter_map(Result::ok)
        .map(|r| r.name.to_owned())
        .collect();
    eprintln!(
        "Collected {} packed ref names in {:?}",
        packed_names.len(),
        collect_start.elapsed()
    );

    // Strategy A — current behavior, with loose-stat per call.
    let warmup = Instant::now();
    for name in packed_names.iter().take(1000) {
        let _ = store.try_find(name.as_ref())?;
    }
    eprintln!("Warmup (1000 lookups): {:?}", warmup.elapsed());

    let a_start = Instant::now();
    let mut a_found = 0usize;
    for name in &packed_names {
        if store.try_find(name.as_ref())?.is_some() {
            a_found += 1;
        }
    }
    let a_elapsed = a_start.elapsed();
    eprintln!(
        "Strategy A (store.try_find — per-call loose stat): {:?}, found {} of {} ({:.1} µs/lookup)",
        a_elapsed,
        a_found,
        packed_names.len(),
        a_elapsed.as_secs_f64() * 1_000_000.0 / packed_names.len() as f64,
    );

    // Strategy B — simulated loose-index: pre-enumerate loose names, skip stat
    // when name isn't present.
    let loose_set_start = Instant::now();
    let loose_set: HashSet<_> = store.loose_iter()?.filter_map(Result::ok).map(|r| r.name).collect();
    eprintln!(
        "Built loose-name set ({} entries) in {:?}",
        loose_set.len(),
        loose_set_start.elapsed()
    );

    let b_start = Instant::now();
    let mut b_found = 0usize;
    for name in &packed_names {
        if loose_set.contains(name) {
            // Take slow path for shadowed entries.
            if store.try_find(name.as_ref())?.is_some() {
                b_found += 1;
            }
        } else if packed.try_find(name.as_ref())?.is_some() {
            b_found += 1;
        }
    }
    let b_elapsed = b_start.elapsed();
    eprintln!(
        "Strategy B (loose-set short-circuit + packed direct): {:?}, found {} of {} ({:.1} µs/lookup)",
        b_elapsed,
        b_found,
        packed_names.len(),
        b_elapsed.as_secs_f64() * 1_000_000.0 / packed_names.len() as f64,
    );

    eprintln!(
        "Loose-stat overhead reclaimable by an in-Store loose-name cache: {:?} ({:.1}% of A)",
        a_elapsed.saturating_sub(b_elapsed),
        (a_elapsed.saturating_sub(b_elapsed).as_secs_f64() / a_elapsed.as_secs_f64()) * 100.0,
    );
    assert_eq!(a_found, b_found, "both strategies find the same number of refs");
    Ok(())
}

#[test]
fn find_speed() -> crate::Result {
    let store = store_at("make_repository_with_lots_of_packed_refs.sh")?;
    let packed = store.open_packed_buffer()?.expect("packed-refs present");
    let start = std::time::Instant::now();
    let mut num_refs = 0;
    for r in packed.iter()?.take(10_000) {
        num_refs += 1;
        let r = r?;
        assert_eq!(
            packed.try_find(r.name)?.expect("ref was found"),
            r,
            "the refs are the same"
        );
    }
    let elapsed = start.elapsed().as_secs_f32();
    eprintln!(
        "Found {} refs in {}s ({} refs/s)",
        num_refs,
        elapsed,
        num_refs as f32 / elapsed
    );
    Ok(())
}

use gix_ref::file::Store;
use gix_serve::refs::collect_refs;

fn store_from(script: &str) -> Store {
    let path = gix_testtools::scripted_fixture_read_only_standalone(script).expect("fixture script should run");
    Store::at(
        path.join(".git"),
        gix_ref::store::init::Options {
            write_reflog: gix_ref::store::WriteReflog::Disable,
            object_hash: gix_hash::Kind::Sha1,
            precompose_unicode: false,
            prohibit_windows_device_names: false,
        },
    )
}

// --- simple repo: HEAD, main, feature ---

#[test]
fn simple_returns_head_as_symref() {
    let store = store_from("make_repo_simple.sh");
    let refs = collect_refs(&store).unwrap();

    let head = refs.iter().find(|r| r.name == "HEAD").expect("HEAD should exist");
    assert_eq!(
        head.symref_target.as_ref().map(|s| s.as_slice()),
        Some(b"refs/heads/main".as_slice()),
    );
}

#[test]
fn simple_includes_branches() {
    let store = store_from("make_repo_simple.sh");
    let refs = collect_refs(&store).unwrap();

    assert!(refs.iter().any(|r| r.name == "refs/heads/main"));
    assert!(refs.iter().any(|r| r.name == "refs/heads/feature"));
}

#[test]
fn simple_head_and_main_share_oid() {
    let store = store_from("make_repo_simple.sh");
    let refs = collect_refs(&store).unwrap();

    let head = refs.iter().find(|r| r.name == "HEAD").unwrap();
    let main = refs.iter().find(|r| r.name == "refs/heads/main").unwrap();
    assert_eq!(head.object_id, main.object_id);
}

#[test]
fn simple_all_have_valid_oids() {
    let store = store_from("make_repo_simple.sh");
    let refs = collect_refs(&store).unwrap();

    assert!(!refs.is_empty());
    for r in &refs {
        assert!(!r.object_id.is_null(), "{} has null oid", r.name);
    }
}

// --- repo with tags: lightweight + annotated, packed refs ---

#[test]
fn tags_includes_both_tag_types() {
    let store = store_from("make_repo_with_tags.sh");
    let refs = collect_refs(&store).unwrap();

    assert!(refs.iter().any(|r| r.name == "refs/tags/lightweight-tag"));
    assert!(refs.iter().any(|r| r.name == "refs/tags/v1.0"));
}

#[test]
fn tags_annotated_tag_has_peeled_oid() {
    let store = store_from("make_repo_with_tags.sh");
    let refs = collect_refs(&store).unwrap();

    let tag = refs.iter().find(|r| r.name == "refs/tags/v1.0").unwrap();
    let main = refs.iter().find(|r| r.name == "refs/heads/main").unwrap();

    assert!(tag.peeled.is_some(), "annotated tag should have peeled oid");
    assert_eq!(tag.peeled.unwrap(), main.object_id, "peeled should point to the commit");
    assert_ne!(tag.object_id, main.object_id, "tag object differs from commit");
}

#[test]
fn tags_lightweight_tag_has_no_peeled() {
    let store = store_from("make_repo_with_tags.sh");
    let refs = collect_refs(&store).unwrap();

    let tag = refs.iter().find(|r| r.name == "refs/tags/lightweight-tag").unwrap();
    assert!(tag.peeled.is_none(), "lightweight tag should not have peeled oid");
}

// --- multi-branch: branches at different commits ---

#[test]
fn multi_branch_dev_has_different_oid() {
    let store = store_from("make_repo_multi_branch.sh");
    let refs = collect_refs(&store).unwrap();

    let main = refs.iter().find(|r| r.name == "refs/heads/main").unwrap();
    let dev = refs.iter().find(|r| r.name == "refs/heads/dev").unwrap();
    let feature = refs.iter().find(|r| r.name == "refs/heads/feature").unwrap();

    assert_ne!(main.object_id, dev.object_id, "dev has an extra commit");
    assert_eq!(main.object_id, feature.object_id, "feature branched from main");
}

// --- empty repo: no commits, dangling HEAD ---

#[test]
fn empty_repo_returns_no_refs() {
    let store = store_from("make_repo_empty.sh");
    let refs = collect_refs(&store).unwrap();

    assert!(refs.is_empty(), "empty repo should have no advertisable refs");
}

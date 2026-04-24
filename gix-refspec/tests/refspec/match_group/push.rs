/// Push-direction matching tests for [`gix_refspec::MatchGroup::match_push`].
///
/// In push semantics items are **local** references; the spec's LHS (src) is matched against
/// them and the RHS (dst) becomes the remote destination.  This mirrors git's
/// `match_push_refs` / `match_explicit_refs` behaviour from `remote.c`.
use bstr::BString;
use gix_hash::ObjectId;
use gix_refspec::{match_group::Item, parse::Operation, MatchGroup};

// ── helpers ──────────────────────────────────────────────────────────────────

fn oid(hex: &str) -> ObjectId {
    ObjectId::from_hex(hex.as_bytes()).expect("valid fixed-length hex")
}

/// A lightweight stand-in for a local ref used in tests.
struct LocalRef {
    name: BString,
    target: ObjectId,
}

impl LocalRef {
    fn new(name: &str, hex: &str) -> Self {
        Self {
            name: name.into(),
            target: oid(hex),
        }
    }

    fn item(&self) -> Item<'_> {
        Item {
            full_ref_name: self.name.as_ref(),
            target: &self.target,
            object: None,
        }
    }
}

fn parse_push(spec: &str) -> gix_refspec::RefSpec {
    gix_refspec::parse(spec.into(), Operation::Push)
        .expect("valid push spec")
        .to_owned()
}

// ── 1. Simple src:dst ────────────────────────────────────────────────────────

/// `refs/heads/main:refs/heads/main` with local refs {main, dev} produces exactly one mapping
/// (main → main), and dev is not included because the spec is not a glob.
#[test]
fn simple_src_dst_exact_match() {
    let refs = [
        LocalRef::new("refs/heads/main", "1111111111111111111111111111111111111111"),
        LocalRef::new("refs/heads/dev", "2222222222222222222222222222222222222222"),
    ];
    let items: Vec<_> = refs.iter().map(LocalRef::item).collect();

    let spec = parse_push("refs/heads/main:refs/heads/main");
    let group = MatchGroup::from_push_specs([spec.to_ref()]);
    let outcome = group.match_push(items.iter().copied());

    assert_eq!(outcome.mappings.len(), 1, "only main should match");
    let m = &outcome.mappings[0];
    assert_eq!(m.item_index, Some(0), "item_index points to refs/heads/main");
    assert_eq!(m.lhs.to_string(), "refs/heads/main");
    assert_eq!(
        m.rhs.as_deref().expect("rhs set"),
        "refs/heads/main".as_bytes(),
        "remote destination"
    );
}

// ── 2. One-sided spec (src only, no dst) ─────────────────────────────────────

/// `refs/heads/main` with no colon means push to the same remote ref name.
#[test]
fn one_sided_spec_pushes_to_same_name() {
    let refs = [
        LocalRef::new("refs/heads/main", "1111111111111111111111111111111111111111"),
        LocalRef::new("refs/heads/dev", "2222222222222222222222222222222222222222"),
    ];
    let items: Vec<_> = refs.iter().map(LocalRef::item).collect();

    let spec = parse_push("refs/heads/main");
    let group = MatchGroup::from_push_specs([spec.to_ref()]);
    let outcome = group.match_push(items.iter().copied());

    assert_eq!(outcome.mappings.len(), 1, "only main matches the one-sided spec");
    let m = &outcome.mappings[0];
    assert_eq!(m.item_index, Some(0));
    assert_eq!(m.lhs.to_string(), "refs/heads/main");
    // One-sided push specs have no rhs: caller is expected to push to the same remote ref.
    assert!(
        m.rhs.is_none(),
        "no rhs for a one-sided spec; caller uses the lhs as the remote ref"
    );
}

// ── 3. Glob expansion ────────────────────────────────────────────────────────

/// `refs/heads/*:refs/heads/*` with local refs {main, dev, tags/v1} yields two mappings
/// (only refs under refs/heads/ match the glob prefix).
#[test]
fn glob_expansion_includes_only_matching_prefix() {
    let refs = [
        LocalRef::new("refs/heads/main", "1111111111111111111111111111111111111111"),
        LocalRef::new("refs/heads/dev", "2222222222222222222222222222222222222222"),
        LocalRef::new("refs/tags/v1", "3333333333333333333333333333333333333333"),
    ];
    let items: Vec<_> = refs.iter().map(LocalRef::item).collect();

    let spec = parse_push("refs/heads/*:refs/heads/*");
    let group = MatchGroup::from_push_specs([spec.to_ref()]);
    let outcome = group.match_push(items.iter().copied());

    assert_eq!(outcome.mappings.len(), 2, "only refs/heads/* match");
    // mappings are in item iteration order
    assert_eq!(outcome.mappings[0].lhs.to_string(), "refs/heads/main");
    assert_eq!(
        outcome.mappings[0].rhs.as_deref().expect("rhs"),
        "refs/heads/main".as_bytes()
    );
    assert_eq!(outcome.mappings[1].lhs.to_string(), "refs/heads/dev");
    assert_eq!(
        outcome.mappings[1].rhs.as_deref().expect("rhs"),
        "refs/heads/dev".as_bytes()
    );
}

/// Glob maps src pattern to dst pattern with wildcard expansion.
#[test]
fn glob_expansion_with_different_src_and_dst_patterns() {
    let refs = [
        LocalRef::new("refs/heads/main", "1111111111111111111111111111111111111111"),
        LocalRef::new("refs/heads/dev", "2222222222222222222222222222222222222222"),
    ];
    let items: Vec<_> = refs.iter().map(LocalRef::item).collect();

    let spec = parse_push("refs/heads/*:refs/remotes/origin/*");
    let group = MatchGroup::from_push_specs([spec.to_ref()]);
    let outcome = group.match_push(items.iter().copied());

    assert_eq!(outcome.mappings.len(), 2);
    assert_eq!(
        outcome.mappings[0].rhs.as_deref().expect("rhs"),
        "refs/remotes/origin/main".as_bytes()
    );
    assert_eq!(
        outcome.mappings[1].rhs.as_deref().expect("rhs"),
        "refs/remotes/origin/dev".as_bytes()
    );
}

// ── 4. Delete spec ───────────────────────────────────────────────────────────

/// `:refs/heads/gone` — empty src signals "delete this remote ref".
/// The mapping has `item_index = None` (no local source) and `rhs` names the remote ref.
#[test]
fn delete_spec_produces_delete_mapping() {
    let refs = [LocalRef::new(
        "refs/heads/main",
        "1111111111111111111111111111111111111111",
    )];
    let items: Vec<_> = refs.iter().map(LocalRef::item).collect();

    let spec = parse_push(":refs/heads/gone");
    let group = MatchGroup::from_push_specs([spec.to_ref()]);
    let outcome = group.match_push(items.iter().copied());

    assert_eq!(outcome.mappings.len(), 1, "one delete mapping");
    let m = &outcome.mappings[0];
    assert_eq!(m.item_index, None, "delete mappings have no item_index");
    assert_eq!(
        m.rhs.as_deref().expect("rhs is the remote ref to delete"),
        "refs/heads/gone".as_bytes()
    );
}

/// A delete spec alongside a glob spec should emit both a delete mapping and
/// the glob-expanded mappings independently.
#[test]
fn delete_spec_combined_with_glob() {
    let refs = [
        LocalRef::new("refs/heads/main", "1111111111111111111111111111111111111111"),
        LocalRef::new("refs/heads/dev", "2222222222222222222222222222222222222222"),
    ];
    let items: Vec<_> = refs.iter().map(LocalRef::item).collect();

    let delete_spec = parse_push(":refs/heads/gone");
    let glob_spec = parse_push("refs/heads/*:refs/heads/*");
    let group = MatchGroup::from_push_specs([delete_spec.to_ref(), glob_spec.to_ref()]);
    let outcome = group.match_push(items.iter().copied());

    // 1 delete + 2 glob matches
    assert_eq!(outcome.mappings.len(), 3);
    // Delete mapping comes first (processed in spec order before item iteration).
    let delete_m = outcome
        .mappings
        .iter()
        .find(|m| m.item_index.is_none())
        .expect("delete mapping present");
    assert_eq!(delete_m.rhs.as_deref().expect("rhs"), "refs/heads/gone".as_bytes());
}

// ── 5. Validation: dst conflict detection ────────────────────────────────────

/// Two specs mapping different local refs to the same remote destination should produce a
/// validation error.
#[test]
fn validated_errors_on_dst_conflict() {
    let refs = [
        LocalRef::new("refs/heads/main", "1111111111111111111111111111111111111111"),
        LocalRef::new("refs/heads/dev", "2222222222222222222222222222222222222222"),
    ];
    let items: Vec<_> = refs.iter().map(LocalRef::item).collect();

    let spec_a = parse_push("refs/heads/main:refs/heads/conflict");
    let spec_b = parse_push("refs/heads/dev:refs/heads/conflict");
    let group = MatchGroup::from_push_specs([spec_a.to_ref(), spec_b.to_ref()]);
    let outcome = group.match_push(items.iter().copied());

    assert_eq!(outcome.mappings.len(), 2, "two mappings before validation");
    let result = outcome.validated();
    assert!(result.is_err(), "validation must fail on dst conflict");
    let err = result.unwrap_err();
    assert_eq!(err.issues.len(), 1);
    let issue_str = err.issues[0].to_string();
    assert!(
        issue_str.contains("refs/heads/conflict"),
        "error message should name the conflicting dst: {issue_str}"
    );
}

/// Two specs targeting distinct remote refs should pass validation without issues.
#[test]
fn validated_ok_when_no_dst_conflict() {
    let refs = [
        LocalRef::new("refs/heads/main", "1111111111111111111111111111111111111111"),
        LocalRef::new("refs/heads/dev", "2222222222222222222222222222222222222222"),
    ];
    let items: Vec<_> = refs.iter().map(LocalRef::item).collect();

    let spec_a = parse_push("refs/heads/main:refs/heads/main");
    let spec_b = parse_push("refs/heads/dev:refs/heads/dev");
    let group = MatchGroup::from_push_specs([spec_a.to_ref(), spec_b.to_ref()]);
    let outcome = group.match_push(items.iter().copied());

    let (validated, fixes) = outcome.validated().expect("no conflicts");
    assert_eq!(validated.mappings.len(), 2);
    assert!(fixes.is_empty(), "no fixes expected for push validation");
}

// ── 6. No-match (spec doesn't match any local ref) ───────────────────────────

/// A spec whose src names a ref that doesn't exist locally produces zero mappings.
#[test]
fn nonexistent_local_ref_produces_no_mapping() {
    let refs = [LocalRef::new(
        "refs/heads/dev",
        "2222222222222222222222222222222222222222",
    )];
    let items: Vec<_> = refs.iter().map(LocalRef::item).collect();

    let spec = parse_push("refs/heads/main:refs/heads/main");
    let group = MatchGroup::from_push_specs([spec.to_ref()]);
    let outcome = group.match_push(items.iter().copied());

    assert_eq!(outcome.mappings.len(), 0, "no local ref matches; no mappings emitted");
}

// ── 7. Force-flag passthrough (+ prefix) ─────────────────────────────────────

/// A force spec (`+refs/heads/main:refs/heads/main`) is parsed and matched the same way.
/// The force flag is carried on the spec; mapping itself is structurally identical.
#[test]
fn force_spec_matches_same_as_normal() {
    let refs = [LocalRef::new(
        "refs/heads/main",
        "1111111111111111111111111111111111111111",
    )];
    let items: Vec<_> = refs.iter().map(LocalRef::item).collect();

    let spec = parse_push("+refs/heads/main:refs/heads/main");
    let group = MatchGroup::from_push_specs([spec.to_ref()]);
    let outcome = group.match_push(items.iter().copied());

    assert_eq!(outcome.mappings.len(), 1);
    let m = &outcome.mappings[0];
    assert_eq!(m.lhs.to_string(), "refs/heads/main");
    assert_eq!(m.rhs.as_deref().expect("rhs"), "refs/heads/main".as_bytes());
}

// ── 8. Deduplication ─────────────────────────────────────────────────────────

/// The same mapping produced by two overlapping specs (e.g., glob + explicit) appears only once.
#[test]
fn duplicate_mappings_are_deduplicated() {
    let refs = [LocalRef::new(
        "refs/heads/main",
        "1111111111111111111111111111111111111111",
    )];
    let items: Vec<_> = refs.iter().map(LocalRef::item).collect();

    // glob covers main, explicit also maps main → main; dedup should collapse them.
    let spec_glob = parse_push("refs/heads/*:refs/heads/*");
    let spec_exact = parse_push("refs/heads/main:refs/heads/main");
    let group = MatchGroup::from_push_specs([spec_glob.to_ref(), spec_exact.to_ref()]);
    let outcome = group.match_push(items.iter().copied());

    assert_eq!(outcome.mappings.len(), 1, "deduplicated to a single mapping");
}

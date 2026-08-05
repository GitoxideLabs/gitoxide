use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    path::Path,
};

use bstr::ByteSlice;
use gix_object::{FindExt, Write, tree::EntryMode};

use super::{basic_merge_options, new_blob_merge_platform, new_diff_resource_cache};

type Tree = BTreeMap<String, (EntryMode, gix_hash::ObjectId)>;

#[derive(Debug)]
struct Case {
    left_operation: String,
    right_operation: String,
    left_commit: gix_hash::ObjectId,
    right_commit: gix_hash::ObjectId,
    forward_file: String,
    git_forward_conflicted: bool,
    reverse_file: String,
    git_reverse_conflicted: bool,
}

#[derive(Debug)]
struct Result {
    tree_id: gix_hash::ObjectId,
    tree: Tree,
    conflicted: bool,
    conflicted_with_forced_resolution: bool,
}

/// Record the current behavior of merge-ORT and gix for a finite Cartesian model of tree changes.
///
/// See `tree-cartesian-baseline.sh` for the model and `cartesian-baseline.txt` for the metrics and
/// all observed differences. Set `GIX_MERGE_UPDATE_CARTESIAN_BASELINE=1` to accept a new status quo.
#[test]
fn records_status_quo_sha1() -> crate::Result {
    if gix_testtools::object_hash() != gix_hash::Kind::Sha1 {
        return Ok(());
    }

    let root = gix_testtools::scripted_fixture_read_only("tree-cartesian-baseline.sh")?;
    let repo = root.join("matrix");
    let cases = std::fs::read_to_string(repo.join("cases.tsv"))?
        .lines()
        .map(parse_case)
        .collect::<Vec<_>>();
    let operations = cases
        .iter()
        .flat_map(|case| [&case.left_operation, &case.right_operation])
        .collect::<BTreeSet<_>>();
    assert_eq!(operations.len(), 14, "the fixture documents fourteen atomic states");
    assert_eq!(
        cases.len(),
        operations.len() * (operations.len() + 1) / 2,
        "the fixture must contain the unordered Cartesian product; each case is merged both ways"
    );

    let odb = gix_odb::at_opts(
        repo.join(".git/objects"),
        Vec::new(),
        gix_odb::store::init::Options {
            object_hash: gix_hash::Kind::Sha1,
            ..Default::default()
        },
    )?;
    let objects = gix_odb::memory::Proxy::new(odb, gix_hash::Kind::Sha1);
    let base_tree_id =
        gix_hash::ObjectId::from_hex(std::fs::read_to_string(repo.join(".git/base.tree"))?.trim().as_bytes())?;
    let base_tree = flatten_tree(base_tree_id, &objects)?;
    let mut graph = gix_revwalk::Graph::new(&objects, None);
    let mut diff_resource_cache = new_diff_resource_cache(&repo);
    let mut blob_merge = new_blob_merge_platform(&repo, 100);
    let options = basic_merge_options();
    let mut force_ours_options = options.clone();
    force_ours_options.tree_merge.tree_conflicts = Some(gix_merge::tree::ResolveWith::Ours);
    force_ours_options.tree_merge.blob_merge.text.conflict =
        gix_merge::blob::builtin_driver::text::Conflict::ResolveWithOurs;
    let git_kind = gix_merge::tree::TreatAsUnresolved::git();
    let forced_resolution = gix_merge::tree::TreatAsUnresolved::forced_resolution();
    let mut commit_trees = BTreeMap::<gix_hash::ObjectId, Tree>::new();

    let mut exact_agreement = 0;
    let mut shape_agreement = 0;
    let mut conflict_agreement = 0;
    let mut git_inverse_exact = Vec::new();
    let mut gix_inverse_exact = Vec::new();
    let mut git_inverse_shape = Vec::new();
    let mut gix_inverse_shape = Vec::new();
    let mut git_inverse_conflict = Vec::new();
    let mut gix_inverse_conflict = Vec::new();
    let mut git_inverse_payload = Vec::new();
    let mut gix_inverse_payload = Vec::new();
    let mut implementation_differences = Vec::new();
    let mut git_payload_losses = Vec::new();
    let mut gix_payload_losses = Vec::new();
    let mut payload_checks = 0;
    let mut git_payloads_retained = 0;
    let mut gix_payloads_retained = 0;
    let mut payload_agreement = 0;
    let mut git_conflicted = 0;
    let mut gix_conflicted = 0;
    let mut oracle_cases = 0;
    let mut git_oracle_passes = 0;
    let mut gix_oracle_passes = 0;
    let mut git_oracle_failures = Vec::new();
    let mut gix_oracle_failures = Vec::new();
    let mut ours_unresolved = Vec::new();
    let mut ours_changed_clean_merge = Vec::new();
    let mut ours_lost_current_payload = Vec::new();
    let mut ours_forgot_conflict = Vec::new();
    let mut ours_current_payload_checks = 0;
    let mut ours_clean_directional_merges = 0;
    let mut ours_conflict_provenance_checks = 0;
    let mut ours_oracle_passes = 0;
    let mut ours_oracle_failures = Vec::new();

    for case in &cases {
        let case_name = format!("{} + {}", case.left_operation, case.right_operation);
        let left_tree = commit_trees
            .entry(case.left_commit)
            .or_insert(flatten_commit(case.left_commit, &objects)?)
            .clone();
        let right_tree = commit_trees
            .entry(case.right_commit)
            .or_insert(flatten_commit(case.right_commit, &objects)?)
            .clone();

        let git_forward = git_result(&repo, &case.forward_file, case.git_forward_conflicted, &objects)?;
        let git_reverse = git_result(&repo, &case.reverse_file, case.git_reverse_conflicted, &objects)?;
        let mut run_gix = |ours,
                           theirs,
                           ours_label: &str,
                           theirs_label: &str,
                           options: &gix_merge::commit::Options|
         -> crate::Result<Result> {
            let mut outcome = gix_merge::commit(
                ours,
                theirs,
                gix_merge::blob::builtin_driver::text::Labels {
                    ancestor: Some("BASE".into()),
                    current: Some(ours_label.into()),
                    other: Some(theirs_label.into()),
                },
                &mut graph,
                &mut diff_resource_cache,
                &mut blob_merge,
                &objects,
                &mut |id| id.to_hex_with_len(7).to_string(),
                options.clone(),
            )
            .map_err(|err| err.into_error())?
            .tree_merge;
            let conflicted = outcome.has_unresolved_conflicts(git_kind);
            let conflicted_with_forced_resolution = outcome.has_unresolved_conflicts(forced_resolution);
            let tree_id = outcome.tree.write(|tree| objects.write(tree))?;
            Ok(Result {
                tree_id,
                tree: flatten_tree(tree_id, &objects)?,
                conflicted,
                conflicted_with_forced_resolution,
            })
        };
        let gix_forward = run_gix(
            case.left_commit,
            case.right_commit,
            &format!("A-{}", case.left_operation),
            &format!("B-{}", case.right_operation),
            &options,
        )?;
        let gix_reverse = run_gix(
            case.right_commit,
            case.left_commit,
            &format!("B-{}", case.right_operation),
            &format!("A-{}", case.left_operation),
            &options,
        )?;
        let ours_forward = run_gix(
            case.left_commit,
            case.right_commit,
            &format!("A-{}", case.left_operation),
            &format!("B-{}", case.right_operation),
            &force_ours_options,
        )?;
        let ours_reverse = run_gix(
            case.right_commit,
            case.left_commit,
            &format!("B-{}", case.right_operation),
            &format!("A-{}", case.left_operation),
            &force_ours_options,
        )?;

        let mut difference = Vec::new();
        let mut git_payload_presence = [Vec::new(), Vec::new()];
        let mut gix_payload_presence = [Vec::new(), Vec::new()];
        for (direction_idx, (direction, git, gix)) in
            [("A+B", &git_forward, &gix_forward), ("B+A", &git_reverse, &gix_reverse)]
                .into_iter()
                .enumerate()
        {
            if git.tree_id == gix.tree_id {
                exact_agreement += 1;
            } else {
                difference.push(format!("{direction}:tree"));
            }
            if same_shape(&git.tree, &gix.tree) {
                shape_agreement += 1;
            } else {
                difference.push(format!("{direction}:shape"));
            }
            if git.conflicted == gix.conflicted {
                conflict_agreement += 1;
            } else {
                difference.push(format!("{direction}:conflict"));
            }
            git_conflicted += usize::from(git.conflicted);
            gix_conflicted += usize::from(gix.conflicted);

            let mut direction_payload_differs = false;
            for (side, operation) in [
                ("A", case.left_operation.as_str()),
                ("B", case.right_operation.as_str()),
            ] {
                let Some(payload) = payload(side, operation) else {
                    continue;
                };
                payload_checks += 1;
                let git_has_payload = contains(&git.tree, payload.as_bytes(), &objects)?;
                let gix_has_payload = contains(&gix.tree, payload.as_bytes(), &objects)?;
                git_payload_presence[direction_idx].push(git_has_payload);
                gix_payload_presence[direction_idx].push(gix_has_payload);
                if git_has_payload {
                    git_payloads_retained += 1;
                } else {
                    git_payload_losses.push(format!("{case_name} {direction}: {payload}"));
                }
                if gix_has_payload {
                    gix_payloads_retained += 1;
                } else {
                    gix_payload_losses.push(format!("{case_name} {direction}: {payload}"));
                }
                if git_has_payload == gix_has_payload {
                    payload_agreement += 1;
                } else {
                    direction_payload_differs = true;
                }
            }
            if direction_payload_differs {
                difference.push(format!("{direction}:payload"));
            }
        }
        if !difference.is_empty() {
            implementation_differences.push(format!("{case_name}: {}", difference.join(", ")));
        }

        if git_forward.tree_id != git_reverse.tree_id {
            git_inverse_exact.push(case_name.clone());
        }
        if gix_forward.tree_id != gix_reverse.tree_id {
            gix_inverse_exact.push(case_name.clone());
        }
        if !same_shape(&git_forward.tree, &git_reverse.tree) {
            git_inverse_shape.push(case_name.clone());
        }
        if !same_shape(&gix_forward.tree, &gix_reverse.tree) {
            gix_inverse_shape.push(case_name.clone());
        }
        if git_forward.conflicted != git_reverse.conflicted {
            git_inverse_conflict.push(case_name.clone());
        }
        if gix_forward.conflicted != gix_reverse.conflicted {
            gix_inverse_conflict.push(case_name.clone());
        }
        if git_payload_presence[0] != git_payload_presence[1] {
            git_inverse_payload.push(case_name.clone());
        }
        if gix_payload_presence[0] != gix_payload_presence[1] {
            gix_inverse_payload.push(case_name.clone());
        }

        for (direction, current_side, operation, normal, ours) in [
            ("A+B", "A", case.left_operation.as_str(), &gix_forward, &ours_forward),
            ("B+A", "B", case.right_operation.as_str(), &gix_reverse, &ours_reverse),
        ] {
            if ours.conflicted {
                ours_unresolved.push(format!("{case_name} {direction}"));
            }
            if !normal.conflicted {
                ours_clean_directional_merges += 1;
                if ours.tree_id != normal.tree_id {
                    ours_changed_clean_merge.push(format!("{case_name} {direction}"));
                }
            } else {
                ours_conflict_provenance_checks += 1;
                if !ours.conflicted_with_forced_resolution {
                    ours_forgot_conflict.push(format!("{case_name} {direction}"));
                }
            }
            if let Some(payload) = payload(current_side, operation) {
                ours_current_payload_checks += 1;
                if !contains(&ours.tree, payload.as_bytes(), &objects)? {
                    ours_lost_current_payload.push(format!("{case_name} {direction}: {payload}"));
                }
            }
        }

        if let Some(ideal) = unambiguous_merge(&base_tree, &left_tree, &right_tree) {
            oracle_cases += 1;
            let git_ok = [&git_forward, &git_reverse]
                .into_iter()
                .all(|result| !result.conflicted && result.tree == ideal);
            let gix_ok = [&gix_forward, &gix_reverse]
                .into_iter()
                .all(|result| !result.conflicted && result.tree == ideal);
            if git_ok {
                git_oracle_passes += 1;
            } else {
                git_oracle_failures.push(case_name.clone());
            }
            if gix_ok {
                gix_oracle_passes += 1;
            } else {
                gix_oracle_failures.push(case_name.clone());
            }
            if [&ours_forward, &ours_reverse]
                .into_iter()
                .all(|result| !result.conflicted && result.tree == ideal)
            {
                ours_oracle_passes += 1;
            } else {
                ours_oracle_failures.push(case_name);
            }
        }
    }

    assert!(
        gix_inverse_payload.is_empty(),
        "reversing the merge must not change which side payloads survive: {gix_inverse_payload:#?}"
    );
    assert!(
        ours_unresolved.is_empty(),
        "the combined tree/content ours policy must resolve every modeled conflict: {ours_unresolved:#?}"
    );
    assert!(
        ours_changed_clean_merge.is_empty(),
        "the ours policy must not affect merges which need no resolution: {ours_changed_clean_merge:#?}"
    );
    assert!(
        ours_lost_current_payload.is_empty(),
        "the ours policy must retain unique current-side content: {ours_lost_current_payload:#?}"
    );
    assert!(
        ours_forgot_conflict.is_empty(),
        "forced resolutions must remain visible to the strict conflict policy: {ours_forgot_conflict:#?}"
    );

    let directional_merges = cases.len() * 2;
    let mut report = String::new();
    writeln!(report, "# Cartesian tree-merge status quo")?;
    writeln!(report)?;
    writeln!(
        report,
        "Model: 14 atomic states, {} unordered pairs, {directional_merges} directional merges.",
        cases.len()
    )?;
    writeln!(
        report,
        "States: {}.",
        operations
            .iter()
            .map(|operation| operation.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    )?;
    writeln!(
        report,
        "Assumptions: one merge base; regular non-executable text files; exact renames; one operation per side;"
    )?;
    writeln!(
        report,
        "no attributes, copies, symlinks, submodules, recursive merge bases, or rename-similarity ambiguity."
    )?;
    writeln!(
        report,
        "The clean oracle covers identical side trees and path-disjoint deltas, including file/directory prefixes."
    )?;
    writeln!(
        report,
        "Payload retention checks whether unique changed content survives somewhere in conflicted or clean output."
    )?;
    writeln!(
        report,
        "Git results come from `git merge-tree --write-tree`, which uses merge-ORT."
    )?;
    writeln!(
        report,
        "The gix ours policy combines tree `ResolveWith::Ours` with text `ResolveWithOurs`, as the manual baseline does."
    )?;
    writeln!(
        report,
        "Git's `-Xours` is not equivalent: it favors content and symlink conflicts but does not force all tree conflicts."
    )?;
    writeln!(
        report,
        "The ours policy is directional, so both directions are checked independently rather than required to be identical."
    )?;
    writeln!(
        report,
        "Exact-tree inverse differences include directional conflict-marker ordering; shape, conflict, and payload"
    )?;
    writeln!(
        report,
        "symmetry distinguish those presentation differences from structural or content loss."
    )?;
    writeln!(report)?;
    writeln!(
        report,
        "Git/gix exact tree agreement: {exact_agreement}/{directional_merges}"
    )?;
    writeln!(
        report,
        "Git/gix path-and-mode agreement: {shape_agreement}/{directional_merges}"
    )?;
    writeln!(
        report,
        "Git/gix unresolved-conflict agreement: {conflict_agreement}/{directional_merges}"
    )?;
    writeln!(
        report,
        "Git/gix unique-payload agreement: {payload_agreement}/{payload_checks}"
    )?;
    writeln!(
        report,
        "Git unresolved directional merges: {git_conflicted}/{directional_merges}"
    )?;
    writeln!(
        report,
        "gix unresolved directional merges: {gix_conflicted}/{directional_merges}"
    )?;
    writeln!(
        report,
        "Git inverse exact-tree symmetry: {}/{}",
        cases.len() - git_inverse_exact.len(),
        cases.len()
    )?;
    writeln!(
        report,
        "gix inverse exact-tree symmetry: {}/{}",
        cases.len() - gix_inverse_exact.len(),
        cases.len()
    )?;
    writeln!(
        report,
        "Git inverse path-and-mode symmetry: {}/{}",
        cases.len() - git_inverse_shape.len(),
        cases.len()
    )?;
    writeln!(
        report,
        "gix inverse path-and-mode symmetry: {}/{}",
        cases.len() - gix_inverse_shape.len(),
        cases.len()
    )?;
    writeln!(
        report,
        "Git inverse conflict symmetry: {}/{}",
        cases.len() - git_inverse_conflict.len(),
        cases.len()
    )?;
    writeln!(
        report,
        "gix inverse conflict symmetry: {}/{}",
        cases.len() - gix_inverse_conflict.len(),
        cases.len()
    )?;
    writeln!(
        report,
        "Git inverse payload symmetry: {}/{}",
        cases.len() - git_inverse_payload.len(),
        cases.len()
    )?;
    writeln!(
        report,
        "gix inverse payload symmetry: {}/{}",
        cases.len() - gix_inverse_payload.len(),
        cases.len()
    )?;
    writeln!(
        report,
        "Git unique-payload retention: {git_payloads_retained}/{payload_checks}"
    )?;
    writeln!(
        report,
        "gix unique-payload retention: {gix_payloads_retained}/{payload_checks}"
    )?;
    writeln!(report, "Unambiguous clean-oracle pairs: {oracle_cases}/{}", cases.len())?;
    writeln!(report, "Git clean-oracle passes: {git_oracle_passes}/{oracle_cases}")?;
    writeln!(report, "gix clean-oracle passes: {gix_oracle_passes}/{oracle_cases}")?;
    writeln!(
        report,
        "gix ours unresolved directional merges: {}/{directional_merges}",
        ours_unresolved.len()
    )?;
    writeln!(
        report,
        "gix ours unchanged default-clean merges: {}/{}",
        ours_clean_directional_merges - ours_changed_clean_merge.len(),
        ours_clean_directional_merges
    )?;
    writeln!(
        report,
        "gix ours current-side payload retention: {}/{}",
        ours_current_payload_checks - ours_lost_current_payload.len(),
        ours_current_payload_checks
    )?;
    writeln!(
        report,
        "gix ours conflict provenance retained: {}/{}",
        ours_conflict_provenance_checks - ours_forgot_conflict.len(),
        ours_conflict_provenance_checks
    )?;
    writeln!(
        report,
        "gix ours clean-oracle passes: {ours_oracle_passes}/{oracle_cases}"
    )?;

    write_list(&mut report, "Git/gix differences", &implementation_differences)?;
    write_list(&mut report, "Git inverse exact-tree differences", &git_inverse_exact)?;
    write_list(&mut report, "gix inverse exact-tree differences", &gix_inverse_exact)?;
    write_list(&mut report, "Git inverse path-and-mode differences", &git_inverse_shape)?;
    write_list(&mut report, "gix inverse path-and-mode differences", &gix_inverse_shape)?;
    write_list(&mut report, "Git inverse conflict differences", &git_inverse_conflict)?;
    write_list(&mut report, "gix inverse conflict differences", &gix_inverse_conflict)?;
    write_list(&mut report, "Git inverse payload differences", &git_inverse_payload)?;
    write_list(&mut report, "gix inverse payload differences", &gix_inverse_payload)?;
    write_list(&mut report, "Git clean-oracle failures", &git_oracle_failures)?;
    write_list(&mut report, "gix clean-oracle failures", &gix_oracle_failures)?;
    write_list(&mut report, "Git payload losses", &git_payload_losses)?;
    write_list(&mut report, "gix payload losses", &gix_payload_losses)?;
    write_list(&mut report, "gix ours unresolved merges", &ours_unresolved)?;
    write_list(
        &mut report,
        "gix ours changes to default-clean merges",
        &ours_changed_clean_merge,
    )?;
    write_list(
        &mut report,
        "gix ours current-side payload losses",
        &ours_lost_current_payload,
    )?;
    write_list(
        &mut report,
        "gix ours forgotten conflict provenance",
        &ours_forgot_conflict,
    )?;
    write_list(&mut report, "gix ours clean-oracle failures", &ours_oracle_failures)?;

    let baseline = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/merge/tree/cartesian-baseline.txt");
    if std::env::var_os("GIX_MERGE_UPDATE_CARTESIAN_BASELINE").is_some() {
        std::fs::write(&baseline, report)?;
        return Ok(());
    }
    let expected = std::fs::read_to_string(&baseline)?.replace("\r\n", "\n");
    pretty_assertions::assert_str_eq!(
        report,
        expected,
        "Cartesian merge behavior changed. Review the report, then set \
         GIX_MERGE_UPDATE_CARTESIAN_BASELINE=1 to accept the new status quo."
    );
    Ok(())
}

fn parse_case(line: &str) -> Case {
    let fields = line.split('\t').collect::<Vec<_>>();
    assert_eq!(fields.len(), 8, "invalid Cartesian case: {line:?}");
    Case {
        left_operation: fields[0].into(),
        right_operation: fields[1].into(),
        left_commit: gix_hash::ObjectId::from_hex(fields[2].as_bytes()).expect("SHA-1 commit"),
        right_commit: gix_hash::ObjectId::from_hex(fields[3].as_bytes()).expect("SHA-1 commit"),
        forward_file: fields[4].into(),
        git_forward_conflicted: fields[5] == "conflicted",
        reverse_file: fields[6].into(),
        git_reverse_conflicted: fields[7] == "conflicted",
    }
}

fn git_result(
    repo: &Path,
    filename: &str,
    conflicted: bool,
    objects: &gix_odb::memory::Proxy<gix_odb::Handle>,
) -> crate::Result<Result> {
    let data = std::fs::read(repo.join(".git").join(filename))?;
    let hex = data
        .split(|byte| *byte == 0)
        .next()
        .expect("merge-tree always writes a tree");
    let tree_id = gix_hash::ObjectId::from_hex(hex)?;
    Ok(Result {
        tree_id,
        tree: flatten_tree(tree_id, objects)?,
        conflicted,
        conflicted_with_forced_resolution: conflicted,
    })
}

fn flatten_commit(id: gix_hash::ObjectId, objects: &gix_odb::memory::Proxy<gix_odb::Handle>) -> crate::Result<Tree> {
    let mut buf = Vec::new();
    let tree_id = objects.find_commit(&id, &mut buf)?.tree();
    flatten_tree(tree_id, objects)
}

fn flatten_tree(id: gix_hash::ObjectId, objects: &gix_odb::memory::Proxy<gix_odb::Handle>) -> crate::Result<Tree> {
    fn recurse(
        id: gix_hash::ObjectId,
        prefix: &str,
        objects: &gix_odb::memory::Proxy<gix_odb::Handle>,
        out: &mut Tree,
    ) -> crate::Result {
        let mut buf = Vec::new();
        let entries = objects
            .find_tree(&id, &mut buf)?
            .entries
            .iter()
            .map(|entry| {
                (
                    entry.filename.to_str_lossy().into_owned(),
                    entry.mode,
                    entry.oid.to_owned(),
                )
            })
            .collect::<Vec<_>>();
        for (name, mode, id) in entries {
            let path = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            if mode.is_tree() {
                recurse(id, &path, objects, out)?;
            } else {
                out.insert(path, (mode, id));
            }
        }
        Ok(())
    }

    let mut out = Tree::new();
    recurse(id, "", objects, &mut out)?;
    Ok(out)
}

fn same_shape(left: &Tree, right: &Tree) -> bool {
    left.iter()
        .map(|(path, (mode, _))| (path, mode))
        .eq(right.iter().map(|(path, (mode, _))| (path, mode)))
}

/// Return the unique content marker written by `operation` for `side`, if it writes one.
///
/// `tree-cartesian-baseline.sh` embeds `payload-{side}-{operation}` into every atomic
/// state that adds or modifies content. Including the side makes the marker unique even
/// when A and B perform the same operation. States that only keep, delete, or move existing
/// content return `None` because they introduce no new bytes whose retention can be checked.
///
/// The test searches for each returned marker in every blob of the merged tree, including
/// blobs containing conflict markers. This detects whether either implementation silently
/// loses a side's unique contribution and whether reversing the merge changes which
/// contributions survive. It deliberately does not assert the marker's path, multiplicity,
/// surrounding content, or whether the merge was clean; those properties are covered by
/// the tree, shape, conflict, and clean-oracle metrics.
fn payload(side: &str, operation: &str) -> Option<String> {
    matches!(
        operation,
        "modify-source"
            | "rename-modify-free-a"
            | "add-free-a"
            | "add-free-b"
            | "modify-occupied"
            | "replace-modify-occupied"
            | "file-to-dir"
            | "rename-and-file-to-dir"
    )
    .then(|| format!("payload-{side}-{operation}"))
}

fn contains(tree: &Tree, needle: &[u8], objects: &gix_odb::memory::Proxy<gix_odb::Handle>) -> crate::Result<bool> {
    let mut buf = Vec::new();
    for (_, id) in tree.values() {
        let blob = objects.find_blob(id, &mut buf)?;
        if blob.data.find(needle).is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Return the exact clean merge result when this finite model makes it unambiguous.
///
/// This is a deliberately conservative oracle, independent of both Git and gix. It can
/// prove a result in two situations:
///
/// * both sides produced the same complete tree, in which case that tree is the result;
/// * each side changed paths disjoint from the other side's changes, in which case their
///   additions, modifications, and deletions commute and can be applied to the base in
///   either order.
///
/// Path overlap includes equality and ancestor/descendant relationships, so `file` and
/// `file/child` are not considered independent. If any changed paths overlap, this returns
/// `None`: content merges, rename identity, delete/modify decisions, and file/directory
/// conflicts require semantics this simple oracle intentionally does not model. `None`
/// therefore means "no judgement", not "the merge should conflict".
///
/// The caller requires both merge directions to be conflict-free and exactly equal to the
/// returned tree. This checks the implementations against an independently derived ideal
/// for the easy cases without pretending to define the best result for ambiguous ones.
fn unambiguous_merge(base: &Tree, left: &Tree, right: &Tree) -> Option<Tree> {
    if left == right {
        return Some(left.clone());
    }
    let left_changes = changed_paths(base, left);
    let right_changes = changed_paths(base, right);
    if left_changes
        .iter()
        .any(|left| right_changes.iter().any(|right| paths_overlap(left, right)))
    {
        return None;
    }

    let mut out = base.clone();
    for (changes, side) in [(&left_changes, left), (&right_changes, right)] {
        for path in changes {
            match side.get(path) {
                Some(entry) => {
                    out.insert(path.clone(), *entry);
                }
                None => {
                    out.remove(path);
                }
            }
        }
    }
    Some(out)
}

fn changed_paths(base: &Tree, side: &Tree) -> BTreeSet<String> {
    base.keys()
        .chain(side.keys())
        .filter(|path| base.get(*path) != side.get(*path))
        .cloned()
        .collect()
}

/// Return whether two changed paths address the same tree slot or one is below the other.
///
/// Ancestor/descendant paths overlap because applying one change may determine whether the
/// other path can exist at all, as in a file/directory conflict. The prefix must end at a
/// path-component boundary; a shared byte prefix alone is not enough.
///
/// Examples:
///
/// * `a` and `a` overlap because they are the same path.
/// * `a` and `a/b`, as well as `a/b` and `a`, overlap because one is an ancestor.
/// * `a/b` and `a/c` do not overlap because they are siblings.
/// * `a` and `ab` do not overlap because `a` is not a complete component of `ab`.
fn paths_overlap(left: &str, right: &str) -> bool {
    left == right
        || left.strip_prefix(right).is_some_and(|suffix| suffix.starts_with('/'))
        || right.strip_prefix(left).is_some_and(|suffix| suffix.starts_with('/'))
}

fn write_list(out: &mut String, title: &str, entries: &[String]) -> std::fmt::Result {
    writeln!(out)?;
    writeln!(out, "## {title} ({})", entries.len())?;
    if entries.is_empty() {
        writeln!(out, "- none")
    } else {
        for entry in entries {
            writeln!(out, "- {entry}")?;
        }
        Ok(())
    }
}

//! Path-only metadata and coverage invalidations for normalized paths; performs no I/O.
//! Results pair invalidations with a repository/watch-inventory refresh hint.

use std::path::Path;

use super::{Changes, DependencyKind, Layout, Scope, same_path, strip_prefix, worktrees};

pub(super) fn classify_dependencies(path: &Path, layout: &Layout, ignore_case: bool) -> (Changes, bool) {
    let equal = |a: &Path, b: &Path| same_path(a, b, ignore_case);
    let beneath = |a: &Path, b: &Path| strip_prefix(a, b, ignore_case).is_some();
    let mut changes = Changes::default();
    for dependency in &layout.dependencies {
        if beneath(&dependency.path, path) {
            match dependency.kind {
                DependencyKind::Configuration => changes.configuration = true,
                DependencyKind::Ignore => changes.ignores = true,
                DependencyKind::Attributes => {}
            }
        }
    }
    for directory in [&layout.git_dir, &layout.common_dir] {
        if beneath(directory, path) {
            changes.layout = true;
        }
        // HEAD selects branch-conditioned configuration includes, not just reference views.
        for name in ["config", "config.worktree", "HEAD"] {
            changes.configuration |= equal(path, &directory.join(name));
        }
        // HEAD lives in the current stack; shared symbolic refs may also affect branch includes.
        // Refresh both conservatively without resolving refs.
        changes.configuration |= beneath(path, &directory.join("reftable"));
        for name in ["commondir", "gitdir"] {
            changes.layout |= equal(path, &directory.join(name));
        }
    }
    if layout
        .worktree
        .as_ref()
        .is_some_and(|root| equal(path, root) || equal(path, &root.join(".git")))
    {
        changes.layout = true;
    }
    let refresh = changes.configuration || changes.ignores || changes.layout;
    if refresh {
        changes.worktree = Some(Scope::All);
    }
    (changes, refresh)
}

/// Ownership of metadata invalidations relative to the selected checkout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MetadataScope {
    /// Selected checkout, including coarse common-metadata and foreign-worktree hints.
    SelectedOnly,
    /// Selected checkout and shared metadata in an all-worktree monitor.
    Primary,
    /// Additional checkout, excluding separately identifiable shared refs.
    Other,
}

pub(super) fn classify_metadata(path: &Path, layout: &Layout, ignore_case: bool) -> (Changes, bool) {
    classify_metadata_scoped(path, layout, ignore_case, MetadataScope::SelectedOnly)
}

pub(super) fn classify_metadata_scoped(
    path: &Path,
    layout: &Layout,
    ignore_case: bool,
    scope: MetadataScope,
) -> (Changes, bool) {
    let equal = |a: &Path, b: &Path| same_path(a, b, ignore_case);
    let beneath = |a: &Path, b: &Path| strip_prefix(a, b, ignore_case).is_some();
    let mut changes = Changes::default();
    let mut refresh = false;
    for dependency in &layout.dependencies {
        if beneath(&dependency.path, path) {
            match dependency.kind {
                DependencyKind::Configuration => changes.configuration = true,
                DependencyKind::Ignore => changes.ignores = true,
                DependencyKind::Attributes => changes.attributes = true,
            }
            refresh = true;
        }
    }
    if equal(path, &layout.index)
        || equal(path, &layout.git_dir.join("index.lock"))
        || equal(path, &layout.common_dir.join("index"))
        || equal(path, &layout.common_dir.join("index.lock"))
    {
        return (changes, refresh);
    }
    let worktrees = layout.common_dir.join("worktrees");
    if let Some(relative) = strip_prefix(path, &worktrees, ignore_case) {
        if beneath(path, &layout.git_dir) && layout.git_dir != layout.common_dir {
            // The current worktree's metadata is classified below.
        } else {
            if scope == MetadataScope::Other {
                return (changes, refresh);
            }
            let components: Vec<_> = relative.components().collect();
            if components.len() <= 1 {
                changes.layout = true;
                changes.references = true;
                refresh = true;
            } else if components.len() == 2
                && (equal(Path::new(components[1].as_os_str()), Path::new("HEAD"))
                    || equal(Path::new(components[1].as_os_str()), Path::new("gitdir")))
            {
                changes.references = true;
                changes.layout = true;
            }
            if components
                .get(1)
                .is_some_and(|component| equal(Path::new(component.as_os_str()), Path::new("reftable")))
            {
                // The foreign stack may contain HEAD, just like the loose file above.
                changes.references = true;
                changes.layout = true;
                refresh |= components.len() == 2;
            }
            return (changes, refresh);
        }
    }
    for directory in [&layout.git_dir, &layout.common_dir] {
        if beneath(path, &directory.join("reftable")) {
            // The common stack combines shared refs with main-private HEAD and operation refs.
            // Handle it before the binary private/shared filter: making it merely private would
            // hide shared refs from a linked primary, while making it shared would hide main HEAD.
            if directory == &layout.git_dir || scope == MetadataScope::SelectedOnly {
                changes.references = true;
                changes.operations = true;
                changes.configuration = true;
                refresh = true;
            } else if scope == MetadataScope::Primary {
                changes.references = true;
                refresh |= equal(path, &directory.join("reftable"));
            }
            continue;
        }
        if scope != MetadataScope::SelectedOnly && directory == &layout.common_dir {
            // The common directory also contains the main checkout's private HEAD and operation state.
            let private = worktrees::is_private_metadata(path, directory, ignore_case);
            if directory != &layout.git_dir && private || scope == MetadataScope::Other && !private {
                continue;
            }
        }
        if let Some(relative) = strip_prefix(path, directory, ignore_case) {
            let count = relative.components().count();
            if count == 0 {
                changes.merge(Changes::metadata_all());
                refresh = true;
            } else if beneath(relative, Path::new("refs")) {
                changes.references = true;
                refresh |= count == 1;
            } else if beneath(relative, Path::new("rebase-apply"))
                || beneath(relative, Path::new("rebase-merge"))
                || beneath(relative, Path::new("sequencer"))
            {
                changes.operations = true;
                refresh |= count == 1;
            } else if count == 1 {
                let name = relative.as_os_str().as_encoded_bytes();
                let known: &[&[u8]] = &[
                    b"config",
                    b"config.worktree",
                    b"commondir",
                    b"gitdir",
                    b"HEAD",
                    b"ORIG_HEAD",
                    b"FETCH_HEAD",
                    b"packed-refs",
                    b"shallow",
                    b"MERGE_HEAD",
                    b"REBASE_HEAD",
                    b"CHERRY_PICK_HEAD",
                    b"REVERT_HEAD",
                    b"AUTO_MERGE",
                    b"MERGE_MSG",
                    b"MERGE_MODE",
                    b"SQUASH_MSG",
                    b"BISECT_LOG",
                    b"BISECT_START",
                ];
                let name = if ignore_case {
                    known
                        .iter()
                        .copied()
                        .find(|candidate| name.eq_ignore_ascii_case(candidate))
                        .unwrap_or(name)
                } else {
                    name
                };
                match name {
                    b"config" | b"config.worktree" => {
                        changes.configuration = true;
                        if name == b"config" && directory == &layout.common_dir {
                            // extensions.refStorage can activate an already-present stack.
                            changes.references = true;
                            changes.layout = true;
                        }
                        refresh = true;
                    }
                    b"commondir" | b"gitdir" => {
                        changes.layout = true;
                        refresh = true;
                    }
                    b"HEAD" => {
                        changes.references = true;
                        // Branch-conditioned includes can change precedence without changing source paths.
                        changes.configuration = true;
                        refresh = true;
                    }
                    b"ORIG_HEAD" | b"FETCH_HEAD" | b"packed-refs" | b"shallow" => changes.references = true,
                    b"MERGE_HEAD" | b"REBASE_HEAD" | b"CHERRY_PICK_HEAD" | b"REVERT_HEAD" | b"AUTO_MERGE" => {
                        changes.references = true;
                        changes.operations = true;
                    }
                    b"MERGE_MSG" | b"MERGE_MODE" | b"SQUASH_MSG" | b"BISECT_LOG" | b"BISECT_START" => {
                        changes.operations = true;
                    }
                    _ => {}
                }
            }
        } else if beneath(directory, path) {
            changes.merge(Changes::metadata_all());
            refresh = true;
        }
    }
    if layout
        .worktree
        .as_ref()
        .is_some_and(|root| equal(path, &root.join(".git")) || equal(path, root))
    {
        changes.layout = true;
        refresh = true;
    }
    (changes, refresh)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notify::Dependency;

    fn main_layout() -> Layout {
        let git_dir = Path::new("/repo/.git").to_owned();
        Layout {
            common_dir: git_dir.clone(),
            index: git_dir.join("index"),
            git_dir,
            worktree: Some("/repo".into()),
            dependencies: Vec::new(),
        }
    }

    fn linked_layout() -> Layout {
        let mut layout = main_layout();
        layout.git_dir = layout.common_dir.join("worktrees/selected");
        layout.index = layout.git_dir.join("index");
        layout.worktree = Some("/linked".into());
        layout
    }

    fn references() -> Changes {
        Changes {
            references: true,
            ..Changes::default()
        }
    }

    fn private_stack() -> Changes {
        Changes {
            references: true,
            operations: true,
            configuration: true,
            ..Changes::default()
        }
    }

    fn membership() -> Changes {
        Changes {
            references: true,
            layout: true,
            ..Changes::default()
        }
    }

    #[test]
    fn loose_references_and_head_invalidate_their_dependencies() {
        for layout in [main_layout(), linked_layout()] {
            for name in [
                "refs",
                "refs/heads/main",
                "refs/worktree/private",
                "packed-refs",
                "ORIG_HEAD",
                "FETCH_HEAD",
                "shallow",
                "HEAD",
            ] {
                assert_eq!(
                    classify_metadata(&layout.git_dir.join(name), &layout, false),
                    (
                        Changes {
                            configuration: name == "HEAD",
                            ..references()
                        },
                        matches!(name, "refs" | "HEAD")
                    ),
                    "{name} invalidates references and any branch-conditioned configuration"
                );
            }
        }
    }

    #[test]
    fn operation_refs_files_and_directories_remain_distinct() {
        let layout = main_layout();
        for name in [
            "MERGE_HEAD",
            "REBASE_HEAD",
            "CHERRY_PICK_HEAD",
            "REVERT_HEAD",
            "AUTO_MERGE",
        ] {
            assert_eq!(
                classify_metadata(&layout.git_dir.join(name), &layout, false),
                (
                    Changes {
                        references: true,
                        operations: true,
                        ..Changes::default()
                    },
                    false
                ),
                "{name} is both a reference and operation state"
            );
        }
        for name in [
            "MERGE_MSG",
            "MERGE_MODE",
            "SQUASH_MSG",
            "BISECT_LOG",
            "BISECT_START",
            "rebase-apply",
            "rebase-apply/next",
            "rebase-merge",
            "rebase-merge/head-name",
            "sequencer",
            "sequencer/todo",
        ] {
            assert_eq!(
                classify_metadata(&layout.git_dir.join(name), &layout, false),
                (
                    Changes {
                        operations: true,
                        ..Changes::default()
                    },
                    matches!(name, "rebase-apply" | "rebase-merge" | "sequencer")
                ),
                "{name} only changes operation state; new operation directories need watches"
            );
        }
    }

    #[test]
    fn unrelated_paths_and_indexes_are_not_metadata_invalidations() {
        for layout in [main_layout(), linked_layout()] {
            for name in [
                "index",
                "index.lock",
                "logs/HEAD",
                "logs/refs/heads/main",
                "objects/ab/cdef",
                "description",
                "COMMIT_EDITMSG",
                "unknown",
                "reftable-backup/tables.list",
                "nested/reftable/tables.list",
            ] {
                for scope in [
                    MetadataScope::SelectedOnly,
                    MetadataScope::Primary,
                    MetadataScope::Other,
                ] {
                    assert_eq!(
                        classify_metadata_scoped(&layout.git_dir.join(name), &layout, false, scope),
                        (Changes::default(), false),
                        "{name} is neither recognized metadata nor a recorded dependency"
                    );
                }
            }
        }
    }

    #[test]
    fn recorded_dependencies_and_missing_ancestors_keep_their_kinds() {
        let mut layout = main_layout();
        for (name, kind) in [
            ("includes/missing/config", DependencyKind::Configuration),
            ("info/exclude", DependencyKind::Ignore),
            ("info/attributes", DependencyKind::Attributes),
        ] {
            layout.dependencies.push(Dependency {
                path: layout.git_dir.join(name),
                kind,
            });
        }
        for (name, expected) in [
            (
                "includes",
                Changes {
                    configuration: true,
                    ..Changes::default()
                },
            ),
            (
                "includes/missing/config",
                Changes {
                    configuration: true,
                    ..Changes::default()
                },
            ),
            (
                "info/exclude",
                Changes {
                    ignores: true,
                    ..Changes::default()
                },
            ),
            (
                "info/attributes",
                Changes {
                    attributes: true,
                    ..Changes::default()
                },
            ),
            (
                "info",
                Changes {
                    ignores: true,
                    attributes: true,
                    ..Changes::default()
                },
            ),
        ] {
            assert_eq!(
                classify_metadata(&layout.git_dir.join(name), &layout, false),
                (expected, true),
                "{name} can replace a recorded dependency"
            );
        }
    }

    #[test]
    fn dependencies_survive_index_and_foreign_metadata_exclusions() {
        let mut layout = linked_layout();
        let foreign = layout.common_dir.join("worktrees/other/logs/HEAD");
        for path in [layout.index.clone(), foreign.clone()] {
            layout.dependencies.push(Dependency {
                path,
                kind: DependencyKind::Configuration,
            });
        }
        for path in [&layout.index, &foreign] {
            assert_eq!(
                classify_metadata_scoped(path, &layout, false, MetadataScope::Other),
                (
                    Changes {
                        configuration: true,
                        ..Changes::default()
                    },
                    true
                ),
                "recorded configuration dependencies are independent of metadata ownership"
            );
        }
    }

    #[test]
    fn main_private_files_are_not_shared_with_linked_checkouts() {
        let main = main_layout();
        let linked = linked_layout();
        for name in [
            "HEAD",
            "config.worktree",
            "ORIG_HEAD",
            "MERGE_HEAD",
            "AUTO_MERGE",
            "MERGE_MSG",
            "refs/bisect/bad",
            "refs/rewritten/onto",
            "refs/worktree/private",
            "sequencer/todo",
        ] {
            let path = main.git_dir.join(name);
            let expected = classify_metadata(&path, &main, false);
            assert!(!expected.0.is_empty(), "{name} is recognized main-private metadata");
            assert_eq!(
                classify_metadata_scoped(&path, &main, false, MetadataScope::Other),
                expected,
                "the main child receives its private {name}"
            );
            assert_eq!(
                classify_metadata(&path, &linked, false),
                expected,
                "selected-only monitoring retains coarse common metadata hints"
            );
            for scope in [MetadataScope::Primary, MetadataScope::Other] {
                assert_eq!(
                    classify_metadata_scoped(&path, &linked, false, scope),
                    (Changes::default(), false),
                    "main-private {name} does not belong to the linked checkout"
                );
            }
        }
    }

    #[test]
    fn shared_reference_files_belong_to_the_parent() {
        let main = main_layout();
        let linked = linked_layout();
        for name in ["refs/heads/main", "refs/tags/v1", "packed-refs", "shallow"] {
            let path = main.common_dir.join(name);
            for layout in [&main, &linked] {
                assert_eq!(
                    classify_metadata_scoped(&path, layout, false, MetadataScope::Primary),
                    (references(), false),
                    "the parent owns shared {name}, regardless of its checkout"
                );
                assert_eq!(
                    classify_metadata_scoped(&path, layout, false, MetadataScope::Other),
                    (Changes::default(), false),
                    "children do not duplicate shared {name}"
                );
            }
        }
    }

    #[test]
    fn registration_hints_include_foreign_table_backed_heads() {
        for layout in [main_layout(), linked_layout()] {
            for (name, refresh) in [
                ("worktrees", true),
                ("worktrees/other", true),
                ("worktrees/other/HEAD", false),
                ("worktrees/other/gitdir", false),
                ("worktrees/other/reftable", true),
                ("worktrees/other/reftable/tables.list", false),
                ("worktrees/other/reftable/generated.ref", false),
            ] {
                let path = layout.common_dir.join(name);
                for scope in [MetadataScope::SelectedOnly, MetadataScope::Primary] {
                    assert_eq!(
                        classify_metadata_scoped(&path, &layout, false, scope),
                        (membership(), refresh),
                        "{name} can change aggregate worktree reference views"
                    );
                }
                assert_eq!(
                    classify_metadata_scoped(&path, &layout, false, MetadataScope::Other),
                    (Changes::default(), false),
                    "foreign {name} is not private state of a sibling"
                );
            }
            for name in [
                "index",
                "logs/HEAD",
                "refs/worktree/private",
                "config.worktree",
                "reftable-backup/tables.list",
                "nested/reftable/tables.list",
            ] {
                assert_eq!(
                    classify_metadata(&layout.common_dir.join("worktrees/other").join(name), &layout, false),
                    (Changes::default(), false),
                    "foreign {name} does not imply worktree membership or HEAD changes"
                );
            }
        }
    }

    #[test]
    fn current_linked_stack_is_private_in_every_scope() {
        let layout = linked_layout();
        for name in [
            "reftable",
            "reftable/tables.list",
            "reftable/generated.ref",
            "reftable/tables.list.lock",
        ] {
            for scope in [
                MetadataScope::SelectedOnly,
                MetadataScope::Primary,
                MetadataScope::Other,
            ] {
                assert_eq!(
                    classify_metadata_scoped(&layout.git_dir.join(name), &layout, false, scope),
                    (private_stack(), true),
                    "{name} can contain this checkout HEAD or operation refs; lock-event filtering is external"
                );
            }
        }
    }

    #[test]
    fn common_stack_reaches_shared_views_and_main_private_state() {
        let main = main_layout();
        let linked = linked_layout();
        for name in ["reftable", "reftable/tables.list", "reftable/generated.ref"] {
            let path = main.common_dir.join(name);
            for scope in [
                MetadataScope::SelectedOnly,
                MetadataScope::Primary,
                MetadataScope::Other,
            ] {
                assert_eq!(
                    classify_metadata_scoped(&path, &main, false, scope),
                    (private_stack(), true),
                    "{name} cannot distinguish shared refs from main HEAD and operation refs"
                );
            }
            assert_eq!(
                classify_metadata(&path, &linked, false),
                (private_stack(), true),
                "selected-only monitoring preserves its coarse treatment of main-private state"
            );
            assert_eq!(
                classify_metadata_scoped(&path, &linked, false, MetadataScope::Primary),
                (references(), name == "reftable"),
                "a linked primary receives shared refs without claiming its own HEAD or operation changed"
            );
            assert_eq!(
                classify_metadata_scoped(&path, &linked, false, MetadataScope::Other),
                (Changes::default(), false),
                "a linked sibling does not own any part of the common stack"
            );
        }
    }

    #[test]
    fn shared_configuration_may_activate_an_existing_reference_backend() {
        for layout in [main_layout(), linked_layout()] {
            for scope in [MetadataScope::SelectedOnly, MetadataScope::Primary] {
                assert_eq!(
                    classify_metadata_scoped(&layout.common_dir.join("config"), &layout, false, scope),
                    (
                        Changes {
                            configuration: true,
                            references: true,
                            layout: true,
                            ..Changes::default()
                        },
                        true
                    ),
                    "extensions.refStorage can change without creating or deleting a stack directory"
                );
            }
            assert_eq!(
                classify_metadata(&layout.git_dir.join("config.worktree"), &layout, false),
                (
                    Changes {
                        configuration: true,
                        ..Changes::default()
                    },
                    true
                ),
                "private configuration retains its existing classification"
            );
        }
    }

    #[test]
    fn git_roots_and_layout_markers_request_rediscovery() {
        let layout = main_layout();
        for path in [&layout.git_dir, Path::new("/repo")] {
            assert_eq!(
                classify_metadata(path, &layout, false),
                (Changes::metadata_all(), true),
                "an event at a Git directory or ancestor cannot identify individual metadata changes"
            );
        }
        let layout = linked_layout();
        for path in [
            layout.git_dir.join("commondir"),
            layout.git_dir.join("gitdir"),
            Path::new("/linked").to_owned(),
            Path::new("/linked/.git").to_owned(),
        ] {
            assert_eq!(
                classify_metadata(&path, &layout, false),
                (
                    Changes {
                        layout: true,
                        ..Changes::default()
                    },
                    true
                ),
                "layout markers request rediscovery without inventing lifecycle events"
            );
        }
    }

    #[test]
    fn dependency_classification_observes_loose_and_table_backed_heads() {
        for layout in [main_layout(), linked_layout()] {
            for root in [&layout.git_dir, &layout.common_dir] {
                for name in [
                    "HEAD",
                    "config",
                    "config.worktree",
                    "reftable",
                    "reftable/tables.list",
                    "reftable/generated.ref",
                ] {
                    assert_eq!(
                        classify_dependencies(&root.join(name), &layout, false),
                        (
                            Changes {
                                configuration: true,
                                worktree: Some(Scope::All),
                                ..Changes::default()
                            },
                            true
                        ),
                        "{name} refreshes coverage dependencies without publishing reference or operation flags"
                    );
                }
            }
        }
    }

    #[test]
    fn dependencies_refresh_for_git_directories_and_their_ancestors() {
        for layout in [main_layout(), linked_layout()] {
            for path in layout.git_dir.ancestors() {
                assert_eq!(
                    classify_dependencies(path, &layout, false),
                    (
                        Changes {
                            layout: true,
                            worktree: Some(Scope::All),
                            ..Changes::default()
                        },
                        true
                    ),
                    "{} is the selected Git directory or an ancestor whose replacement can invalidate coverage",
                    path.display()
                );
            }
        }
    }

    #[test]
    fn dependencies_ignore_unrelated_metadata_and_foreign_stacks() {
        let layout = linked_layout();
        for name in [
            "refs",
            "refs/heads/main",
            "rebase-merge",
            "logs/HEAD",
            "FETCH_HEAD",
            "info/attributes",
            "worktrees/other",
            "worktrees/selected-sibling",
            "worktrees/other/HEAD",
            "worktrees/other/reftable/tables.list",
            "reftable-backup/tables.list",
        ] {
            assert_eq!(
                classify_dependencies(&layout.common_dir.join(name), &layout, false),
                (Changes::default(), false),
                "{name} neither selects coverage nor discovers sibling worktrees"
            );
        }
    }

    #[test]
    fn coverage_only_uses_configuration_ignore_and_layout_dependencies() {
        let mut layout = linked_layout();
        for (name, kind) in [
            ("includes/missing/config", DependencyKind::Configuration),
            ("info/exclude", DependencyKind::Ignore),
            ("info/attributes", DependencyKind::Attributes),
        ] {
            layout.dependencies.push(Dependency {
                path: layout.common_dir.join(name),
                kind,
            });
        }
        for (name, expected) in [
            (
                "includes",
                Changes {
                    configuration: true,
                    worktree: Some(Scope::All),
                    ..Changes::default()
                },
            ),
            (
                "info",
                Changes {
                    ignores: true,
                    worktree: Some(Scope::All),
                    ..Changes::default()
                },
            ),
            (
                "commondir",
                Changes {
                    layout: true,
                    worktree: Some(Scope::All),
                    ..Changes::default()
                },
            ),
        ] {
            assert_eq!(
                classify_dependencies(&layout.common_dir.join(name), &layout, false),
                (expected, true),
                "{name} can change directory coverage"
            );
        }
        assert_eq!(
            classify_dependencies(&layout.common_dir.join("info/attributes"), &layout, false),
            (Changes::default(), false),
            "attribute-only dependencies do not select ignore-aware coverage"
        );
    }

    #[test]
    fn tables_use_native_case_folding_and_path_component_boundaries() {
        for layout in [main_layout(), linked_layout()] {
            let path = layout.git_dir.join("REFTABLE/TABLES.LIST");
            assert_eq!(
                classify_metadata(&path, &layout, true),
                (private_stack(), true),
                "case folding recognizes both common and linked stacks"
            );
            assert_eq!(
                classify_metadata(&path, &layout, false),
                (Changes::default(), false),
                "case-sensitive layouts do not recognize a differently spelled stack"
            );
            assert_eq!(
                classify_dependencies(&path, &layout, true),
                (
                    Changes {
                        configuration: true,
                        worktree: Some(Scope::All),
                        ..Changes::default()
                    },
                    true
                ),
                "dependency and metadata classifiers agree on case folding"
            );
            assert_eq!(
                classify_metadata(&layout.git_dir.join("refs/heads/reftable"), &layout, false),
                (references(), false),
                "a ref named reftable is not a storage stack"
            );
        }
        let main = main_layout();
        let linked = linked_layout();
        let path = main.git_dir.join("REFTABLE/TABLES.LIST");
        assert_eq!(
            classify_metadata_scoped(&path, &linked, true, MetadataScope::Primary),
            (references(), false),
            "case-folded common stacks retain shared routing"
        );
        assert_eq!(
            classify_metadata_scoped(&path, &linked, true, MetadataScope::Other),
            (Changes::default(), false),
            "case folding must not bypass sibling ownership"
        );
        let path = main.common_dir.join("WORKTREES/SELECTED/REFTABLE/TABLES.LIST");
        assert_eq!(
            classify_metadata_scoped(&path, &linked, true, MetadataScope::Other),
            (private_stack(), true),
            "case-folded registration paths still identify the current linked stack"
        );
        assert_eq!(
            classify_metadata(&path, &main, true),
            (membership(), false),
            "case-folded foreign stacks retain aggregate membership hints"
        );
    }

    #[test]
    fn fixture_layouts_keep_private_shared_and_foreign_metadata_separate() -> gix_error::TestResult {
        for ref_format in ["files", "reftable"] {
            let Some(fixture) = gix_testtools::scripted_fixture_read_only_with_args_with_git_version(
                "make_notify_worktrees.sh",
                [ref_format],
                |version| ref_format == "files" || version >= (2, 45, 0),
            )?
            else {
                continue;
            };
            for (selected, sibling) in [("main", "linked-a"), ("linked-a", "linked-b"), ("bare.git", "bare-a")] {
                let repo = crate::open_opts(fixture.join(selected), crate::open::Options::isolated())?;
                let sibling = crate::open_opts(fixture.join(sibling), crate::open::Options::isolated())?;
                assert_classification(
                    &Layout::from_repository(&repo)?,
                    &Layout::from_repository(&sibling)?.git_dir,
                );
            }
        }
        Ok(())
    }

    fn assert_classification(layout: &Layout, sibling_git_dir: &Path) {
        let private_stack = (
            Changes {
                references: true,
                configuration: true,
                operations: true,
                ..Changes::default()
            },
            true,
        );
        let configuration_dependency = (
            Changes {
                configuration: true,
                worktree: Some(Scope::All),
                ..Changes::default()
            },
            true,
        );
        let linked = layout.git_dir != layout.common_dir;
        for name in ["reftable", "reftable/tables.list"] {
            let private = layout.git_dir.join(name);
            let common = layout.common_dir.join(name);
            for scope in [
                MetadataScope::SelectedOnly,
                MetadataScope::Primary,
                MetadataScope::Other,
            ] {
                assert_eq!(
                    classify_metadata_scoped(&private, layout, false, scope),
                    private_stack,
                    "{scope:?} recognizes the current main or linked stack even with files reference storage"
                );
                let expected = if !linked || scope == MetadataScope::SelectedOnly {
                    private_stack.clone()
                } else if scope == MetadataScope::Primary {
                    (
                        Changes {
                            references: true,
                            ..Changes::default()
                        },
                        name == "reftable",
                    )
                } else {
                    (Changes::default(), false)
                };
                assert_eq!(
                    classify_metadata_scoped(&common, layout, false, scope),
                    expected,
                    "{scope:?} routes the common stack to shared views and main-private state, not linked-private operations"
                );
            }
            for path in [&private, &common] {
                assert_eq!(
                    classify_dependencies(path, layout, false),
                    configuration_dependency,
                    "current and common stacks refresh branch-conditioned coverage without reference or operation flags"
                );
            }
        }

        for (name, references) in [("MERGE_HEAD", true), ("MERGE_MSG", false)] {
            let operation = (
                Changes {
                    references,
                    operations: true,
                    ..Changes::default()
                },
                false,
            );
            for scope in [
                MetadataScope::SelectedOnly,
                MetadataScope::Primary,
                MetadataScope::Other,
            ] {
                assert_eq!(
                    classify_metadata_scoped(&layout.git_dir.join(name), layout, false, scope),
                    operation,
                    "{scope:?} retains operation state belonging to the current checkout"
                );
                assert_eq!(
                    classify_metadata_scoped(&layout.common_dir.join(name), layout, false, scope),
                    if linked && scope != MetadataScope::SelectedOnly {
                        (Changes::default(), false)
                    } else {
                        operation.clone()
                    },
                    "main-private operation files are not linked-private state in all-worktree routing"
                );
            }
            assert_eq!(
                classify_dependencies(&layout.git_dir.join(name), layout, false),
                (Changes::default(), false),
                "operation state alone does not change ignore-aware coverage dependencies"
            );
        }

        for (name, refresh_inventory) in [
            ("", true),
            ("HEAD", false),
            ("gitdir", false),
            ("reftable", true),
            ("reftable/tables.list", false),
        ] {
            let path = sibling_git_dir.join(name);
            for scope in [
                MetadataScope::SelectedOnly,
                MetadataScope::Primary,
                MetadataScope::Other,
            ] {
                let expected = if scope == MetadataScope::Other {
                    (Changes::default(), false)
                } else {
                    (
                        Changes {
                            references: true,
                            layout: true,
                            ..Changes::default()
                        },
                        refresh_inventory,
                    )
                };
                assert_eq!(
                    classify_metadata_scoped(&path, layout, false, scope),
                    expected,
                    "{scope:?} treats a foreign registration or HEAD as an aggregate hint, not private operation state"
                );
            }
            assert_eq!(
                classify_dependencies(&path, layout, false),
                (Changes::default(), false),
                "dependency-only classification does not discover sibling worktrees"
            );
        }

        let uppercase_stack = layout.git_dir.join("REFTABLE/TABLES.LIST");
        assert_eq!(
            classify_metadata_scoped(&uppercase_stack, layout, true, MetadataScope::Primary),
            private_stack,
            "callers can request ASCII case folding independently of the host filesystem"
        );
        assert_eq!(
            classify_dependencies(&uppercase_stack, layout, true),
            configuration_dependency,
            "dependency classification uses the same case-folding policy"
        );
        for path in [uppercase_stack, layout.git_dir.join("reftable-backup/tables.list")] {
            assert_eq!(
                classify_metadata_scoped(&path, layout, false, MetadataScope::Primary),
                (Changes::default(), false),
                "case-sensitive classification requires exact path components"
            );
            assert_eq!(
                classify_dependencies(&path, layout, false),
                (Changes::default(), false),
                "unrelated stack-like paths do not select coverage dependencies"
            );
        }
    }
}

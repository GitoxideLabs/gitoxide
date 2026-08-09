use std::{hint::black_box, path::PathBuf};

use bstr::ByteSlice;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use gix_dir::walk;
use gix_testtools::FixtureState;

fn dirwalk(c: &mut Criterion) {
    let fixture = gix_testtools::rust_fixture_read_only("dirwalk-benchmark", 2, |state| {
        if let FixtureState::Uninitialized(root) = state {
            create_clean_flat(&root.join("clean-flat"))?;
            create_clean_wide(&root.join("clean-wide"))?;
            create_untracked_wide(&root.join("untracked-wide"))?;
        }
        Ok(())
    })
    .expect("benchmark fixture can be created")
    .0;

    let mut group = c.benchmark_group("dirwalk");
    for name in ["clean-flat", "clean-wide", "untracked-wide"] {
        let scenario = Scenario::new(fixture.join(name));
        group.bench_with_input(BenchmarkId::from_parameter(name), &scenario, |b, scenario| {
            b.iter(|| black_box(scenario.walk()));
        });
    }
}

criterion_group!(benches, dirwalk);
criterion_main!(benches);

struct Scenario {
    root: PathBuf,
    git_dir_realpath: PathBuf,
    index: gix_index::State,
}

impl Scenario {
    fn new(root: PathBuf) -> Self {
        let git_dir = root.join(".git");
        let index = std::fs::read(git_dir.join("index"))
            .map_err(|err| format!("cannot read benchmark index: {err}"))
            .and_then(|bytes| {
                gix_index::State::from_bytes(
                    &bytes,
                    std::time::UNIX_EPOCH.into(),
                    gix_index::hash::Kind::Sha1,
                    Default::default(),
                )
                .map(|(index, _)| index)
                .map_err(|err| format!("cannot decode benchmark index: {err}"))
            })
            .expect("Git creates a valid benchmark index");
        assert!(index.untracked().is_some(), "Git must populate the UNTR cache");
        Scenario {
            git_dir_realpath: gix_path::realpath(&git_dir).expect("git directory can be resolved"),
            root,
            index,
        }
    }

    fn walk(&self) -> walk::Outcome {
        let mut pathspec = gix_pathspec::Search::from_specs(
            std::iter::empty::<gix_pathspec::Pattern>(),
            None,
            "benchmark has no absolute pathspecs".as_ref(),
        )
        .expect("empty pathspec is valid");
        let mut excludes = gix_worktree::Stack::from_state_and_ignore_case(
            &self.root,
            false,
            gix_worktree::stack::State::IgnoreStack(gix_worktree::stack::state::Ignore::new(
                Default::default(),
                Default::default(),
                None,
                gix_worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped,
                Default::default(),
            )),
            &self.index,
            self.index.path_backing(),
        );
        let mut delegate = Ignore;
        walk(
            &self.root,
            walk::Context {
                should_interrupt: None,
                git_dir_realpath: &self.git_dir_realpath,
                current_dir: &self.root,
                index: &self.index,
                ignore_case_index_lookup: None,
                pathspec: &mut pathspec,
                pathspec_attributes: &mut |_, _, _, _| unreachable!("benchmark pathspecs have no attributes"),
                excludes: Some(&mut excludes),
                objects: &gix_object::find::Never,
                explicit_traversal_root: None,
            },
            walk::Options {
                use_untracked_cache: true,
                emit_untracked: walk::EmissionMode::CollapseDirectory,
                ..Default::default()
            },
            &mut delegate,
        )
        .expect("benchmark dirwalk succeeds")
        .0
    }
}

struct Ignore;

impl walk::Delegate for Ignore {
    fn emit(
        &mut self,
        _entry: gix_dir::EntryRef<'_>,
        _collapsed_directory_status: Option<gix_dir::entry::Status>,
    ) -> walk::Action {
        std::ops::ControlFlow::Continue(())
    }
}

fn create_clean_flat(root: &std::path::Path) -> gix_testtools::Result {
    init(root)?;
    for file_idx in 0..10_000 {
        std::fs::write(root.join(format!("file-{file_idx:05}")), [])?;
    }
    commit_and_prime(root)
}

fn create_clean_wide(root: &std::path::Path) -> gix_testtools::Result {
    init(root)?;
    create_wide_tree(root)?;
    commit_and_prime(root)
}

fn create_untracked_wide(root: &std::path::Path) -> gix_testtools::Result {
    init(root)?;
    std::fs::write(root.join("tracked"), [])?;
    commit_and_prime(root)?;
    create_wide_tree(root)?;
    prime(root)
}

fn create_wide_tree(root: &std::path::Path) -> gix_testtools::Result {
    for dir_idx in 0..100 {
        let dir = root.join(format!("dir-{dir_idx:03}"));
        std::fs::create_dir(&dir)?;
        for file_idx in 0..100 {
            std::fs::write(dir.join(format!("file-{file_idx:03}")), [])?;
        }
    }
    Ok(())
}

fn init(root: &std::path::Path) -> gix_testtools::Result {
    std::fs::create_dir(root)?;
    gix_testtools::git(root, "init --quiet")?;
    gix_testtools::git(root, "config core.untrackedCache true")?;
    gix_testtools::git(root, "config core.excludesFile .git/no-global-excludes")?;
    Ok(())
}

fn commit_and_prime(root: &std::path::Path) -> gix_testtools::Result {
    gix_testtools::git(root, "add .")?;
    gix_testtools::git(root, "commit --quiet -m baseline")?;
    prime(root)
}

fn prime(root: &std::path::Path) -> gix_testtools::Result {
    let status = gix_testtools::git(root, "status --porcelain")?;
    black_box(status.as_bytes().as_bstr());
    Ok(())
}

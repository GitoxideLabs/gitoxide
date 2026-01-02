use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};

use anyhow::{bail, Context};
use bytesize::ByteSize;
use gix::{Count, NestedProgress, Progress};
use rusqlite::params;

use super::db;
use crate::{
    corpus,
    corpus::{Engine, Task},
    organize::find_git_repository_workdirs,
};

pub type ProgressItem = gix::progress::DoOrDiscard<gix::progress::prodash::tree::Item>;

pub struct State {
    pub progress: ProgressItem,
    pub gitoxide_version: String,
    pub trace_to_progress: bool,
    pub reverse_trace_lines: bool,
}

impl Engine {
    /// Open the corpus DB or create it.
    pub fn open_or_create(db: PathBuf, state: State) -> anyhow::Result<Engine> {
        let con = crate::corpus::db::create(db).context("Could not open or create database")?;
        Ok(Engine { con, state })
    }

    /// Run on the existing set of repositories we have already seen or obtain them from `path` if there is none yet.
    pub fn run(
        &mut self,
        corpus_path: PathBuf,
        threads: Option<usize>,
        dry_run: bool,
        repo_sql_suffix: Option<String>,
        allowed_task_names: Vec<String>,
    ) -> anyhow::Result<()> {
        let tasks = self.tasks_or_insert(&allowed_task_names)?;
        if tasks.is_empty() {
            bail!("Cannot run without any task to perform on the repositories");
        }
        let (corpus_path, corpus_id) = self.prepare_corpus_path(corpus_path)?;
        let gitoxide_id = self.gitoxide_version_id_or_insert()?;
        let runner_id = self.runner_id_or_insert()?;
        let repos = self.find_repos_or_insert(&corpus_path, corpus_id, repo_sql_suffix)?;
        self.perform_run(&corpus_path, gitoxide_id, runner_id, &tasks, repos, threads, dry_run)
    }

    pub fn refresh(&mut self, corpus_path: PathBuf) -> anyhow::Result<()> {
        let (corpus_path, corpus_id) = self.prepare_corpus_path(corpus_path)?;
        let repos = self.refresh_repos(&corpus_path, corpus_id)?;
        self.state.progress.set_name("refresh repos".into());
        self.state.progress.info(format!(
            "Added or updated {} repositories under '{corpus_path}'",
            repos.len(),
            corpus_path = corpus_path.display(),
        ));
        Ok(())
    }
}

impl Engine {
    #[allow(clippy::too_many_arguments)]
    fn perform_run(
        &mut self,
        corpus_path: &Path,
        gitoxide_id: db::Id,
        runner_id: db::Id,
        tasks: &[(db::Id, &'static Task)],
        mut repos: Vec<db::Repo>,
        threads: Option<usize>,
        dry_run: bool,
    ) -> anyhow::Result<()> {
        let start = Instant::now();
        let repo_progress = &mut self.state.progress;
        let threads = gix::parallel::num_threads(threads);
        let db_path = self.con.path().expect("opened from path on disk").to_owned();
        'tasks_loop: for (task_id, task) in tasks {
            let task_start = Instant::now();
            let task_info = format!("run '{}'", task.short_name);
            repo_progress.set_name(task_info.clone());
            repo_progress.init(Some(repos.len()), gix::progress::count("repos"));
            if task.execute_exclusive || threads == 1 || dry_run {
                if dry_run {
                    repo_progress.set_name("WOULD run".into());
                    for repo in &repos {
                        repo_progress.info(format!(
                            "{}",
                            repo.path
                                .strip_prefix(corpus_path)
                                .expect("corpus contains repo")
                                .display()
                        ));
                        repo_progress.inc();
                    }
                    repo_progress.info(format!("with {} tasks", tasks.len()));
                    for (_, task) in tasks {
                        repo_progress.info(format!("task '{}' ({})", task.description, task.short_name));
                    }
                    break 'tasks_loop;
                }
                let mut run_progress = repo_progress.add_child("set later");
                let (_guard, current_id) = corpus::trace::override_thread_subscriber(
                    db_path.as_str(),
                    self.state.trace_to_progress.then(|| repo_progress.add_child("trace")),
                    self.state.reverse_trace_lines,
                )?;

                let mut num_errors = 0;
                for repo in &repos {
                    if gix::interrupt::is_triggered() {
                        bail!("interrupted by user");
                    }
                    run_progress.set_name(format!(
                        "{}",
                        repo.path
                            .strip_prefix(corpus_path)
                            .expect("corpus contains repo")
                            .display()
                    ));

                    // TODO: wait for new release of `tracing-forest` to be able to provide run_id via span attributes
                    let mut run = Self::insert_run(&self.con, gitoxide_id, runner_id, *task_id, repo.id)?;
                    current_id.store(run.id, Ordering::SeqCst);
                    tracing::info_span!("run", run_id = run.id).in_scope(|| {
                        task.perform(
                            &mut run,
                            &repo.path,
                            &mut run_progress,
                            Some(threads),
                            &gix::interrupt::IS_INTERRUPTED,
                        );
                    });
                    if let Some(err) = run.error.as_deref() {
                        num_errors += 1;
                        repo_progress.fail(err.to_owned());
                    }
                    Self::update_run(&self.con, run)?;
                    repo_progress.inc();
                }
                repo_progress.show_throughput(task_start);
                if num_errors != 0 {
                    repo_progress.fail(format!(
                        "{} repositories failed to run task {}",
                        num_errors, task.short_name
                    ));
                }
            } else {
                let counter = repo_progress.counter();
                let num_errors = AtomicUsize::default();
                let repo_progress = gix::threading::OwnShared::new(gix::threading::Mutable::new(
                    repo_progress.add_child("in parallel"),
                ));
                gix::parallel::in_parallel_with_slice(
                    &mut repos,
                    Some(threads),
                    {
                        let shared_repo_progress = repo_progress.clone();
                        let db_path = db_path.clone();
                        move |tid| {
                            let mut progress = gix::threading::lock(&shared_repo_progress);
                            (
                                // threaded printing is usually spammy, and lines interleave so it's useless.
                                corpus::trace::override_thread_subscriber(db_path.as_str(), None, false),
                                progress.add_child(format!("{tid}")),
                                rusqlite::Connection::open(&db_path),
                            )
                        }
                    },
                    |repo, (subscriber, progress, con), _threads_left, should_interrupt| -> anyhow::Result<()> {
                        progress.set_name(format!(
                            "{}",
                            repo.path
                                .strip_prefix(corpus_path)
                                .expect("corpus contains repo")
                                .display()
                        ));
                        let current_id = match subscriber {
                            Ok((_guard, current_id)) => current_id,
                            Err(err) => {
                                progress.fail(format!("{err:#?}"));
                                should_interrupt.store(true, Ordering::SeqCst);
                                return Ok(());
                            }
                        };
                        let con = match con {
                            Ok(con) => con,
                            Err(err) => {
                                progress.fail(format!("{err:#?}"));
                                should_interrupt.store(true, Ordering::SeqCst);
                                return Ok(());
                            }
                        };
                        let mut run = Self::insert_run(con, gitoxide_id, runner_id, *task_id, repo.id)?;
                        current_id.store(run.id, Ordering::SeqCst);
                        tracing::info_span!("run", run_id = run.id).in_scope(|| {
                            task.perform(&mut run, &repo.path, progress, Some(1), should_interrupt);
                        });
                        if let Some(err) = run.error.as_deref() {
                            num_errors.fetch_add(1, Ordering::Relaxed);
                            progress.fail(err.to_owned());
                        }
                        Self::update_run(con, run)?;
                        counter.fetch_add(1, Ordering::Relaxed);
                        Ok(())
                    },
                    || (!gix::interrupt::is_triggered()).then(|| Duration::from_millis(100)),
                    std::convert::identity,
                )?;
                let repo_progress = gix::threading::lock(&repo_progress);
                repo_progress.show_throughput(task_start);
                let num_errors = num_errors.load(Ordering::Relaxed);
                if num_errors != 0 {
                    repo_progress.fail(format!(
                        "{} repositories failed to run task {}",
                        num_errors, task.short_name
                    ));
                }
            }

            repo_progress.inc();
        }
        repo_progress.show_throughput(start);
        Ok(())
    }

    fn prepare_corpus_path(&self, corpus_path: PathBuf) -> anyhow::Result<(PathBuf, db::Id)> {
        let corpus_path = gix::path::realpath(corpus_path)?;
        let corpus_id = self.corpus_id_or_insert(&corpus_path)?;
        Ok((corpus_path, corpus_id))
    }

    fn find_repos(
        &mut self,
        corpus_path: &Path,
        corpus_id: db::Id,
        sql_suffix: Option<&str>,
    ) -> anyhow::Result<Vec<db::Repo>> {
        self.state.progress.set_name("query db-repos".into());
        self.state.progress.init(None, gix::progress::count("repos"));

        Ok(self
            .con
            .prepare(&format!(
                "SELECT id, rela_path, odb_size, num_objects, num_references FROM repository WHERE corpus = ?1 {}",
                sql_suffix.unwrap_or_default()
            ))?
            .query_map([corpus_id], |r| {
                Ok(db::Repo {
                    id: r.get(0)?,
                    path: corpus_path.join(r.get::<_, String>(1)?),
                    odb_size: ByteSize(r.get::<_, i64>(2)? as u64),
                    num_objects: r.get::<_, i64>(3)? as u64,
                    num_references: r.get::<_, i64>(4)? as usize,
                })
            })?
            .inspect(|_| self.state.progress.inc())
            .collect::<Result<_, _>>()?)
    }

    fn refresh_repos(&mut self, corpus_path: &Path, corpus_id: db::Id) -> anyhow::Result<Vec<db::Repo>> {
        let start = Instant::now();
        self.state.progress.set_name("refresh".into());
        self.state.progress.init(None, gix::progress::count("repos"));

        let repos = std::thread::scope({
            let progress = &mut self.state.progress;
            let con = &mut self.con;
            |scope| -> anyhow::Result<_> {
                let threads = std::thread::available_parallelism()
                    .map(std::num::NonZeroUsize::get)
                    .ok()
                    .unwrap_or(1);
                let (path_tx, repo_rx) = {
                    let (path_tx, path_rx) = crossbeam_channel::bounded(threads * 2);
                    let (repo_tx, repo_rx) = std::sync::mpsc::channel::<(PathBuf, anyhow::Result<db::Repo>)>();
                    (0..threads).for_each(|_| {
                        scope.spawn({
                            let path_rx = path_rx.clone();
                            let repo_tx = repo_tx.clone();
                            move || -> anyhow::Result<_> {
                                for repo_path in path_rx {
                                    let res = (|| {
                                        let repo = gix::open_opts(&repo_path, gix::open::Options::isolated())?;
                                        db::Repo::try_from(&repo)
                                    })();
                                    repo_tx.send((repo_path, res))?;
                                }
                                Ok(())
                            }
                        });
                    });
                    (path_tx, repo_rx)
                };

                let find_progress = progress.add_child("find");
                let write_db = scope.spawn(move || -> anyhow::Result<Vec<db::Repo>> {
                    progress.set_name("write to DB".into());
                    progress.init(None, gix::progress::count("repos"));

                    let mut out = Vec::new();
                    let transaction = con.transaction()?;
                    let mut statement = transaction.prepare("INSERT INTO repository (rela_path, corpus, odb_size, num_objects, num_references) VALUES (?1, ?2, ?3, ?4, ?5)\
                                                    ON CONFLICT DO UPDATE SET rela_path = rela_path, corpus = corpus, odb_size = ?3, num_objects = ?4, num_references = ?5\
                                                    RETURNING id")?;
                    for (repo_path, repo_res) in repo_rx {
                        match repo_res {
                            Ok(mut repo) => {
                                let rela_path = repo.path.strip_prefix(corpus_path)?;
                                repo.id = statement.query_row(params![rela_path.to_str().context("only valid UTF8 is allowed for repository paths")?, corpus_id, repo.odb_size.as_u64() as i64, repo.num_objects as i64, repo.num_references as i64], |r| r.get(0))?;
                                out.push(repo);
                                progress.inc();
                            }
                            Err(err) => progress.fail(format!("{repo_path}: {err:#?}", repo_path = repo_path.display())),
                        }
                    }
                    statement.finalize()?;
                    transaction.commit()?;
                    progress.show_throughput(start);
                    Ok(out)
                });

                let repos = gix::interrupt::Iter::new(
                    find_git_repository_workdirs(corpus_path, find_progress, false, Some(threads)),
                    || anyhow::anyhow!("interrupted by user"),
                );
                for res in repos {
                    let (repo_path, _kind) = res?;
                    path_tx.send(repo_path)?;
                }
                drop(path_tx);
                write_db.join().expect("no panic")
            }
        })?;

        Ok(repos)
    }

    fn find_repos_or_insert(
        &mut self,
        corpus_path: &Path,
        corpus_id: db::Id,
        sql_suffix: Option<String>,
    ) -> anyhow::Result<Vec<db::Repo>> {
        let start = Instant::now();
        let repos = self.find_repos(corpus_path, corpus_id, sql_suffix.as_deref())?;
        if repos.is_empty() {
            let res = self.refresh_repos(corpus_path, corpus_id);
            if sql_suffix.is_some() {
                self.find_repos(corpus_path, corpus_id, sql_suffix.as_deref())
            } else {
                res
            }
        } else {
            self.state.progress.show_throughput(start);
            Ok(repos)
        }
    }
}

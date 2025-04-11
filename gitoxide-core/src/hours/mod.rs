use std::{collections::BTreeSet, io, path::Path, time::Instant};

use anyhow::bail;
use gix::{
    actor,
    bstr::{BStr, ByteSlice},
    prelude::*,
    progress, Count, NestedProgress, Progress,
};

/// Additional configuration for the hours estimation functionality.
pub struct Context<W> {
    /// Ignore github bots which match the `[bot]` search string.
    pub ignore_bots: bool,
    /// Show personally identifiable information before the summary. Includes names and email addresses.
    pub show_pii: bool,
    /// Collect how many files have been added, removed and modified (without rename tracking).
    pub file_stats: bool,
    /// Collect how many lines in files have been added, removed and modified (without rename tracking).
    pub line_stats: bool,
    /// The amount of threads to use. If unset, use all cores, if 0 use al physical cores.
    pub threads: Option<usize>,
    /// Omit unifying identities by name and email which can lead to the same author appear multiple times
    /// due to using different names or email addresses.
    pub omit_unify_identities: bool,
    /// Where to write our output to
    pub out: W,
}

/// Estimate the hours it takes to produce the content of the repository in `_working_dir_`, with `_refname_` for
/// the start of the commit graph traversal.
///
/// * `_working_dir_` - The directory containing a '.git/' folder.
/// * `_refname_` - The name of the ref like 'main' or 'master' at which to start iterating the commit graph.
/// * `_progress_` - A way to provide progress and performance information
pub fn estimate<W, P>(
    working_dir: &Path,
    rev_spec: &BStr,
    mut progress: P,
    Context {
        show_pii,
        ignore_bots,
        file_stats,
        line_stats,
        omit_unify_identities,
        threads,
        mut out,
    }: Context<W>,
) -> anyhow::Result<()>
where
    W: io::Write,
    P: NestedProgress,
{
    let repo = gix::discover(working_dir)?;
    let commit_id = repo.rev_parse_single(rev_spec)?.detach();
    let mut string_heap = BTreeSet::<&'static [u8]>::new();
    let needs_stats = file_stats || line_stats;
    let threads = gix::features::parallel::num_threads(threads);

    let (commit_authors, stats, is_shallow, skipped_merge_commits) = {
        std::thread::scope(|scope| -> anyhow::Result<_> {
            let start = Instant::now();
            let (tx, rx) = std::sync::mpsc::channel::<(u32, Vec<u8>)>();
            let mailmap = repo.open_mailmap();

            let extract_signatures = scope.spawn(move || -> anyhow::Result<Vec<_>> {
                let mut out = Vec::new();
                for (commit_idx, commit_data) in rx {
                    if let Ok(author) = gix::objs::CommitRefIter::from_bytes(&commit_data)
                        .author()
                        .map(|author| mailmap.resolve_cow(author.trim()))
                    {
                        let mut string_ref = |s: &[u8]| -> &'static BStr {
                            match string_heap.get(s) {
                                Some(n) => n.as_bstr(),
                                None => {
                                    let sv: Vec<u8> = s.to_owned();
                                    string_heap.insert(Box::leak(sv.into_boxed_slice()));
                                    (*string_heap.get(s).expect("present")).as_ref()
                                }
                            }
                        };
                        let name = string_ref(author.name.as_ref());
                        let email = string_ref(author.email.as_ref());
                        let time = string_ref(author.time.as_ref());

                        out.push((commit_idx, actor::SignatureRef { name, email, time }));
                    }
                }
                out.shrink_to_fit();
                out.sort_by(|a, b| {
                    a.1.email
                        .cmp(b.1.email)
                        .then(a.1.seconds().cmp(&b.1.seconds()).reverse())
                });
                Ok(out)
            });

            let (stats_progresses, stats_counters) = needs_stats
                .then(|| {
                    let mut sp = progress.add_child("extract stats");
                    sp.init(None, progress::count("commits"));
                    let sc = sp.counter();

                    let mut cp = progress.add_child("find changes");
                    cp.init(None, progress::count("modified files"));
                    let cc = cp.counter();

                    let mut lp = progress.add_child("find changes");
                    lp.init(None, progress::count("diff lines"));
                    let lc = lp.counter();

                    (Some((sp, cp, lp)), Some((sc, cc, lc)))
                })
                .unwrap_or_default();

            let mut progress = progress.add_child("traverse commit graph");
            progress.init(None, progress::count("commits"));

            let (tx_tree_id, stat_threads) = needs_stats
                .then(|| {
                    let (tx, threads) = spawn_tree_delta_threads(
                        scope,
                        threads,
                        line_stats,
                        repo.clone(),
                        stats_counters.clone().expect("counters are set"),
                    );
                    (Some(tx), threads)
                })
                .unwrap_or_default();

            let mut commit_idx = 0_u32;
            let mut skipped_merge_commits = 0;
            const CHUNK_SIZE: usize = 50;
            let mut chunk = Vec::with_capacity(CHUNK_SIZE);
            let mut commit_iter = commit_id.ancestors(&repo.objects);
            let mut is_shallow = false;
            while let Some(c) = commit_iter.next() {
                progress.inc();
                if gix::interrupt::is_triggered() {
                    bail!("Cancelled by user");
                }
                match c {
                    Ok(c) => {
                        tx.send((commit_idx, commit_iter.commit_data().to_owned())).ok();
                        let tree_delta_info = tx_tree_id.as_ref().and_then(|tx| {
                            let mut parents = c.parent_ids.into_iter();
                            parents
                                .next()
                                .map(|first_parent| (tx, Some(first_parent), c.id.to_owned()))
                                .filter(|_| {
                                    if parents.next().is_some() {
                                        skipped_merge_commits += 1;
                                        false
                                    } else {
                                        true
                                    }
                                })
                        });
                        if let Some((tx_tree, first_parent, commit)) = tree_delta_info {
                            if chunk.len() == CHUNK_SIZE {
                                tx_tree
                                    .send(std::mem::replace(&mut chunk, Vec::with_capacity(CHUNK_SIZE)))
                                    .ok();
                            } else {
                                chunk.push((commit_idx, first_parent, commit));
                            }
                        }
                        commit_idx += 1;
                    }
                    Err(gix::traverse::commit::simple::Error::Find { .. }) => {
                        is_shallow = true;
                        break;
                    }
                    Err(err) => return Err(err.into()),
                }
            }
            if let Some(tx) = tx_tree_id {
                tx.send(chunk).ok();
            }
            drop(tx);
            progress.show_throughput(start);
            drop(progress);

            let stats_by_commit_idx = match stats_progresses {
                Some((mut stat_progress, change_progress, line_progress)) => {
                    stat_progress.set_max(Some(commit_idx as usize - skipped_merge_commits));
                    let mut stats = Vec::new();
                    for handle in stat_threads {
                        stats.extend(handle.join().expect("no panic")?);
                        if gix::interrupt::is_triggered() {
                            bail!("Cancelled by user");
                        }
                    }
                    stats.sort_by_key(|t| t.0);
                    stat_progress.show_throughput(start);
                    change_progress.show_throughput(start);
                    line_progress.show_throughput(start);
                    stats
                }
                None => Vec::new(),
            };

            Ok((
                extract_signatures.join().expect("no panic")?,
                stats_by_commit_idx,
                is_shallow,
                skipped_merge_commits,
            ))
        })?
    };

    if commit_authors.is_empty() {
        bail!("No commits to process");
    }

    let start = Instant::now();
    let mut current_email = &commit_authors[0].1.email;
    let mut slice_start = 0;
    let mut results_by_hours = Vec::new();
    let mut ignored_bot_commits = 0_u32;
    for (idx, (_, elm)) in commit_authors.iter().enumerate() {
        if elm.email != *current_email {
            let estimate = estimate_hours(&commit_authors[slice_start..idx], &stats);
            slice_start = idx;
            current_email = &elm.email;
            if ignore_bots && estimate.name.contains_str(b"[bot]") {
                ignored_bot_commits += estimate.num_commits;
                continue;
            }
            results_by_hours.push(estimate);
        }
    }
    if let Some(commits) = commit_authors.get(slice_start..) {
        results_by_hours.push(estimate_hours(commits, &stats));
    }

    let num_authors = results_by_hours.len();
    let mut results_by_hours = if !omit_unify_identities {
        deduplicate_identities(&results_by_hours)
    } else {
        results_by_hours
            .iter()
            .fold(Vec::with_capacity(results_by_hours.len()), |mut acc, e| {
                acc.push(e.into());
                acc
            })
    };
    let elapsed = start.elapsed();
    progress.done(format!(
        "Extracted and organized data from {} commits in {:?} ({:0.0} commits/s)",
        commit_authors.len(),
        elapsed,
        commit_authors.len() as f32 / elapsed.as_secs_f32()
    ));

    let num_unique_authors = results_by_hours.len();
    let (total_hours, total_commits, total_files, total_lines) = results_by_hours
        .iter()
        .map(|e| (e.hours, e.num_commits, e.files, e.lines))
        .reduce(|a, b| (a.0 + b.0, a.1 + b.1, a.2.clone().added(&b.2), a.3.clone().added(&b.3)))
        .expect("at least one commit at this point");
    if show_pii {
        results_by_hours.sort_by(|a, b| a.hours.partial_cmp(&b.hours).unwrap_or(std::cmp::Ordering::Equal));
        for entry in &results_by_hours {
            entry.write_to(
                total_hours,
                file_stats.then_some(total_files),
                line_stats.then_some(total_lines),
                &mut out,
            )?;
            writeln!(out)?;
        }
    }
    writeln!(
        out,
        "total hours: {:.02}\ntotal 8h days: {:.02}\ntotal commits = {}{}\ntotal authors: {}",
        total_hours,
        total_hours / HOURS_PER_WORKDAY,
        total_commits,
        is_shallow.then_some(" (shallow)").unwrap_or_default(),
        num_authors
    )?;
    if file_stats {
        writeln!(
            out,
            "total files added/removed/modified/remaining: {}/{}/{}/{}",
            total_files.added,
            total_files.removed,
            total_files.modified,
            total_files.added - total_files.removed
        )?;
    }
    if line_stats {
        writeln!(
            out,
            "total lines added/removed/remaining: {}/{}/{}",
            total_lines.added,
            total_lines.removed,
            total_lines.added - total_lines.removed
        )?;
    }
    if !omit_unify_identities {
        writeln!(
            out,
            "total unique authors: {} ({:.02}% duplication)",
            num_unique_authors,
            (1.0 - (num_unique_authors as f32 / num_authors as f32)) * 100.0
        )?;
    }
    if ignored_bot_commits != 0 {
        writeln!(out, "commits by bots: {ignored_bot_commits}")?;
    }
    if needs_stats && skipped_merge_commits != 0 {
        writeln!(out, "stats omitted for {skipped_merge_commits} merge commits")?;
    }
    assert_eq!(
        total_commits,
        commit_authors.len() as u32 - ignored_bot_commits,
        "need to get all commits"
    );
    Ok(())
}

mod core;
use self::core::{deduplicate_identities, estimate_hours, HOURS_PER_WORKDAY};

mod util;
use util::{CommitIdx, FileStats, LineStats, WorkByEmail, WorkByPerson};

use crate::hours::core::spawn_tree_delta_threads;

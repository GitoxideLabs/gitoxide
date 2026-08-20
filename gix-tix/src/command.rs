use std::{
    collections::HashSet,
    ffi::{OsStr, OsString},
    io::Write,
    path::PathBuf,
    sync::atomic::AtomicBool,
};

use anyhow::{Context, Result};
use clap::Parser;
use ratatui::text::Line;

mod enrich;
mod new;
mod rebase;
mod reword;
mod travel;

/// Arguments and commands shared by the standalone `tix` binary and `gix tix`.
#[derive(Debug, clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Platform {
    /// Exit once all commits and graph lanes have been computed.
    #[arg(long)]
    quit_on_finish: bool,
    /// Hide this revision and every commit reachable from it.
    #[arg(short = 'x', long, value_name = "REVSPEC")]
    hide: Vec<OsString>,
    #[command(subcommand)]
    command: Option<Command>,
    /// Revisions whose reachable commits should be shown, or HEAD if omitted.
    revisions: Vec<OsString>,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    /// Print the ref-tree of non-hidden references without opening the terminal UI.
    RefTree(RefTree),
    /// Print the complete history view without opening the terminal UI.
    #[command(visible_alias = "status")]
    Show(Show),
    /// Perform repository maintenance.
    #[command(subcommand)]
    Admin(Admin),
    /// Manage commit and tree enrichments.
    #[command(subcommand)]
    Enrich(enrich::Command),
    /// Add staged changes, or worktree changes when nothing is staged, to HEAD.
    Amend(Amend),
    /// Move the changes introduced by HEAD into the worktree.
    Spill,
    /// Split HEAD by amending worktree changes into it and committing staged index changes on top.
    Split(Split),
    /// Save index and worktree changes in a gix stash associated with the HEAD commit.
    Stash,
    /// Pin one or more commits as persistent history tips.
    Pin(Pin),
    /// Copy the change introduced by one commit above another commit.
    CopyInsert(CopyInsert),
    /// Travel to a commit while preserving reachable history through tix pins.
    Travel(travel::Args),
    /// Edit a commit and lazily rebase every descendant retained by a tix pin.
    Reword(reword::Args),
    /// Create a new commit at HEAD.
    New(new::Args),
    /// Generate or apply a self-contained history-rebase todo.
    #[command(subcommand)]
    Rebase(rebase::Command),
}

#[derive(Debug, clap::Subcommand)]
enum Admin {
    /// Clear this worktree's undo and redo history.
    ClearUndo,
}

#[derive(Debug, clap::Args)]
struct RefTree {
    /// Omit tags as labels, traversal tips, and topology anchors.
    #[arg(long)]
    no_tags: bool,
    /// Hide this revision and every commit reachable from it.
    #[arg(short = 'x', long, value_name = "REVSPEC")]
    hide: Vec<OsString>,
    /// Do not infer hidden local branches from remote HEADs.
    #[arg(long)]
    no_auto_hide: bool,
    /// Use the ref-tree view's Unicode line and node glyphs instead of ASCII.
    #[arg(long)]
    unicode: bool,
    /// Revisions to traverse instead of all normal references.
    #[arg(value_name = "REVSPEC")]
    revisions: Vec<OsString>,
}

#[derive(Debug, clap::Args)]
struct Show {
    /// Hide this revision and every commit reachable from it.
    #[arg(short = 'x', long, value_name = "REVSPEC")]
    hide: Vec<OsString>,
    /// Do not infer hidden local branches from remote HEADs.
    #[arg(long)]
    no_auto_hide: bool,
    /// Visible traversal tips, or HEAD if omitted.
    #[arg(value_name = "TIP")]
    revisions: Vec<OsString>,
}

#[derive(Debug, clap::Args)]
struct Amend {
    /// Amend only staged index changes, without falling back to worktree changes.
    #[arg(long)]
    index: bool,
}

#[derive(Debug, clap::Args)]
struct Split {
    /// Mark the new upper commit as TODO.
    #[arg(long)]
    todo: bool,
}

#[derive(Debug, clap::Args)]
struct Pin {
    /// Revisions resolving to commits to pin.
    #[arg(required = true, value_name = "REVSPEC")]
    revisions: Vec<OsString>,
}

#[derive(Debug, clap::Args)]
#[command(
    after_long_help = "Conflicts change nothing by default. To materialize one and write a continuation todo:\n  tix copy-insert --materialize-conflicts=todo.continue.md C I\nResolve the index, then run:\n  tix rebase apply todo.continue.md\nUse --materialize-conflicts=- to write a continuation to non-terminal stdout."
)]
struct CopyInsert {
    /// On conflict, materialize it and write a continuation todo to FILE, or stdout if omitted or '-'.
    #[arg(
        long,
        value_name = "CONTINUE",
        num_args = 0..=1,
        default_missing_value = "-",
        require_equals = true
    )]
    materialize_conflicts: Option<PathBuf>,
    /// Revision resolving to the commit to copy.
    #[arg(value_name = "SOURCE")]
    source: OsString,
    /// Revision resolving to the commit above which to insert the copy.
    #[arg(value_name = "TARGET")]
    target: OsString,
}

#[derive(Debug, clap::Parser)]
#[command(
    name = "tix",
    about = "Browse or edit commit history",
    after_long_help = "Commands which open an editor use Git's normal editor selection. Set GIT_EDITOR=<command> to override it."
)]
struct Cli {
    #[command(flatten)]
    platform: Platform,
}

/// Parse the standalone `tix` command line.
pub fn parse() -> Platform {
    Cli::parse_from(gix::env::args_os()).platform
}

impl Platform {
    /// Run this command against `repository`.
    pub fn run(self, repository: gix::ThreadSafeRepository) -> Result<()> {
        let Platform {
            quit_on_finish,
            hide,
            command,
            revisions,
        } = self;
        let Some(command) = command else {
            return crate::run(repository, revisions, crate::Options { quit_on_finish, hide });
        };

        let repository = repository.to_thread_local();
        let command = match command {
            Command::RefTree(args) => return print_ref_tree(&repository, args),
            Command::Show(args) => return show(&repository, args),
            command => command,
        };
        let _log_guard = crate::logging::init();
        match command {
            Command::RefTree(_) | Command::Show(_) => unreachable!("display commands return before logging"),
            Command::Amend(args) => {
                let graph = crate::edit::loaded_view_graph(&repository)?;
                let output_repository = repository.clone();
                let amended = if args.index {
                    crate::edit::head::amend_index_reporting(repository, &graph)?
                } else {
                    crate::edit::head::perform_reporting(repository, &graph, crate::edit::head::Kind::Amend)?
                };
                match amended {
                    Some(outcome) => {
                        let selected = outcome.selected.context("amending did not produce a selection")?;
                        println!("{}", crate::change_id::display(&output_repository, selected, 7)?);
                        print_ref_rewrites(&output_repository, &outcome.ref_rewrites)?;
                        record_undo(&output_repository, "amend", Ok(outcome.ref_changes));
                    }
                    None => println!("nothing to amend"),
                }
            }
            Command::Spill => {
                let graph = crate::edit::loaded_view_graph(&repository)?;
                edit_head(repository, &graph, crate::edit::head::Kind::Spill, "spill")?;
            }
            Command::Split(args) => {
                let graph = crate::edit::loaded_view_graph(&repository)?;
                split(repository, &graph, args)?;
            }
            Command::Stash => {
                let id = repository
                    .head_id()
                    .context("stashing changes requires a born HEAD")?
                    .detach();
                let notice = crate::edit::stash::save_manual(repository.git_dir(), repository.is_bare(), id)?;
                println!("{}", notice_with_change_id(&repository, &notice, id)?);
            }
            Command::Pin(args) => pin(&repository, args)?,
            Command::CopyInsert(args) => return copy_insert(repository, args),
            Command::Admin(Admin::ClearUndo) => crate::edit::undo::clear(&repository)?,
            Command::Travel(args) => return travel::run(repository, args),
            Command::Reword(args) => return reword::run(repository, args),
            Command::New(args) => return new::run(repository, args),
            Command::Enrich(command) => return enrich::run(repository, command),
            Command::Rebase(command) => return rebase::run(repository, command),
        }
        Ok(())
    }
}

fn print_ref_tree(repository: &gix::Repository, args: RefTree) -> Result<()> {
    let rendered = render_ref_tree(repository, args)?;
    std::io::Write::write_all(&mut std::io::stdout().lock(), rendered.as_bytes())
        .context("could not write ref-tree")?;
    Ok(())
}

fn render_ref_tree(repository: &gix::Repository, args: RefTree) -> Result<String> {
    let (hide, unavailable) = crate::history::available_hidden_revisions(repository, &args.hide, !args.no_auto_hide)?;
    for (revision, err) in unavailable {
        eprintln!(
            "warning: ignoring unavailable hidden revision {}: {err}",
            revision.to_string_lossy()
        );
    }
    let revisions = if args.revisions.is_empty() {
        crate::history::ref_tree_revisions(repository, !args.no_tags)?
    } else {
        args.revisions
    };
    crate::ref_tree::render_full(repository, &revisions, &hide, !args.no_tags, args.unicode)
}

fn show(repository: &gix::Repository, args: Show) -> Result<()> {
    let (hide, unavailable) = crate::history::available_hidden_revisions(repository, &args.hide, !args.no_auto_hide)?;
    if hide.is_empty() {
        anyhow::bail!("show requires at least one -x/--hide revision when no remote HEAD maps to a local branch");
    }
    for (revision, err) in unavailable {
        eprintln!(
            "warning: ignoring unavailable hidden revision {}: {err}",
            revision.to_string_lossy()
        );
    }
    write_history(repository, &args.revisions, &hide, std::io::stdout().lock())
}

fn write_history(
    repository: &gix::Repository,
    revisions: &[OsString],
    hide: &[OsString],
    mut out: impl Write,
) -> Result<()> {
    let authors = gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(
        crate::history::Authors::default(),
    ));
    let refs = crate::history::snapshot(repository, revisions, hide, false)?;
    let mut app = crate::app::App::new(usize::MAX);
    app.id_mode = crate::app::IdMode::Commit;
    let mut decorations = crate::history::Decorations::default();
    let mut history_graph = None;
    crate::history::load(
        repository,
        revisions,
        hide,
        false,
        &authors,
        &AtomicBool::new(false),
        |event| {
            match event {
                crate::history::Event::Decorations(value) => decorations = value,
                crate::history::Event::Commits(rows) => app.extend_commits(rows),
                crate::history::Event::HiddenCommits(rows) => app.extend_hidden_commits(rows),
                crate::history::Event::Complete(graph) => history_graph = Some(graph),
                crate::history::Event::VisibleComplete | crate::history::Event::Cancelled => {}
            }
            true
        },
    )?;
    let graph = history_graph.context("history traversal did not complete")?;
    let rows = app
        .start_lane_computation()
        .context("history rows were unavailable for lane computation")?;
    let (rows, lanes, elapsed) = crate::app::compute_lanes(rows);
    app.finish_lane_computation(rows, lanes, elapsed);
    crate::update_hidden_branch_updates(&mut app, Some(&graph), &refs);

    for index in 0..app.rows.len() {
        if app.rows[index].metadata_loaded {
            continue;
        }
        let id = app.rows[index].id;
        let (metadata, attributions) =
            crate::history::load_metadata(repository, id, &authors).context("could not load displayed commit")?;
        app.set_metadata(index, metadata, attributions);
    }

    let mut note_ids = HashSet::new();
    let mut notes = repository.notes().context("could not open Git notes")?;
    for row in &app.rows {
        if !notes
            .get(row.id)
            .context("could not load displayed commit notes")?
            .is_empty()
        {
            note_ids.insert(row.id);
        }
    }

    let mut todo_ids = HashSet::new();
    let mut enrichment_note_ids = HashSet::new();
    let mut enrichments = crate::enrich::open(repository)?;
    for row in &app.rows {
        let loaded = crate::change_id::for_commit(repository, row.id)
            .and_then(|change_id| crate::enrich::load(&mut enrichments, change_id));
        match loaded {
            Ok(enrichment) => {
                if enrichment.todo {
                    todo_ids.insert(row.id);
                }
                if enrichment.note.is_some() {
                    enrichment_note_ids.insert(row.id);
                }
            }
            Err(err) => tracing::warn!(commit_id = %row.id, error = %err, "ignored malformed tix enrichment"),
        }
    }

    let mut checks_pass_ids = HashSet::new();
    let mut tree_enrichments = crate::enrich::open_tree(repository)?;
    for row in &app.rows {
        let loaded = crate::enrich::tree_id(repository, row.id)
            .and_then(|tree_id| crate::enrich::load_tree(&mut tree_enrichments, tree_id));
        match loaded {
            Ok(enrichment) if enrichment.checks_pass => {
                checks_pass_ids.insert(row.id);
            }
            Ok(_) => {}
            Err(err) => tracing::warn!(commit_id = %row.id, error = %err, "ignored malformed tix tree enrichment"),
        }
    }

    let change_ids = crate::change_id::abbreviations(repository, app.rows.iter().map(|row| row.id), 7)?;

    let mailmap = repository.open_mailmap();
    let lanes = app.render_lanes(0..app.rows.len());
    let enrichment_gutter = app
        .rows
        .iter()
        .map(|row| {
            Line::raw(crate::enrich::marker(
                todo_ids.contains(&row.id),
                enrichment_note_ids.contains(&row.id),
                checks_pass_ids.contains(&row.id),
            ))
            .width()
        })
        .max()
        .unwrap_or_default();
    let ambiguity_gutter = (!change_ids.ambiguous.is_empty()).then(|| Line::raw("💥").width());
    let render_line = |index: usize, row: &crate::app::SharedCommitRow| {
        let metadata = crate::ui::plain_history_metadata(
            &app,
            row,
            &decorations,
            &mailmap,
            note_ids.contains(&row.id),
            change_ids.values.get(&row.id).copied(),
        );
        let enrichment_marker = crate::enrich::marker(
            todo_ids.contains(&row.id),
            enrichment_note_ids.contains(&row.id),
            checks_pass_ids.contains(&row.id),
        );
        let ambiguity_marker = if change_ids.ambiguous.contains(&row.id) {
            "💥"
        } else {
            ""
        };
        let mut gutter = String::new();
        if enrichment_gutter != 0 {
            gutter.push_str(enrichment_marker);
            gutter.push_str(&" ".repeat(enrichment_gutter.saturating_sub(Line::raw(enrichment_marker).width())));
        }
        if let Some(width) = ambiguity_gutter {
            gutter.push_str(ambiguity_marker);
            gutter.push_str(&" ".repeat(width.saturating_sub(Line::raw(ambiguity_marker).width())));
        }
        let behind = app
            .hidden_branch_behind(row.id)
            .map(|behind| format!(" ⇣{behind}"))
            .unwrap_or_default();
        let line = format!("{gutter}{}{metadata}{behind}", lanes.lane(index));
        let base = (app.visual_count(index) == Some(0))
            .then(|| format!("base {enrichment_marker}{ambiguity_marker}{metadata}{behind}"));
        (line, base)
    };
    let width = app
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let (line, base) = render_line(index, row);
            base.as_ref()
                .map_or_else(|| Line::raw(&line).width(), |base| Line::raw(base).width() + 10)
        })
        .max()
        .unwrap_or_default();
    for (index, row) in app.rows.iter().enumerate() {
        let (line, base) = render_line(index, row);
        if let Some(base) = base {
            let rails = width.saturating_sub(Line::raw(&base).width() + 2).max(8);
            let left = rails / 2;
            writeln!(out, "{} {base} {}", "─".repeat(left), "─".repeat(rails - left))
                .context("could not write history base")?;
        } else {
            writeln!(out, "{line}").context("could not write history row")?;
        }
    }
    Ok(())
}

fn resolve_commit(
    repository: &gix::Repository,
    revision: &OsStr,
    description: &str,
) -> Result<(gix::ObjectId, Option<crate::history::HistoryGraph>)> {
    let revision = gix::path::os_str_into_bstr(revision)
        .with_context(|| format!("revision {} is not valid UTF-8", revision.to_string_lossy()))?;
    match crate::history::resolve_revision(repository, revision) {
        Ok((id, _reference)) => Ok((id, None)),
        Err(revision_error) => {
            let graph = crate::edit::loaded_view_graph(repository)?;
            let resolved = std::str::from_utf8(revision)
                .ok()
                .map(|prefix| crate::change_id::resolve_prefix(repository, prefix, graph.stored_commit_ids()))
                .transpose()?
                .flatten();
            match resolved {
                Some(id) => Ok((id, Some(graph))),
                None => Err(revision_error).with_context(|| format!("could not resolve {description} {revision:?}")),
            }
        }
    }
}

fn copy_insert(repository: gix::Repository, args: CopyInsert) -> Result<()> {
    repository.workdir().context("copy-insert requires a worktree")?;
    let (source, _) = resolve_commit(&repository, &args.source, "copy source")?;
    let (target, _) = resolve_commit(&repository, &args.target, "copy target")?;
    let revisions = [
        OsString::from("HEAD"),
        OsString::from(source.to_string()),
        OsString::from(target.to_string()),
    ];
    let graph = crate::edit::loaded_view_graph_with(&repository, &revisions)?;
    let plan = crate::edit::rebase::copy_insert_plan(&repository, &graph, source, target)?;
    let repository_path = repository.git_dir().to_owned();
    let bare = repository.is_bare();
    match crate::edit::rebase::perform_plan(&repository, &graph, plan)? {
        crate::edit::rebase::PlanPerform::Complete(outcome) => {
            let copied = outcome.selected.context("copy-insert did not produce a selection")?;
            let (_, changes) =
                match crate::edit::time_travel::checkout_plan_reporting(&repository_path, bare, &outcome, &[], false) {
                    Ok(result) => result,
                    Err(err) => {
                        record_undo(&repository, "copy-insert commit", Ok(outcome.ref_changes));
                        return Err(err).context("copy-insert applied, but could not check out the copied commit");
                    }
                };
            println!("{}", crate::change_id::display(&repository, copied, 7)?);
            print_ref_rewrites(&repository, &outcome.ref_rewrites)?;
            record_undo(&repository, "copy-insert commit", Ok(changes));
            Ok(())
        }
        crate::edit::rebase::PlanPerform::Conflict(conflict) => rebase::handle_plan_conflict(
            &repository,
            conflict,
            args.materialize_conflicts.as_deref(),
            &[],
            "copy-insert",
        ),
    }
}

fn pin(repository: &gix::Repository, args: Pin) -> Result<()> {
    let pins = create_pins(repository, &args.revisions)?;
    for (pin, _created) in &pins {
        println!("{}", display_pin(repository, pin)?);
    }
    record_undo(
        repository,
        "pin commit",
        Ok(pins
            .into_iter()
            .filter_map(|(pin, created)| {
                created.then_some(crate::edit::undo::RefChange {
                    name: pin.name,
                    before: crate::edit::undo::State::Missing,
                    after: match pin.target {
                        gix::refs::Target::Object(id) => crate::edit::undo::State::Object(id),
                        gix::refs::Target::Symbolic(name) => crate::edit::undo::State::Symbolic(name),
                    },
                })
            })
            .collect()),
    );
    Ok(())
}

fn create_pins(repository: &gix::Repository, revisions: &[OsString]) -> Result<Vec<(crate::history::Pin, bool)>> {
    let mut seen = HashSet::new();
    let targets = revisions
        .iter()
        .map(|revision| {
            let revision = gix::path::os_str_into_bstr(revision)
                .with_context(|| format!("revision {} is not valid UTF-8", revision.to_string_lossy()))?;
            let (id, reference) = crate::history::resolve_revision(repository, revision)
                .with_context(|| format!("could not resolve revision {revision:?}"))?;
            let target = match reference {
                Some(reference) if repository.find_reference(reference.as_ref())?.peel_to_commit()?.id == id => {
                    gix::refs::Target::Symbolic(reference)
                }
                _ => gix::refs::Target::Object(id),
            };
            Ok((target, id))
        })
        .collect::<Result<Vec<_>>>()?;
    targets
        .into_iter()
        .filter(|(target, _id)| seen.insert(target.clone()))
        .map(|(target, id)| crate::edit::time_travel::create_or_reuse_pin(repository, target, id, "tix pin"))
        .collect()
}

fn display_pin(repository: &gix::Repository, pin: &crate::history::Pin) -> Result<String> {
    Ok(format!(
        "{} {}",
        crate::edit::time_travel::pin_label(pin),
        crate::change_id::display_short(repository, pin.id)?
    ))
}

fn edit_head(
    repository: gix::Repository,
    graph: &crate::history::HistoryGraph,
    kind: crate::edit::head::Kind,
    verb: &str,
) -> Result<()> {
    let output_repository = repository.clone();
    match crate::edit::head::perform_reporting(repository, graph, kind)? {
        Some(outcome) => {
            let selected = outcome.selected.context("editing HEAD did not produce a selection")?;
            println!("{}", crate::change_id::display(&output_repository, selected, 7)?);
            print_ref_rewrites(&output_repository, &outcome.ref_rewrites)?;
            record_undo(&output_repository, verb, Ok(outcome.ref_changes));
        }
        None => println!("nothing to {verb}"),
    }
    Ok(())
}

fn split(repository: gix::Repository, graph: &crate::history::HistoryGraph, args: Split) -> Result<()> {
    let repository_path = repository.git_dir().to_owned();
    let bare = repository.is_bare();
    let prepared = crate::edit::split::prepare(repository, args.todo)?;
    let Some(edited) = crate::edit::edit_document_without_terminal(
        &prepared.editor,
        &prepared.document,
        &format!("tix-split-{}.md", std::process::id()),
    )?
    else {
        println!("no split performed: no input was provided");
        return Ok(());
    };
    let mut repository = crate::open_repository(&repository_path, bare, false)
        .context("could not reopen repository after editing split")?;
    repository.object_cache_size(None);
    let outcome = crate::edit::split::apply_reporting(repository, graph, prepared, &edited)?;
    let output_repository =
        crate::open_repository(&repository_path, bare, false).context("could not reopen repository after splitting")?;
    let selected = outcome.selected.context("splitting did not produce a selection")?;
    println!("{}", crate::change_id::display(&output_repository, selected, 7)?);
    print_ref_rewrites(&output_repository, &outcome.ref_rewrites)?;
    record_undo(&output_repository, "split commit", Ok(outcome.ref_changes));
    Ok(())
}

pub(super) fn record_undo(
    repository: &gix::Repository,
    title: &str,
    changes: Result<Vec<crate::edit::undo::RefChange>>,
) {
    if let Err(err) = changes.and_then(|changes| crate::edit::undo::record(repository, title, &changes).map(|_| ())) {
        eprintln!("warning: operation completed, but undo history was not updated: {err:#}");
    }
}

fn print_ref_rewrites(repository: &gix::Repository, rewrites: &[crate::edit::rebase::RefRewrite]) -> Result<()> {
    for line in ref_rewrite_lines(repository, rewrites)? {
        println!("{line}");
    }
    Ok(())
}

fn ref_rewrite_lines(
    repository: &gix::Repository,
    rewrites: &[crate::edit::rebase::RefRewrite],
) -> Result<Vec<String>> {
    let mut rewrites = rewrites.to_vec();
    rewrites.sort_by(|a, b| a.name.cmp(&b.name));
    rewrites.dedup();
    rewrites
        .into_iter()
        .map(|rewrite| {
            Ok(format!(
                "{}: {} -> {}",
                rewrite.name,
                crate::change_id::display(repository, rewrite.old, 7)?,
                crate::change_id::display(repository, rewrite.new, 7)?
            ))
        })
        .collect()
}

fn notice_with_change_id(repository: &gix::Repository, notice: &str, id: gix::ObjectId) -> Result<String> {
    let hash = id.to_hex_with_len(7).to_string();
    Ok(notice.replacen(&hash, &crate::change_id::display(repository, id, 7)?, 1))
}

#[cfg(test)]
mod tests {
    use std::{path::Path, process::Command as ProcessCommand};

    use clap::{CommandFactory, error::ErrorKind};

    use super::*;

    #[test]
    fn rewritten_ref_lines_are_sorted_and_show_the_commit_mapping() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = crate::test_repository::open(fixture.path())?;
        let old = repository.rev_parse_single("main~1")?.detach();
        let new = repository.rev_parse_single("main")?.detach();
        let branch = crate::edit::rebase::RefRewrite {
            name: "refs/heads/z".try_into().expect("valid ref name"),
            old,
            new,
        };
        let first = crate::edit::rebase::RefRewrite {
            name: "refs/heads/a".try_into().expect("valid ref name"),
            old,
            new,
        };
        assert_eq!(
            ref_rewrite_lines(&repository, &[branch.clone(), first, branch])?,
            [
                format!(
                    "refs/heads/a: {} -> {}",
                    crate::change_id::display(&repository, old, 7)?,
                    crate::change_id::display(&repository, new, 7)?
                ),
                format!(
                    "refs/heads/z: {} -> {}",
                    crate::change_id::display(&repository, old, 7)?,
                    crate::change_id::display(&repository, new, 7)?
                )
            ],
            "ref mappings are stable and duplicate-free"
        );
        assert!(
            ref_rewrite_lines(&repository, &[])?.is_empty(),
            "unchanged refs add no output"
        );
        Ok(())
    }

    #[test]
    fn commit_notices_pair_their_hash_with_the_change_id() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = crate::test_repository::open(fixture.path())?;
        let id = repository.head_id()?.detach();
        let notice = format!("stashed changes at {}; retained warning", id.to_hex_with_len(7));

        assert_eq!(
            notice_with_change_id(&repository, &notice, id)?,
            format!(
                "stashed changes at {}; retained warning",
                crate::change_id::display(&repository, id, 7)?
            ),
            "the change ID stays adjacent to the hash without disturbing later text"
        );
        Ok(())
    }

    #[test]
    fn clap_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_tui_options_and_top_level_commands() {
        let cli = Cli::try_parse_from(["tix", "--quit-on-finish", "-x", "main", "--hide", "tag", "topic"])
            .expect("TUI arguments parse");
        assert!(cli.platform.quit_on_finish);
        assert_eq!(cli.platform.hide, ["main", "tag"], "hide options append");
        assert_eq!(
            cli.platform.revisions,
            ["topic"],
            "positional revisions remain visible tips"
        );
        assert!(cli.platform.command.is_none(), "omitting a command launches the TUI");

        let ref_tree = Cli::try_parse_from([
            "tix",
            "ref-tree",
            "--no-tags",
            "-x",
            "private",
            "--unicode",
            "main",
            "topic",
        ])
        .expect("ref-tree options parse")
        .platform
        .command;
        let Some(Command::RefTree(ref_tree)) = ref_tree else {
            panic!("ref-tree was expected")
        };
        assert!(ref_tree.no_tags);
        assert_eq!(ref_tree.hide, ["private"]);
        assert!(!ref_tree.no_auto_hide);
        assert!(ref_tree.unicode);
        assert_eq!(ref_tree.revisions, ["main", "topic"]);

        let ref_tree = Cli::try_parse_from(["tix", "ref-tree", "--no-auto-hide"])
            .expect("ref-tree can disable automatic hiding")
            .platform
            .command;
        let Some(Command::RefTree(ref_tree)) = ref_tree else {
            panic!("ref-tree was expected")
        };
        assert!(ref_tree.no_auto_hide);

        let show = Cli::try_parse_from(["tix", "show", "-x", "main", "--hide", "tag", "topic"])
            .expect("show options parse")
            .platform
            .command;
        let Some(Command::Show(show)) = show else {
            panic!("show was expected")
        };
        assert_eq!(show.hide, ["main", "tag"]);
        assert!(!show.no_auto_hide);
        assert_eq!(show.revisions, ["topic"]);

        let show = Cli::try_parse_from(["tix", "show", "--no-auto-hide", "topic"])
            .expect("show can disable automatic hiding")
            .platform
            .command;
        let Some(Command::Show(show)) = show else {
            panic!("show was expected")
        };
        assert!(show.no_auto_hide);
        assert!(show.hide.is_empty());

        assert!(matches!(
            Cli::try_parse_from(["tix", "status", "-x", "main"])
                .expect("status is a visible show alias")
                .platform
                .command,
            Some(Command::Show(_))
        ));
        assert!(
            Cli::command().render_help().to_string().contains("status"),
            "top-level help advertises the status alias"
        );

        for arguments in [
            &["tix", "enrich", "commit", "todo"][..],
            &["tix", "enrich", "commit", "todo", "--clear", "topic"][..],
            &["tix", "enrich", "commit", "note", "topic"][..],
            &["tix", "enrich", "commit", "git-note"][..],
            &["tix", "enrich", "tree", "checks-pass"][..],
            &["tix", "enrich", "tree", "checks-pass", "--clear", "topic"][..],
        ] {
            assert!(
                matches!(
                    Cli::try_parse_from(arguments)
                        .expect("enrich command parses")
                        .platform
                        .command,
                    Some(Command::Enrich(_))
                ),
                "{arguments:?} reaches the enrich command"
            );
        }
        assert!(
            Cli::try_parse_from(["tix", "enrich", "commit", "checks-pass"]).is_err(),
            "tree enrichments are not commit subcommands"
        );
        assert!(
            Cli::try_parse_from(["tix", "enrich", "tree", "todo"]).is_err(),
            "commit enrichments are not tree subcommands"
        );

        assert!(
            Cli::try_parse_from(["tix", "--worktrees"]).is_err(),
            "the removed TUI worktree option is rejected"
        );
        assert!(
            Cli::try_parse_from(["tix", "ref-tree", "-w"]).is_err(),
            "the removed diagnostic worktree option is rejected"
        );

        assert!(
            Cli::try_parse_from(["tix", "ref-tree", "--layout", "rail"]).is_err(),
            "the removed layout selector is rejected"
        );
        let old_name = Cli::try_parse_from(["tix", "tree"])
            .expect("tree remains a valid revision")
            .platform;
        assert!(
            old_name.command.is_none(),
            "the old tree command has no compatibility alias"
        );
        assert_eq!(old_name.revisions, ["tree"]);

        let amend = Cli::try_parse_from(["tix", "amend", "--index"])
            .expect("index-only amend parses")
            .platform
            .command;
        let Some(Command::Amend(amend)) = amend else {
            panic!("amend was expected")
        };
        assert!(amend.index);
        let amend = Cli::try_parse_from(["tix", "amend"])
            .expect("default amend parses")
            .platform
            .command;
        assert!(matches!(amend, Some(Command::Amend(Amend { index: false }))));
        assert!(matches!(
            Cli::try_parse_from(["tix", "spill"])
                .expect("spill parses")
                .platform
                .command,
            Some(Command::Spill)
        ));
        assert!(matches!(
            Cli::try_parse_from(["tix", "split"])
                .expect("split parses")
                .platform
                .command,
            Some(Command::Split(Split { todo: false }))
        ));
        assert!(matches!(
            Cli::try_parse_from(["tix", "split", "--todo"])
                .expect("TODO split parses")
                .platform
                .command,
            Some(Command::Split(Split { todo: true }))
        ));
        assert!(matches!(
            Cli::try_parse_from(["tix", "stash"])
                .expect("stash parses")
                .platform
                .command,
            Some(Command::Stash)
        ));
        assert!(matches!(
            Cli::try_parse_from(["tix", "admin", "clear-undo"])
                .expect("clear-undo parses")
                .platform
                .command,
            Some(Command::Admin(Admin::ClearUndo))
        ));
        let pin = Cli::try_parse_from(["tix", "pin", "main", "HEAD~2"])
            .expect("one or more pin revisions parse")
            .platform
            .command;
        let Some(Command::Pin(pin)) = pin else {
            panic!("pin was expected")
        };
        assert_eq!(pin.revisions, ["main", "HEAD~2"]);
        let copy_insert = Cli::try_parse_from([
            "tix",
            "copy-insert",
            "--materialize-conflicts=continue.md",
            "main",
            "HEAD~1",
        ])
        .expect("copy-insert parses")
        .platform
        .command;
        let Some(Command::CopyInsert(copy_insert)) = copy_insert else {
            panic!("copy-insert was expected")
        };
        assert_eq!(copy_insert.source, "main");
        assert_eq!(copy_insert.target, "HEAD~1");
        assert_eq!(copy_insert.materialize_conflicts, Some("continue.md".into()));
        let Some(Command::CopyInsert(copy_insert)) =
            Cli::try_parse_from(["tix", "copy-insert", "--materialize-conflicts", "HEAD", "main~1"])
                .expect("copy-insert defaults continuation output to stdout")
                .platform
                .command
        else {
            panic!("copy-insert was expected")
        };
        assert_eq!(copy_insert.materialize_conflicts, Some("-".into()));
        let travel = Cli::try_parse_from(["tix", "travel", "--materialize-conflicts", "HEAD~1"])
            .expect("travel parses")
            .platform
            .command;
        let Some(Command::Travel(travel)) = travel else {
            panic!("travel was expected")
        };
        assert!(travel.materialize_conflicts);
        assert_eq!(travel.revision.as_deref(), Some(std::ffi::OsStr::new("HEAD~1")));
        assert_eq!(travel.to, None);
        for (value, expected) in [
            ("first", travel::To::First),
            ("parent", travel::To::Parent),
            ("child", travel::To::Child),
            ("tip", travel::To::Tip),
        ] {
            let parsed = Cli::try_parse_from(["tix", "travel", "--to", value])
                .expect("relative travel parses")
                .platform
                .command;
            let Some(Command::Travel(travel)) = parsed else {
                panic!("travel was expected")
            };
            assert_eq!(travel.revision, None);
            assert_eq!(travel.to, Some(expected));
        }
        assert_eq!(
            Cli::try_parse_from(["tix", "travel"])
                .expect_err("travel requires one destination")
                .kind(),
            ErrorKind::MissingRequiredArgument
        );
        assert_eq!(
            Cli::try_parse_from(["tix", "travel", "HEAD", "--to", "tip"])
                .expect_err("relative and explicit travel are mutually exclusive")
                .kind(),
            ErrorKind::ArgumentConflict
        );
        assert_eq!(
            Cli::try_parse_from(["tix", "travel", "--to", "last"])
                .expect_err("tip is the sole name for the upper endpoint")
                .kind(),
            ErrorKind::InvalidValue
        );
        let travel_help = Cli::try_parse_from(["tix", "travel", "--help"])
            .expect_err("help exits through clap")
            .to_string();
        for description in [
            "Visit the oldest reachable root",
            "Visit HEAD's direct visible parent",
            "Visit HEAD's direct visible child",
            "Visit the reachable leaf",
        ] {
            assert!(
                travel_help.contains(description),
                "travel help describes every relative destination: {travel_help}"
            );
        }
        let reword = Cli::try_parse_from(["tix", "reword", "HEAD~2"])
            .expect("reword parses")
            .platform
            .command;
        let Some(Command::Reword(reword)) = reword else {
            panic!("reword was expected")
        };
        assert_eq!(reword.revision, "HEAD~2");
        assert!(reword.edit.message.is_empty());
        assert!(reword.edit.file.is_none());
        assert!(reword.edit.author.is_none());
        let reword = Cli::try_parse_from([
            "tix",
            "reword",
            "HEAD~2",
            "--author",
            "Agent <agent@example.com>",
            "-m",
            "title",
            "-m",
            "body",
        ])
        .expect("reword messages parse")
        .platform
        .command;
        let Some(Command::Reword(reword)) = reword else {
            panic!("reword was expected")
        };
        assert_eq!(reword.edit.message, ["title", "body"]);
        assert_eq!(
            reword.edit.author.as_deref(),
            Some(std::ffi::OsStr::new("Agent <agent@example.com>"))
        );
        assert!(
            Cli::try_parse_from(["tix", "reword", "HEAD", "-m", "message", "-f", "message.txt"]).is_err(),
            "message and file inputs are mutually exclusive"
        );
        let new = Cli::try_parse_from([
            "tix",
            "new",
            "--index",
            "--allow-empty",
            "--todo",
            "--author",
            "Agent <agent@example.com>",
            "-m",
            "title",
        ])
        .expect("new options parse")
        .platform
        .command;
        let Some(Command::New(new)) = new else {
            panic!("new was expected")
        };
        assert!(new.index);
        assert!(!new.worktree);
        assert!(!new.worktree_untracked);
        assert!(new.allow_empty);
        assert!(new.todo);
        assert_eq!(new.edit.message, ["title"]);
        assert!(Cli::try_parse_from(["tix", "new", "--index", "--worktree", "-m", "title"]).is_err());
        assert!(Cli::try_parse_from(["tix", "new", "--index", "--worktree-untracked", "-m", "title"]).is_err());
        assert!(Cli::try_parse_from(["tix", "new", "--worktree", "--worktree-untracked", "-m", "title"]).is_err());
        assert!(Cli::try_parse_from(["tix", "new", "HEAD", "-m", "title"]).is_err());
        assert!(matches!(
            Cli::try_parse_from([
                "tix",
                "rebase",
                "todo",
                "--no-auto-hide",
                "-x",
                "main",
                "--onto",
                "next",
                "--edit-and-apply",
                "--materialize-conflicts",
                "continue.md",
                "topic"
            ])
            .expect("rebase todo parses")
            .platform
            .command,
            Some(Command::Rebase(rebase::Command::Todo(_)))
        ));
        assert!(matches!(
            Cli::try_parse_from(["tix", "rebase", "todo", "-x", "main", "--update-base", "topic"])
                .expect("rebase update todo parses")
                .platform
                .command,
            Some(Command::Rebase(rebase::Command::Todo(_)))
        ));
        assert!(
            Cli::try_parse_from(["tix", "rebase", "todo", "-x", "main", "--onto", "next", "--update-base"]).is_err(),
            "explicit and inferred rebase targets are mutually exclusive"
        );
        assert!(
            Cli::try_parse_from(["tix", "rebase", "todo", "-x", "main", "--materialize-conflicts"]).is_err(),
            "todo conflict materialization requires immediate editing and application"
        );
        assert!(matches!(
            Cli::try_parse_from(["tix", "rebase", "apply", "-"])
                .expect("rebase apply parses")
                .platform
                .command,
            Some(Command::Rebase(rebase::Command::Apply(_)))
        ));
        let parsed = Cli::try_parse_from([
            "tix",
            "rebase",
            "apply",
            "--materialize-conflicts",
            "continue.md",
            "todo.md",
        ])
        .expect("conflict materialization output parses");
        let Some(Command::Rebase(rebase::Command::Apply(args))) = parsed.platform.command else {
            panic!("rebase apply was expected")
        };
        assert_eq!(
            args.materialize_conflicts.as_deref(),
            Some(std::path::Path::new("continue.md"))
        );
        assert_eq!(args.file.as_deref(), Some(std::path::Path::new("todo.md")));
        assert!(
            Cli::command()
                .render_help()
                .to_string()
                .contains("Split HEAD by amending worktree changes into it and committing staged index changes on top"),
            "short help explains how split distributes index and worktree changes"
        );
        assert!(
            Cli::command()
                .render_help()
                .to_string()
                .contains("gix stash associated with the HEAD commit"),
            "short help distinguishes a commit-associated gix stash"
        );
        assert!(
            Cli::command().render_long_help().to_string().contains("GIT_EDITOR"),
            "top-level help explains how to override Git's editor"
        );
    }

    #[test]
    fn copy_insert_command_rewrites_the_target_stack_and_is_undoable() -> gix_testtools::Result {
        fn git(path: &Path, args: &[&str]) -> gix_testtools::Result<Vec<u8>> {
            let output = ProcessCommand::new("git").arg("-C").arg(path).args(args).output()?;
            if !output.status.success() {
                return Err(format!(
                    "git {} failed: {}",
                    args.join(" "),
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }
            Ok(output.stdout)
        }

        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let path = fixture.path();
        git(path, &["checkout", "-q", "-b", "destination", "HEAD~2"])?;
        std::fs::write(path.join("destination"), b"target\n")?;
        git(path, &["add", "destination"])?;
        git(path, &["commit", "-q", "-m", "destination target"])?;
        let target = crate::test_repository::open(path)?.head_id()?.detach();
        std::fs::write(path.join("destination-child"), b"child\n")?;
        git(path, &["add", "destination-child"])?;
        git(path, &["commit", "-q", "-m", "destination child"])?;
        let destination_before = crate::test_repository::open(path)?.head_id()?.detach();
        git(path, &["checkout", "-q", "-b", "excluded", &target.to_string()])?;
        std::fs::write(path.join("excluded"), b"excluded\n")?;
        git(path, &["add", "excluded"])?;
        git(path, &["commit", "-q", "-m", "excluded child"])?;
        let excluded_before = crate::test_repository::open(path)?.head_id()?.detach();
        git(path, &["checkout", "-q", "destination"])?;

        let repository = crate::test_repository::open(path)?;
        let source = repository.rev_parse_single("main")?.detach();
        copy_insert(
            repository,
            CopyInsert {
                materialize_conflicts: None,
                source: "main".into(),
                target: target.to_string().into(),
            },
        )?;

        let repository = crate::test_repository::open(path)?;
        let copied = repository.head_id()?.detach();
        assert!(repository.head()?.is_detached(), "the new copy is checked out detached");
        assert_eq!(
            repository.find_reference("refs/heads/main")?.id().detach(),
            source,
            "the source branch remains at the original occurrence"
        );
        assert_eq!(
            repository.find_reference("refs/heads/excluded")?.id().detach(),
            excluded_before,
            "an unpinned branch outside the command view is not rewritten"
        );
        assert_eq!(
            repository.find_commit(copied)?.parent_ids().next().map(gix::Id::detach),
            Some(target),
            "the copy is inserted immediately above the target"
        );
        let destination_after = repository.find_reference("refs/heads/destination")?.id().detach();
        assert_ne!(
            destination_after, destination_before,
            "the target descendant is rewritten"
        );
        assert_eq!(
            repository
                .find_commit(destination_after)?
                .parent_ids()
                .next()
                .map(gix::Id::detach),
            Some(copied),
            "the rewritten descendant follows the copy"
        );
        assert_eq!(
            crate::edit::undo::position(&repository)?.title,
            "copy-insert commit",
            "the command records one undoable operation"
        );
        let pins = crate::history::all_pins(&repository)?;
        assert_eq!(pins.len(), 1, "the previous checkout receives one HEAD pin");
        assert_eq!(
            pins[0].target.try_name().map(gix::refs::FullNameRef::as_bstr),
            Some(gix::bstr::BStr::new(b"refs/heads/destination")),
            "the HEAD pin remembers the destination branch"
        );

        crate::edit::undo::plan_undo(&repository)?
            .context("copy-insert can be undone")?
            .apply(&repository)?;
        let repository = crate::test_repository::open(path)?;
        assert_eq!(
            repository.head_id()?.detach(),
            destination_before,
            "undo restores the previous checkout"
        );
        assert_eq!(
            repository.head()?.referent_name().map(ToOwned::to_owned),
            Some("refs/heads/destination".try_into().expect("valid branch name")),
            "undo reattaches HEAD"
        );
        assert_eq!(
            repository.find_reference("refs/heads/destination")?.id().detach(),
            destination_before,
            "undo restores the target branch"
        );
        assert_eq!(repository.find_reference("refs/heads/main")?.id().detach(), source);
        assert_eq!(
            repository.find_reference("refs/heads/excluded")?.id().detach(),
            excluded_before
        );
        assert!(
            crate::history::all_pins(&repository)?.is_empty(),
            "undo removes the checkout pin"
        );
        Ok(())
    }

    #[test]
    fn copy_insert_conflicts_are_atomic_or_materialize_a_continuation() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_conflict.sh")?;
        let path = fixture.path();
        let repository = crate::test_repository::open(path)?;
        let source = repository.head_id()?.detach();
        let target = repository.rev_parse_single("HEAD~2")?.detach();
        let before = gix_testtools::repository::snapshot(path)?;
        let err = copy_insert(
            repository,
            CopyInsert {
                materialize_conflicts: None,
                source: source.to_string().into(),
                target: target.to_string().into(),
            },
        )
        .expect_err("copy-insert conflicts are atomic by default");
        assert!(format!("{err:#}").contains("pass --materialize-conflicts"));
        assert_eq!(
            gix_testtools::repository::snapshot(path)?,
            before,
            "the default conflict leaves the repository unchanged"
        );

        let output_dir = gix_testtools::tempfile::tempdir()?;
        let continuation = output_dir.path().join("continue.md");
        let repository = crate::test_repository::open(path)?;
        let err = copy_insert(
            repository,
            CopyInsert {
                materialize_conflicts: Some(continuation.clone()),
                source: source.to_string().into(),
                target: target.to_string().into(),
            },
        )
        .expect_err("materializing a conflict exits unsuccessfully");
        assert!(format!("{err:#}").contains("copy-insert stopped at a materialized conflict"));
        let document = std::fs::read(&continuation)?;
        let repository = crate::test_repository::open(path)?;
        assert!(
            crate::edit::todo::parse(&repository, &document)?.is_some(),
            "the continuation is accepted by the ordinary rebase parser"
        );
        assert_eq!(
            crate::edit::undo::position(&repository)?.title,
            "materialize rebase conflict",
            "materialization is independently undoable"
        );
        let unresolved = ProcessCommand::new("git")
            .arg("-C")
            .arg(path)
            .args(["diff", "--name-only", "--diff-filter=U"])
            .output()?;
        assert!(unresolved.status.success());
        assert_eq!(
            unresolved.stdout, b"file\n",
            "materialization writes the unmerged index"
        );

        std::fs::write(path.join("file"), b"base\n")?;
        assert!(
            ProcessCommand::new("git")
                .arg("-C")
                .arg(path)
                .args(["add", "file"])
                .status()?
                .success()
        );
        rebase::run(
            crate::test_repository::open(path)?,
            rebase::Command::Apply(rebase::Apply {
                materialize_conflicts: None,
                file: Some(continuation),
            }),
        )?;
        let unresolved = ProcessCommand::new("git")
            .arg("-C")
            .arg(path)
            .args(["diff", "--name-only", "--diff-filter=U"])
            .output()?;
        assert!(unresolved.status.success());
        assert!(
            unresolved.stdout.is_empty(),
            "the continuation consumes the resolved index"
        );
        Ok(())
    }

    #[test]
    fn copy_insert_rejects_a_bare_repository_before_rewriting_it() -> gix_testtools::Result {
        let source = gix_testtools::scripted_fixture_read_only("rebase_edit.sh")?;
        let fixture = gix_testtools::tempfile::tempdir()?;
        assert!(
            ProcessCommand::new("git")
                .args(["clone", "-q", "--bare"])
                .arg(source)
                .arg(fixture.path())
                .status()?
                .success()
        );
        let repository = crate::test_repository::open(fixture.path())?;
        let before = repository.head_id()?.detach();
        let target = repository.rev_parse_single("HEAD~2")?.detach();
        let err = copy_insert(
            repository,
            CopyInsert {
                materialize_conflicts: None,
                source: before.to_string().into(),
                target: target.to_string().into(),
            },
        )
        .expect_err("copy-insert requires a checkout");
        assert!(format!("{err:#}").contains("requires a worktree"));
        assert_eq!(
            crate::test_repository::open(fixture.path())?.head_id()?.detach(),
            before,
            "the attached branch is unchanged"
        );
        Ok(())
    }

    #[test]
    fn ref_tree_omits_explicit_and_inferred_hidden_references() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = crate::test_repository::open(fixture.path())?;
        let topic = repository.rev_parse_single("topic")?.detach();
        repository.reference(
            "refs/heads/visible",
            topic,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "test visible alias",
        )?;

        let args = |hide, no_auto_hide| RefTree {
            no_tags: false,
            hide,
            no_auto_hide,
            unicode: false,
            revisions: Vec::new(),
        };
        let all = render_ref_tree(&repository, args(Vec::new(), true))?;
        let hidden = render_ref_tree(&repository, args(vec!["topic".into()], true))?;
        assert!(all.contains("topic"), "the complete tree includes topic: {all:?}");
        assert!(
            hidden.contains("visible"),
            "a visible ref sharing the target remains: {hidden:?}"
        );
        assert!(
            !hidden.contains("topic"),
            "the explicitly hidden label is gone: {hidden:?}"
        );

        for git_args in [
            ["config", "remote.origin.url", "https://example.com/repo"].as_slice(),
            ["config", "remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*"].as_slice(),
            ["update-ref", "refs/remotes/origin/main", "main"].as_slice(),
            ["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main"].as_slice(),
        ] {
            let status = ProcessCommand::new("git")
                .current_dir(fixture.path())
                .args(git_args)
                .status()?;
            assert!(status.success(), "git {git_args:?} prepares remote HEAD inference");
        }
        let repository = crate::test_repository::open(fixture.path())?;
        let automatic = render_ref_tree(&repository, args(Vec::new(), false))?;
        assert!(
            all.contains("@main"),
            "disabling inference retains the local default: {all:?}"
        );
        assert!(
            !automatic.contains("@main") && automatic.contains("origin/main"),
            "automatic hiding removes only the inferred local ref: {automatic:?}"
        );
        Ok(())
    }

    #[test]
    fn preserves_hide_and_help_semantics() {
        for command in [
            &[][..],
            &["ref-tree"],
            &["show"],
            &["status"],
            &["amend"],
            &["spill"],
            &["split"],
            &["stash"],
            &["pin"],
            &["copy-insert"],
            &["travel"],
            &["reword"],
            &["admin"],
            &["admin", "clear-undo"],
            &["enrich"],
            &["enrich", "commit"],
            &["enrich", "commit", "todo"],
            &["enrich", "commit", "note"],
            &["enrich", "commit", "git-note"],
            &["enrich", "tree"],
            &["enrich", "tree", "checks-pass"],
            &["rebase"],
            &["rebase", "todo"],
            &["rebase", "apply"],
        ] {
            for help in ["-h", "--help"] {
                let arguments = std::iter::once("tix").chain(command.iter().copied()).chain([help]);
                assert_eq!(
                    Cli::try_parse_from(arguments)
                        .expect_err("help exits through clap")
                        .kind(),
                    ErrorKind::DisplayHelp,
                    "{command:?} supports {help}"
                );
            }
        }
        assert_eq!(
            Cli::try_parse_from(["tix", "-x"])
                .expect_err("hide requires a value")
                .kind(),
            ErrorKind::InvalidValue
        );
        assert_eq!(
            Cli::try_parse_from(["tix", "amend", "topic"])
                .expect_err("commands reject TUI arguments")
                .kind(),
            ErrorKind::UnknownArgument
        );
        assert_eq!(
            Cli::try_parse_from(["tix", "pin"])
                .expect_err("pin requires at least one revision")
                .kind(),
            ErrorKind::MissingRequiredArgument
        );

        let cli = Cli::try_parse_from(["tix", "--", "amend"]).expect("-- makes amend a revision");
        assert!(cli.platform.command.is_none());
        assert_eq!(cli.platform.revisions, ["amend"]);
    }

    #[test]
    fn clear_undo_is_worktree_local_and_idempotent() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let linked = gix_testtools::tempfile::tempdir()?;
        let linked_path = linked.path().join("linked");
        assert!(
            ProcessCommand::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["worktree", "add", "-q", "--detach"])
                .arg(&linked_path)
                .arg("topic")
                .status()?
                .success(),
            "git creates the linked worktree"
        );
        let main = crate::test_repository::open(fixture.path())?;
        let linked = crate::test_repository::open(&linked_path)?;
        let retained_ref: gix::refs::FullName = "refs/worktree/tix/admin-clear-test"
            .try_into()
            .expect("valid worktree reference");
        for repository in [&main, &linked] {
            let id = repository.head_id()?.detach();
            repository.reference(
                retained_ref.clone(),
                id,
                gix::refs::transaction::PreviousValue::MustNotExist,
                "test clear-undo",
            )?;
            crate::edit::undo::record(
                repository,
                "create test ref",
                &[crate::edit::undo::RefChange {
                    name: retained_ref.clone(),
                    before: crate::edit::undo::State::Missing,
                    after: crate::edit::undo::State::Object(id),
                }],
            )?;
        }
        let main_position = crate::edit::undo::position(&main)?;
        let linked_target = linked.find_reference(retained_ref.as_ref())?.id().detach();

        Platform {
            quit_on_finish: false,
            hide: Vec::new(),
            command: Some(Command::Admin(Admin::ClearUndo)),
            revisions: Vec::new(),
        }
        .run(linked.into_sync())?;

        let linked = crate::test_repository::open(&linked_path)?;
        assert!(linked.try_find_reference(crate::edit::undo::TIP_REF)?.is_none());
        assert!(linked.try_find_reference(crate::edit::undo::CURSOR_REF)?.is_none());
        assert_eq!(
            linked.find_reference(retained_ref.as_ref())?.id().detach(),
            linked_target,
            "clearing history does not apply or reverse a recorded operation"
        );
        assert_eq!(
            crate::edit::undo::position(&main)?,
            main_position,
            "another worktree keeps its private queue"
        );

        Platform {
            quit_on_finish: false,
            hide: Vec::new(),
            command: Some(Command::Admin(Admin::ClearUndo)),
            revisions: Vec::new(),
        }
        .run(linked.into_sync())?;
        Ok(())
    }

    #[test]
    fn pin_follows_direct_references_and_keeps_derived_revisions_fixed() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = crate::test_repository::open(fixture.path())?;
        let revisions = |values: &[&str]| values.iter().map(OsString::from).collect::<Vec<_>>();
        let main = repository.rev_parse_single("main")?.detach();

        for invalid in ["missing", "main..topic", "HEAD^{tree}"] {
            create_pins(&repository, &revisions(&["main", invalid]))
                .expect_err("a non-commit revision rejects the complete request");
            assert!(
                crate::history::all_pins(&repository)?.is_empty(),
                "resolution failure is unobservable"
            );
        }

        assert!(
            ProcessCommand::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["symbolic-ref", "refs/worktree/tix/pins/follow", "refs/heads/main",])
                .status()?
                .success(),
            "the fixture has a movable symbolic pin"
        );
        let root = repository.rev_parse_single("v1")?.object()?.peel_to_commit()?.id;
        let parent = repository.rev_parse_single("main~1")?.detach();
        let short_main = main.to_hex_with_len(7).to_string();
        let pins = create_pins(&repository, &revisions(&["main", "v1", "main~1", &short_main]))?;
        assert_eq!(
            pins.iter().map(|(pin, _created)| pin.id).collect::<Vec<_>>(),
            [main, root, parent, main],
            "distinct pin targets preserve argument order even when IDs match"
        );
        assert_eq!(
            pins.iter()
                .map(|(pin, _created)| pin.target.try_name().is_some())
                .collect::<Vec<_>>(),
            [true, true, false, false],
            "direct reference names follow symbolically while derived revisions and IDs stay fixed"
        );
        assert_eq!(
            crate::history::all_pins(&repository)?.len(),
            4,
            "the existing branch pin is reused while other semantic targets remain distinct"
        );

        let repeated = create_pins(&repository, &revisions(&["main"]))?;
        assert_eq!(repeated[0].0.name, pins[0].0.name, "an existing symbolic pin is reused");
        assert_eq!(crate::history::all_pins(&repository)?.len(), 4);
        let display = display_pin(&repository, &pins[0].0)?;
        let (label, ids) = display.split_once(' ').context("pin output has a label and IDs")?;
        assert!(label.starts_with("pin:"), "output names the pin");
        assert_eq!(
            ids,
            crate::change_id::display_short(&repository, main)?,
            "output uses matching repository-abbreviated commit and change IDs"
        );
        repository.reference(
            "refs/heads/main",
            parent,
            gix::refs::transaction::PreviousValue::MustExistAndMatch(gix::refs::Target::Object(main)),
            "advance pinned reference",
        )?;
        let followed = crate::history::all_pins(&repository)?
            .into_iter()
            .find(|pin| pin.name == pins[0].0.name)
            .context("the symbolic pin remains")?;
        assert_eq!(followed.id, parent, "the symbolic pin follows the moved branch");
        Ok(())
    }

    #[test]
    fn show_prints_the_complete_plain_history_view() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = crate::test_repository::open(fixture.path())?;
        let err = show(
            &repository,
            Show {
                hide: Vec::new(),
                no_auto_hide: true,
                revisions: Vec::new(),
            },
        )
        .expect_err("disabling auto-hide requires an explicit hidden revision");
        assert!(format!("{err:#}").contains("at least one -x/--hide"));
        let mut rounded = Vec::new();
        write_history(&repository, &[], &[OsString::from("v1")], &mut rounded)?;
        let rounded = String::from_utf8(rounded)?;
        assert!(
            rounded.contains(['╭', '╮', '╰', '╯']) && !rounded.contains(['┌', '┐', '└', '┘']),
            "history graph turns use rounded corners: {rounded:?}"
        );
        create_pins(&repository, &[OsString::from("topic")])?;
        let old_head = repository.head_id()?.detach();
        let parent = repository
            .find_commit(old_head)?
            .parent_ids()
            .next()
            .context("the fixture head has a parent")?
            .detach();
        let mut commit = repository.find_commit(old_head)?.decode()?.into_owned()?;
        commit.extra_headers.push((
            crate::change_id::HEADER.into(),
            crate::change_id::for_commit(&repository, parent)?.to_string().into(),
        ));
        let head = repository.write_object(&commit)?.detach();
        let head_ref = repository
            .head()?
            .referent_name()
            .context("the fixture head is attached")?
            .to_owned();
        repository.reference(
            head_ref,
            head,
            gix::refs::transaction::PreviousValue::ExistingMustMatch(gix::refs::Target::Object(old_head)),
            "test ambiguous change ID",
        )?;
        let mut orphan = repository.find_commit(parent)?.decode()?.into_owned()?;
        orphan.parents.clear();
        orphan.message = "orphan base".into();
        let orphan = repository.write_object(&orphan)?.detach();
        create_pins(&repository, &[OsString::from(orphan.to_string())])?;
        let head_change_id = crate::change_id::for_commit(&repository, head)?;
        assert!(crate::enrich::toggle(&repository, head)?.todo);
        assert!(crate::enrich::toggle_checks_pass(&repository, head)?.checks_pass);

        let mut output = Vec::new();
        write_history(&repository, &[], &[OsString::from("v1")], &mut output)?;
        let output = String::from_utf8(output)?;

        assert_eq!(output.lines().count(), 6, "the complete projected history is printed");
        let bases = output
            .lines()
            .filter(|line| line.contains(" base "))
            .collect::<Vec<_>>();
        assert_eq!(bases.len(), 2, "each distinct visible root becomes a base separator");
        assert!(
            bases
                .iter()
                .all(|line| line.starts_with("────") && line.ends_with("────")),
            "base separators use the rebase-todo rails: {bases:?}"
        );
        assert!(
            bases
                .iter()
                .any(|line| line.contains(&orphan.to_hex_with_len(7).to_string()) && line.contains("orphan base")),
            "a base separator retains commit metadata: {bases:?}"
        );
        assert_eq!(
            bases
                .iter()
                .map(|line| Line::raw(*line).width())
                .collect::<HashSet<_>>()
                .len(),
            1,
            "all base separators span the same display width"
        );
        assert!(
            output.contains(&format!(
                "{} {}",
                head.to_hex_with_len(7),
                head_change_id.to_reverse_hex_with_len(7)
            )),
            "a change ID follows its commit hash even when ambiguous: {output:?}"
        );
        for id in [head, parent] {
            let line = output
                .lines()
                .find(|line| line.contains(&id.to_hex_with_len(7).to_string()))
                .context("the ambiguous commit is shown")?;
            assert!(
                line.contains('💥'),
                "ambiguous change IDs are marked in the gutter: {line:?}"
            );
        }
        assert!(output.contains('●'), "history graph lanes are rendered");
        assert!(
            output.lines().any(|line| line.starts_with("🚧✔️💥├")),
            "commit and tree enrichments directly lead their rows: {output:?}"
        );
        assert!(output.contains("📌"), "applicable pins are decorated and traversed");
        assert!(
            output.contains("topic"),
            "a pinned tip outside HEAD history is included"
        );
        assert!(
            output.contains("Mailmapped Author"),
            "default mailmap formatting is retained"
        );
        assert!(
            output.contains("Co: Human Coauthor"),
            "default trailer attribution is retained"
        );
        assert!(
            output.contains("v1") && output.contains("root"),
            "the hidden boundary row is included"
        );
        assert!(!output.contains('\u{1b}'), "plain output contains no terminal escapes");
        Ok(())
    }

    #[test]
    fn split_command_uses_the_index_for_the_new_commit_and_worktree_for_its_parent() -> gix_testtools::Result {
        fn git(path: &Path, args: &[&str]) -> gix_testtools::Result<Vec<u8>> {
            let output = ProcessCommand::new("git").arg("-C").arg(path).args(args).output()?;
            if !output.status.success() {
                return Err(format!(
                    "git {} failed: {}",
                    args.join(" "),
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }
            Ok(output.stdout)
        }

        let fixture = gix_testtools::scripted_fixture_writable("split_commit.sh")?;
        let repository =
            crate::test_repository::open_with(fixture.path(), ["core.editor=sed -i.bak -e 's/^what$/split/'"])?;
        let graph = crate::edit::loaded_view_graph(&repository)?;
        let original = repository.head_id()?.detach();
        crate::enrich::set_note(&repository, original, Some(b"source marker"))?;
        split(repository, &graph, Split { todo: true })?;

        assert_eq!(git(fixture.path(), &["log", "-1", "--format=%s"])?, b"split\n");
        assert_eq!(git(fixture.path(), &["show", "HEAD^:unstaged"])?, b"worktree\n");
        assert_eq!(git(fixture.path(), &["show", "HEAD:staged"])?, b"staged\n");
        assert!(git(fixture.path(), &["diff", "--exit-code"])?.is_empty());
        assert!(git(fixture.path(), &["diff", "--cached", "--exit-code"])?.is_empty());
        let repository = crate::test_repository::open(fixture.path())?;
        let upper = repository.head_id()?.detach();
        let lower = repository
            .find_commit(upper)?
            .parent_ids()
            .next()
            .expect("split has a lower commit")
            .detach();
        let mut enrichments = crate::enrich::open(&repository)?;
        assert_eq!(
            crate::enrich::load(&mut enrichments, crate::change_id::for_commit(&repository, upper)?)?,
            crate::enrich::Enrichment { todo: true, note: None },
            "--todo marks only the new upper commit"
        );
        assert_eq!(
            crate::enrich::load(&mut enrichments, crate::change_id::for_commit(&repository, lower)?)?.note,
            Some("source marker".into()),
            "the original enrichment remains with the rewritten lower identity"
        );
        Ok(())
    }
}

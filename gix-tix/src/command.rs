use std::{
    collections::HashSet,
    ffi::{OsStr, OsString},
    io::Write,
    sync::atomic::AtomicBool,
};

use anyhow::{Context, Result};
use clap::Parser;
use gix::prelude::ReferenceExt;
use ratatui::text::Line;

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
    /// Print the complete ref-tree without opening the terminal UI.
    RefTree(RefTree),
    /// Print the complete history view without opening the terminal UI.
    Show(Show),
    /// Add staged changes, or worktree changes when nothing is staged, to HEAD.
    Amend(Amend),
    /// Move the changes introduced by HEAD into the worktree.
    Spill,
    /// Split HEAD by amending worktree changes into it and committing staged index changes on top.
    Split(Split),
    /// Save index and worktree changes at HEAD.
    Stash,
    /// Pin one or more commits as persistent history tips.
    Pin(Pin),
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

#[derive(Debug, clap::Args)]
struct RefTree {
    /// Omit tags as labels, traversal tips, and topology anchors.
    #[arg(long)]
    no_tags: bool,
    /// Hide this revision and every commit reachable from it.
    #[arg(short = 'x', long, value_name = "REVSPEC")]
    hide: Vec<OsString>,
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
            Command::Travel(args) => return travel::run(repository, args),
            Command::Reword(args) => return reword::run(repository, args),
            Command::New(args) => return new::run(repository, args),
            Command::Rebase(command) => return rebase::run(repository, command),
        }
        Ok(())
    }
}

fn print_ref_tree(repository: &gix::Repository, args: RefTree) -> Result<()> {
    let revisions = if args.revisions.is_empty() {
        crate::history::ref_tree_revisions(repository, !args.no_tags)?
    } else {
        args.revisions
    };
    let rendered = crate::ref_tree::render_full(repository, &revisions, &args.hide, !args.no_tags, args.unicode)?;
    std::io::Write::write_all(&mut std::io::stdout().lock(), rendered.as_bytes())
        .context("could not write ref-tree")?;
    Ok(())
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
    let mut notes = repository
        .notes()
        .map_err(gix::Exn::into_error)
        .context("could not open Git notes")?;
    for row in &app.rows {
        if !notes
            .get(row.id)
            .map_err(gix::Exn::into_error)
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
        let enrichment_marker =
            crate::enrich::marker(todo_ids.contains(&row.id), enrichment_note_ids.contains(&row.id));
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
    match repository.rev_parse_single(revision) {
        Ok(id) => {
            let id = id
                .object()
                .with_context(|| format!("could not read {description}"))?
                .peel_to_commit()
                .with_context(|| format!("{description} does not resolve to a commit"))?
                .id;
            Ok((id, None))
        }
        Err(revision_error) => {
            let graph = crate::edit::loaded_view_graph(repository)?;
            let resolved = std::str::from_utf8(revision)
                .ok()
                .map(|prefix| crate::change_id::resolve_prefix(repository, prefix, graph.commit_ids()))
                .transpose()?
                .flatten();
            match resolved {
                Some(id) => Ok((id, Some(graph))),
                None => Err(revision_error).with_context(|| format!("could not resolve {description} {revision:?}")),
            }
        }
    }
}

fn pin(repository: &gix::Repository, args: Pin) -> Result<()> {
    for pin in create_pins(repository, &args.revisions)? {
        println!("{}", display_pin(repository, &pin)?);
    }
    Ok(())
}

fn create_pins(repository: &gix::Repository, revisions: &[OsString]) -> Result<Vec<crate::history::Pin>> {
    let mut seen = HashSet::new();
    let targets = revisions
        .iter()
        .map(|revision| {
            let revision = gix::path::os_str_into_bstr(revision)
                .with_context(|| format!("revision {} is not valid UTF-8", revision.to_string_lossy()))?;
            let spec = repository
                .rev_parse(revision)
                .with_context(|| format!("could not resolve revision {revision:?}"))?;
            let reference = spec.first_reference().cloned();
            let id = spec
                .single()
                .with_context(|| format!("revision {revision:?} does not name a single object"))?
                .object()
                .context("could not read pin target")?
                .peel_to_commit()
                .context("pin target does not resolve to a commit")?
                .id;
            let target = match reference {
                Some(reference) if reference.clone().attach(repository).peel_to_commit()?.id == id => {
                    gix::refs::Target::Symbolic(reference.name)
                }
                _ => gix::refs::Target::Object(id),
            };
            Ok((target, id))
        })
        .collect::<Result<Vec<_>>>()?;
    targets
        .into_iter()
        .filter(|(target, _id)| seen.insert(target.clone()))
        .map(|(target, id)| {
            crate::edit::time_travel::create_or_reuse_pin(repository, target, id, "tix pin").map(|(pin, _created)| pin)
        })
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
    Ok(())
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
        assert!(ref_tree.unicode);
        assert_eq!(ref_tree.revisions, ["main", "topic"]);

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
        let pin = Cli::try_parse_from(["tix", "pin", "main", "HEAD~2"])
            .expect("one or more pin revisions parse")
            .platform
            .command;
        let Some(Command::Pin(pin)) = pin else {
            panic!("pin was expected")
        };
        assert_eq!(pin.revisions, ["main", "HEAD~2"]);
        let travel = Cli::try_parse_from(["tix", "travel", "--materialize-conflicts", "HEAD~1"])
            .expect("travel parses")
            .platform
            .command;
        let Some(Command::Travel(travel)) = travel else {
            panic!("travel was expected")
        };
        assert!(travel.materialize_conflicts);
        assert_eq!(travel.revision, "HEAD~1");
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
            Cli::command().render_long_help().to_string().contains("GIT_EDITOR"),
            "top-level help explains how to override Git's editor"
        );
    }

    #[test]
    fn preserves_hide_and_help_semantics() {
        for command in [
            &[][..],
            &["ref-tree"],
            &["show"],
            &["amend"],
            &["spill"],
            &["split"],
            &["stash"],
            &["pin"],
            &["travel"],
            &["reword"],
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
            pins.iter().map(|pin| pin.id).collect::<Vec<_>>(),
            [main, root, parent, main],
            "distinct pin targets preserve argument order even when IDs match"
        );
        assert_eq!(
            pins.iter()
                .map(|pin| pin.target.try_name().is_some())
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
        assert_eq!(repeated[0].name, pins[0].name, "an existing symbolic pin is reused");
        assert_eq!(crate::history::all_pins(&repository)?.len(), 4);
        let display = display_pin(&repository, &pins[0])?;
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
            .find(|pin| pin.name == pins[0].name)
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
            output.lines().any(|line| line.starts_with("🚧💥├")),
            "todos directly lead their rows: {output:?}"
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

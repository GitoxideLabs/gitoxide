use std::{collections::HashSet, ffi::OsString};

use anyhow::{Context, Result};
use clap::Parser;
use gix::prelude::{ObjectIdExt, ReferenceExt};

mod rebase;
mod reword;
mod travel;

/// Arguments and commands shared by the standalone `tix` binary and `gix tix`.
#[derive(Debug, clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Platform {
    /// Print help.
    #[arg(long, action = clap::ArgAction::HelpLong)]
    help: Option<bool>,
    /// Exit once all commits and graph lanes have been computed.
    #[arg(long)]
    quit_on_finish: bool,
    /// Hide this revision and every commit reachable from it.
    #[arg(short = 'h', long, value_name = "REVSPEC")]
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
    /// Add staged changes, or worktree changes when nothing is staged, to HEAD.
    Amend,
    /// Move the changes introduced by HEAD into the worktree.
    Spill,
    /// Split HEAD by amending worktree changes into it and committing staged index changes on top.
    Split,
    /// Pin one or more commits as persistent history tips.
    Pin(Pin),
    /// Travel to a commit while preserving reachable history through tix pins.
    Travel(travel::Args),
    /// Edit a commit and lazily rebase every descendant retained by a tix pin.
    Reword(reword::Args),
    /// Generate or apply a self-contained history-rebase todo.
    #[command(subcommand)]
    Rebase(rebase::Command),
}

#[derive(Debug, clap::Args)]
struct RefTree {
    /// Print help.
    #[arg(long, action = clap::ArgAction::HelpLong)]
    help: Option<bool>,
    /// Omit tags as labels, traversal tips, and topology anchors.
    #[arg(long)]
    no_tags: bool,
    /// Hide this revision and every commit reachable from it.
    #[arg(long, value_name = "REVSPEC")]
    hide: Vec<OsString>,
    /// Use the ref-tree view's Unicode line and node glyphs instead of ASCII.
    #[arg(long)]
    unicode: bool,
    /// Revisions to traverse instead of all normal references.
    #[arg(value_name = "REVSPEC")]
    revisions: Vec<OsString>,
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
    disable_help_flag = true,
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
            help: _,
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
            command => command,
        };
        let _log_guard = crate::logging::init().context("could not initialize tix diagnostics")?;
        match command {
            Command::RefTree(_) => unreachable!("ref-tree commands return before logging"),
            Command::Amend => {
                let graph = crate::edit::loaded_view_graph(&repository)?;
                edit_head(repository, &graph, crate::edit::head::Kind::Amend, "amend")?;
            }
            Command::Spill => {
                let graph = crate::edit::loaded_view_graph(&repository)?;
                edit_head(repository, &graph, crate::edit::head::Kind::Spill, "spill")?;
            }
            Command::Split => {
                let graph = crate::edit::loaded_view_graph(&repository)?;
                split(repository, &graph)?;
            }
            Command::Pin(args) => pin(&repository, args)?,
            Command::Travel(args) => return travel::run(repository, args),
            Command::Reword(args) => return reword::run(repository, args),
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
        pin.id
            .attach(repository)
            .shorten()
            .context("could not shorten pinned commit ID")?
    ))
}

fn edit_head(
    repository: gix::Repository,
    graph: &crate::history::HistoryGraph,
    kind: crate::edit::head::Kind,
    verb: &str,
) -> Result<()> {
    match crate::edit::head::perform(repository, graph, kind, None)? {
        Some(id) => println!("{}", id.to_hex_with_len(7)),
        None => println!("nothing to {verb}"),
    }
    Ok(())
}

fn split(repository: gix::Repository, graph: &crate::history::HistoryGraph) -> Result<()> {
    let repository_path = repository.git_dir().to_owned();
    let bare = repository.is_bare();
    let prepared = crate::edit::split::prepare(repository)?;
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
    let id = crate::edit::split::apply(repository, graph, prepared, &edited)?;
    println!("{}", id.to_hex_with_len(7));
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{path::Path, process::Command as ProcessCommand};

    use clap::{CommandFactory, error::ErrorKind};

    use super::*;

    #[test]
    fn clap_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_tui_options_and_top_level_commands() {
        let cli = Cli::try_parse_from(["tix", "--quit-on-finish", "-h", "main", "--hide", "tag", "topic"])
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
            "--hide",
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
        assert!(ref_tree.help.is_none());
        assert!(ref_tree.no_tags);
        assert_eq!(ref_tree.hide, ["private"]);
        assert!(ref_tree.unicode);
        assert_eq!(ref_tree.revisions, ["main", "topic"]);

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

        assert!(matches!(
            Cli::try_parse_from(["tix", "amend"])
                .expect("amend parses")
                .platform
                .command,
            Some(Command::Amend)
        ));
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
            Some(Command::Split)
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
        assert!(reword.message.is_empty());
        assert!(reword.file.is_none());
        assert!(reword.author.is_none());
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
        assert_eq!(reword.message, ["title", "body"]);
        assert_eq!(
            reword.author.as_deref(),
            Some(std::ffi::OsStr::new("Agent <agent@example.com>"))
        );
        assert!(
            Cli::try_parse_from(["tix", "reword", "HEAD", "-m", "message", "-f", "message.txt"]).is_err(),
            "message and file inputs are mutually exclusive"
        );
        assert!(matches!(
            Cli::try_parse_from([
                "tix",
                "rebase",
                "todo",
                "-h",
                "main",
                "--onto",
                "next",
                "--edit-and-apply",
                "topic"
            ])
            .expect("rebase todo parses")
            .platform
            .command,
            Some(Command::Rebase(rebase::Command::Todo(_)))
        ));
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
        assert_eq!(
            Cli::try_parse_from(["tix", "--help"])
                .expect_err("long help exits through clap")
                .kind(),
            ErrorKind::DisplayHelp
        );
        assert_eq!(
            Cli::try_parse_from(["tix", "-h"])
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
        let (label, short_id) = display.split_once(' ').context("pin output has a label and ID")?;
        assert!(label.starts_with("pin:"), "output names the pin");
        assert_eq!(
            short_id,
            main.attach(&repository).shorten()?.to_string(),
            "output uses the repository's abbreviated ID"
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
        split(repository, &graph)?;

        assert_eq!(git(fixture.path(), &["log", "-1", "--format=%s"])?, b"split\n");
        assert_eq!(git(fixture.path(), &["show", "HEAD^:unstaged"])?, b"worktree\n");
        assert_eq!(git(fixture.path(), &["show", "HEAD:staged"])?, b"staged\n");
        assert!(git(fixture.path(), &["diff", "--exit-code"])?.is_empty());
        assert!(git(fixture.path(), &["diff", "--cached", "--exit-code"])?.is_empty());
        Ok(())
    }
}

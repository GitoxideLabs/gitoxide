use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use gix::{ObjectId, bstr::BString, prelude::ObjectIdExt};

use super::rebase;

const HELP: &[u8] = br#"

<!--
# Rebase todo help

- Each fork section lists a stack from its oldest commit to its newest. Blank lines are ignored.
- `pick <id>` keeps a commit. Delete its line to drop it, or move the line to reorder it. Each listed commit may be picked only once.
- `## fork <id>` starts a stack at an existing commit or an earlier picked commit. Delete a fork heading to continue its commits on the preceding stack; add one to create a fork. A listed commit must be picked before it can be a fork target.
- `empty <title>` creates an empty commit with the text after the command as its title.
- Commands may be plain text or enclosed in backticks. Text after a backticked command and text after a fork ID is display-only context.
- Prefix `pick` or `empty` with `@` to choose the post-rebase checkout. Retain exactly one generated marker; if none was generated, add at most one. A checkout marker requires a worktree.
- Saving an unchanged document is a no-op unless listed commits have a pending rebase. The ancestry ending at `@` is cherry-picked and re-signed; other stacks remain lazily rebased with invalidated signatures until time travel reaches them.
- Mutable refs follow rewritten and dropped commits, except tags and remote-tracking refs. New unreferenced leaves are pinned.
- A checkout conflict uses tix's standard suspended-conflict flow; concurrent ref changes abort the update.
-->
"#;

pub(crate) struct Commit {
    pub id: ObjectId,
    pub parents: Vec<ObjectId>,
    pub info: String,
}

pub(crate) struct Prepared {
    pub editor: std::ffi::OsString,
    pub document: Vec<u8>,
    base: ObjectId,
    scope: Vec<ObjectId>,
    marker_required: bool,
    checkout_allowed: bool,
    expected_refs: Vec<rebase::ExpectedRef>,
    pub has_pending: bool,
}

struct Section {
    parent: ObjectId,
    commits: Vec<ObjectId>,
}

#[tracing::instrument(skip_all, fields(base = %base, commits = commits.len()))]
pub(crate) fn prepare(
    repo: &gix::Repository,
    base: ObjectId,
    onto: ObjectId,
    onto_title: Option<&str>,
    commits: &[Commit],
    head: Option<ObjectId>,
) -> Result<Prepared> {
    let editor = repo.editor().context("no Git editor is available")?;
    repo.find_commit(base)
        .context("could not find the selected rebase base")?;
    repo.find_commit(onto).context("could not find the rebase target")?;
    let scope: Vec<_> = commits.iter().map(|commit| commit.id).collect();
    let scope_set: HashSet<_> = scope.iter().copied().collect();
    let marker_required = head.is_some_and(|head| scope_set.contains(&head));
    let expected_refs = rebase::capture_refs(repo, &scope)?;
    let has_pending = commits.iter().try_fold(false, |pending, commit| {
        Ok::<_, anyhow::Error>(pending || rebase::has_marker(&repo.find_commit(commit.id)?.decode()?.into_owned()?))
    })?;

    let mut children = HashMap::<ObjectId, Vec<ObjectId>>::new();
    let by_id: HashMap<_, _> = commits.iter().map(|commit| (commit.id, commit)).collect();
    for commit in commits {
        let parent = commit
            .parents
            .first()
            .copied()
            .context("an editable commit has no parent")?;
        if parent != base && !scope_set.contains(&parent) {
            anyhow::bail!("an editable commit is not connected to the selected base");
        }
        children.entry(parent).or_default().push(commit.id);
    }
    let mut sections = Vec::new();
    for child in children.get(&base).into_iter().flatten().copied() {
        let mut section = Section {
            parent: onto,
            commits: Vec::new(),
        };
        let mut branches = Vec::new();
        walk(child, &children, &mut section, &mut branches);
        sections.push(section);
        sections.extend(branches);
    }

    let source = short(repo, base)?;
    let title = if base == onto {
        format!("# Rebase from `{source}`")
    } else {
        format!("# Rebase from `{source}` onto `{}`", short(repo, onto)?)
    };
    let mut document = b"<!-- Rebase help is at the bottom of this file. -->\n\n".to_vec();
    document.extend_from_slice(title.as_bytes());
    document.extend_from_slice(b"\n\n");
    for (section_index, section) in sections.iter().enumerate() {
        if section_index > 0 {
            document.push(b'\n');
        }
        write_fork_heading(
            &mut document,
            repo,
            section.parent,
            (section.parent == onto && onto != base).then_some(onto_title).flatten(),
        )?;
        for id in &section.commits {
            let commit = by_id[id];
            let verb = if Some(*id) == head { "@pick" } else { "pick" };
            document.extend_from_slice(
                format!("`{verb} {}` {}\n", short(repo, *id)?, escape_markdown(&commit.info)).as_bytes(),
            );
        }
    }
    if sections.is_empty() {
        write_fork_heading(
            &mut document,
            repo,
            onto,
            (onto != base).then_some(onto_title).flatten(),
        )?;
    }
    document.extend_from_slice(HELP);
    Ok(Prepared {
        editor,
        document,
        base: onto,
        scope,
        marker_required,
        checkout_allowed: repo.workdir().is_some(),
        expected_refs,
        has_pending,
    })
}

fn write_fork_heading(out: &mut Vec<u8>, repo: &gix::Repository, id: ObjectId, title: Option<&str>) -> Result<()> {
    out.extend_from_slice(format!("## fork {}", short(repo, id)?).as_bytes());
    if let Some(title) = title {
        out.extend_from_slice(format!(" {}", escape_markdown(title)).as_bytes());
    }
    out.push(b'\n');
    Ok(())
}

fn walk(id: ObjectId, children: &HashMap<ObjectId, Vec<ObjectId>>, section: &mut Section, sections: &mut Vec<Section>) {
    section.commits.push(id);
    let Some(child_ids) = children.get(&id) else { return };
    if let Some(first) = child_ids.first() {
        walk(*first, children, section, sections);
    }
    for child in child_ids.iter().skip(1) {
        let mut branch = Section {
            parent: id,
            commits: Vec::new(),
        };
        let mut nested = Vec::new();
        walk(*child, children, &mut branch, &mut nested);
        sections.push(branch);
        sections.extend(nested);
    }
}

fn short(repo: &gix::Repository, id: ObjectId) -> Result<String> {
    Ok(id
        .attach(repo)
        .shorten()
        .context("could not shorten a rebase todo ID")?
        .to_string())
}

fn escape_markdown(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if matches!(ch, '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '>') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

pub(crate) fn parse(repo: &gix::Repository, prepared: Prepared, edited: &[u8]) -> Result<rebase::Plan> {
    let input = std::str::from_utf8(edited).context("the rebase todo is not UTF-8")?;
    let scope: HashSet<_> = prepared.scope.iter().copied().collect();
    let mut picked = HashMap::<ObjectId, usize>::new();
    let mut steps = Vec::new();
    let mut cursor = None;
    let mut checkout = None;
    let mut marker_count = 0;
    let mut sections = 0;
    let mut section_has_commit = false;
    let mut in_comment = false;

    for raw in input.lines() {
        let line = raw.trim();
        if in_comment {
            if line.contains("-->") {
                in_comment = false;
            }
            continue;
        }
        if line.starts_with("<!--") {
            in_comment = !line.contains("-->");
            continue;
        }
        if line.is_empty() || line.starts_with("# ") {
            continue;
        }
        if let Some(target) = line.strip_prefix("## fork ") {
            if sections > 0 && !section_has_commit {
                anyhow::bail!("a fork section contains no commits");
            }
            let id = resolve_commit(
                repo,
                target
                    .split_whitespace()
                    .next()
                    .context("a fork heading needs a commit ID")?,
            )?;
            cursor = Some(if let Some(index) = picked.get(&id) {
                rebase::PlanParent::Step(*index)
            } else if scope.contains(&id) {
                anyhow::bail!("a fork target must be picked before it is used");
            } else {
                rebase::PlanParent::Existing(id)
            });
            sections += 1;
            section_has_commit = false;
            continue;
        }

        let (command, tail) = if let Some(line) = line.strip_prefix('`') {
            let (command, tail) = line
                .split_once('`')
                .context("a Markdown todo command has no closing backtick")?;
            (command, tail.trim())
        } else {
            (line, "")
        };
        let (verb, value) = command.split_once(char::is_whitespace).unwrap_or((command, ""));
        let marked = verb.starts_with('@');
        let verb = verb.strip_prefix('@').unwrap_or(verb);
        if marked {
            marker_count += 1;
            if marker_count > 1 {
                anyhow::bail!("the rebase todo contains more than one @ marker");
            }
            if !prepared.checkout_allowed {
                anyhow::bail!("the rebase todo cannot select a checkout without a worktree");
            }
        }
        let parent = cursor.context("the first todo command must follow a fork heading")?;
        let commit = match verb {
            "pick" => {
                let id = resolve_commit(
                    repo,
                    value.split_whitespace().next().context("a pick needs a commit ID")?,
                )?;
                if !scope.contains(&id) {
                    anyhow::bail!("a pick is outside the editable history");
                }
                if picked.contains_key(&id) {
                    anyhow::bail!("a commit is picked more than once");
                }
                rebase::PlanCommit::Pick(id)
            }
            "empty" => {
                let title = if value.trim().is_empty() { tail } else { value.trim() };
                if title.is_empty() {
                    anyhow::bail!("an empty commit needs a title");
                }
                rebase::PlanCommit::Empty(BString::from(title))
            }
            _ => anyhow::bail!("unsupported rebase todo command {verb:?}"),
        };
        let index = steps.len();
        if let rebase::PlanCommit::Pick(id) = commit {
            picked.insert(id, index);
        }
        steps.push(rebase::PlanStep { parent, commit });
        cursor = Some(rebase::PlanParent::Step(index));
        if marked {
            checkout = Some(index);
        }
        section_has_commit = true;
    }
    if sections == 0 {
        anyhow::bail!("the rebase todo has no fork heading");
    }
    if sections > 1 && !section_has_commit {
        anyhow::bail!("the last fork section contains no commits");
    }
    if prepared.marker_required && marker_count != 1 {
        anyhow::bail!("the current checkout marker must be retained");
    }
    Ok(rebase::Plan {
        base: prepared.base,
        scope: prepared.scope,
        steps,
        checkout,
        expected_refs: prepared.expected_refs,
    })
}

fn resolve_commit(repo: &gix::Repository, value: &str) -> Result<ObjectId> {
    if value.len() < 4 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        anyhow::bail!("{value:?} is not a commit ID prefix");
    }
    let id = repo
        .rev_parse_single(value)
        .with_context(|| format!("could not resolve commit ID {value:?}"))?;
    id.object()
        .context("could not load a todo object")?
        .try_into_commit()
        .context("a todo ID does not name a commit")?;
    Ok(id.detach())
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    fn repo() -> gix_testtools::Result<(gix_testtools::tempfile::TempDir, gix::Repository)> {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = gix::open_opts(
            fixture.path(),
            gix::open::Options::isolated().config_overrides([
                "core.editor=:".to_owned(),
                "core.abbrev=7".to_owned(),
                "user.name=todo author".to_owned(),
                "user.email=todo@example.com".to_owned(),
            ]),
        )?;
        Ok((fixture, repo))
    }

    fn commits(repo: &gix::Repository) -> gix_testtools::Result<(ObjectId, ObjectId, ObjectId, Vec<Commit>)> {
        let base = repo.rev_parse_single("HEAD~2")?.detach();
        let middle = repo.rev_parse_single("HEAD~1")?.detach();
        let tip = repo.head_id()?.detach();
        Ok((
            base,
            middle,
            tip,
            vec![
                Commit {
                    id: tip,
                    parents: vec![middle],
                    info: "(main) 2000-01-03 author tip".into(),
                },
                Commit {
                    id: middle,
                    parents: vec![base],
                    info: "2000-01-02 author middle * markdown".into(),
                },
            ],
        ))
    }

    #[test]
    fn markdown_flows_from_base_to_tip_and_uses_repository_abbreviations() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let (base, _middle, tip, commits) = commits(&repo)?;
        let prepared = prepare(&repo, base, base, None, &commits, Some(tip))?;
        assert!(!prepared.has_pending);
        let document = String::from_utf8(prepared.document.clone())?;
        assert!(document.starts_with("<!-- Rebase help is at the bottom of this file. -->"));
        assert!(document.contains(&format!("# Rebase from `{}`", base.to_hex_with_len(7))));
        assert!(document.contains(&format!("## fork {}", base.to_hex_with_len(7))));
        let middle = document.find("`pick ").expect("the oldest pick is shown");
        let tip = document.find("`@pick ").expect("HEAD is marked");
        assert!(middle < tip, "the todo grows from the base towards the tip");
        assert!(
            document.contains("middle \\* markdown"),
            "metadata is escaped for Markdown"
        );
        assert!(
            document.find("# Rebase todo help").expect("help is present") > tip,
            "complete instructions follow the editable todo"
        );
        assert!(document.ends_with("-->\n"), "all trailing help is one Markdown comment");
        Ok(())
    }

    #[test]
    fn an_unchanged_todo_replays_pending_commits_with_normal_plan_semantics() -> gix_testtools::Result {
        let (fixture, repo) = repo()?;
        let (base, middle, _tip, _) = commits(&repo)?;
        let graph = super::super::loaded_graph(&repo)?;
        let mut commit = repo.find_commit(middle)?.decode()?.into_owned()?;
        commit.tree = repo.find_commit(base)?.tree_id()?.detach();
        let marked = rebase::perform(
            &repo,
            &graph,
            rebase::Edit::Replace { target: middle, commit },
            rebase::Signature::InvalidateExisting,
            rebase::Tree::LeaveAsIsAndMark,
        )?
        .complete()?
        .selected
        .expect("the pending replacement selects its rewritten commit");
        let tip = repo.head_id()?.detach();
        let commits = vec![
            Commit {
                id: tip,
                parents: vec![marked],
                info: "tip".into(),
            },
            Commit {
                id: marked,
                parents: vec![base],
                info: "middle".into(),
            },
        ];
        let prepared = prepare(&repo, base, base, None, &commits, Some(tip))?;
        assert!(
            prepared.has_pending,
            "pending commits make an unchanged todo actionable"
        );
        let document = prepared.document.clone();
        let plan = parse(&repo, prepared, &document)?;
        let graph = super::super::loaded_graph(&repo)?;
        rebase::perform_plan(&repo, &graph, plan)?.complete()?;

        let mut current = Some(repo.head_id()?.detach());
        while let Some(id) = current {
            let commit = repo.find_commit(id)?.decode()?.into_owned()?;
            assert!(!rebase::has_marker(&commit), "the eager @ ancestry is replayed");
            current = commit.parents.first().copied();
        }
        let files = Command::new("git")
            .arg("-C")
            .arg(fixture.path())
            .args(["ls-tree", "-r", "--name-only", "HEAD"])
            .output()?;
        assert!(files.status.success());
        assert_eq!(files.stdout, b"base\ntip\n", "replay uses the recorded original parent");
        Ok(())
    }

    #[test]
    fn update_todo_roots_the_stack_at_the_hidden_tip_and_labels_only_that_heading() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let (base, _middle, tip, commits) = commits(&repo)?;
        let mut commit = repo.find_commit(base)?.decode()?.into_owned()?;
        commit.parents = [base].into_iter().collect();
        commit.message = "updated hidden base".into();
        let onto = repo.write_object(&commit)?.detach();

        let prepared = prepare(
            &repo,
            base,
            onto,
            Some("[A] updated * hidden base"),
            &commits,
            Some(tip),
        )?;
        let document = String::from_utf8(prepared.document.clone())?;
        assert!(
            document.contains(&format!(
                "# Rebase from `{}` onto `{}`",
                base.to_hex_with_len(7),
                onto.to_hex_with_len(7)
            )),
            "the update target is explicit in the document title"
        );
        assert!(
            document.contains(&format!(
                "## fork {} \\[A\\] updated \\* hidden base",
                onto.to_hex_with_len(7)
            )),
            "the unfamiliar fork target carries its escaped UI title"
        );
        assert_eq!(
            document.matches("updated \\* hidden base").count(),
            1,
            "only the new update target is labelled"
        );

        let plan = parse(&repo, prepared, document.as_bytes())?;
        assert_eq!(plan.base, onto);
        assert_eq!(plan.steps[0].parent, rebase::PlanParent::Existing(onto));
        Ok(())
    }

    #[test]
    fn parses_reordering_forks_empty_commits_and_a_moved_checkout() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let (base, middle, tip, commits) = commits(&repo)?;
        let prepared = prepare(&repo, base, base, None, &commits, Some(tip))?;
        let edited = format!(
            "# Rebase\n\n## fork {}\n`pick {}` ignored\n@empty a new checkpoint\n\n## fork {}\n@pick {}\n",
            base.to_hex_with_len(7),
            tip.to_hex_with_len(7),
            tip.to_hex_with_len(7),
            middle.to_hex_with_len(7),
        );
        let err = parse(&repo, prepared, edited.as_bytes()).expect_err("two checkout markers are invalid");
        assert!(format!("{err:#}").contains("more than one @"));

        let prepared = prepare(&repo, base, base, None, &commits, Some(tip))?;
        let edited = format!(
            "## fork {}\npick {} ignored display metadata\nempty a new checkpoint\n\n## fork {}\n@pick {}\n",
            base.to_hex_with_len(7),
            tip.to_hex_with_len(7),
            tip.to_hex_with_len(7),
            middle.to_hex_with_len(7),
        );
        let plan = parse(&repo, prepared, edited.as_bytes())?;
        assert_eq!(plan.steps.len(), 3);
        assert_eq!(plan.checkout, Some(2));
        assert_eq!(plan.steps[2].parent, rebase::PlanParent::Step(0));
        assert!(matches!(&plan.steps[1].commit, rebase::PlanCommit::Empty(title) if title == b"a new checkpoint"));
        Ok(())
    }

    #[test]
    fn unchanged_marker_cannot_be_removed_but_an_empty_plan_is_valid_without_one() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let (base, _middle, tip, commits) = commits(&repo)?;
        let prepared = prepare(&repo, base, base, None, &commits, Some(tip))?;
        let edited = format!("## fork {}\n", base.to_hex_with_len(7));
        let err = parse(&repo, prepared, edited.as_bytes()).expect_err("HEAD must be moved before its pick is dropped");
        assert!(format!("{err:#}").contains("checkout marker"));

        let prepared = prepare(&repo, base, base, None, &commits, None)?;
        let plan = parse(&repo, prepared, edited.as_bytes())?;
        assert!(
            plan.steps.is_empty(),
            "all picks may be dropped when no marker is required"
        );
        Ok(())
    }
}

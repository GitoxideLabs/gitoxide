use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use gix::{
    ObjectId,
    bstr::{BString, ByteSlice},
    prelude::ObjectIdExt,
};
use ratatui::text::Line;

use super::rebase;

const HELP: &str = r#"

<!--
# Rebase todo help

- Read the editable plan from bottom to top. Each fork separator is the base of the stack above it. Blank lines are ignored.
- `pick <id>` keeps a commit. Delete its line to drop it, or move the line to reorder it. Each listed commit may be picked only once.
- `squash <id>` folds a commit into the following command below it in the same fork. Its full message is retained with a source heading, and additional authors become `Co-authored-by` trailers.
- A centered `fork <id>` separator starts the stack above it at an existing commit or a commit picked below it. The selected hidden boundary is labelled `(base)` with its title; a newer hidden tip used by rebase-update is `(updated-base)`, and an explicit command-line target is `(onto)`. Other fork separators stay terse. Delete a separator to continue its commits on the stack below; add one to create a fork. A listed commit must be picked below before it can be a fork target.
- `empty <title>` creates an empty commit with the text after the command as its title.
- Commands may be plain text or enclosed in backticks. Text after a backticked command and text after a fork ID is display-only context.
- Prefix `pick`, `squash`, or `empty` with `@` to choose the post-rebase checkout. Reference lines like `(main, topic)` point refs at the following separator or command below them; moving, adding, or removing names moves, creates, or deletes refs, including existing editable refs outside the generated todo. The current attached ref stays attached while it remains at the `@` command. Prefix one editable ref with `@` to attach HEAD to it explicitly; it must point to the `@` command.
- Saving an unchanged document in the history-view editor is a no-op unless the ancestry ending at `@` has a pending rebase. Explicit `tix rebase apply` and `--edit-and-apply` apply valid unchanged plans. Unchanged picks whose parent stays unchanged retain their IDs; replay starts at the first pending or structurally changed commit. Changed commits through `@` are cherry-picked and re-signed, while descendants and other stacks remain lazily rebased with invalidated signatures until time travel reaches them.
- Tix pins, stashes, and review refs, tags, remote-tracking refs, and symbolic refs stay unchanged and hidden. A ref checked out by another worktree may be moved but not deleted. New unreferenced leaves are pinned.
- A todo conflict changes nothing unless explicitly accepted. The TUI offers `<enter>` to materialize it; command-line apply requires `--materialize-conflicts [CONTINUE]` and writes a continuation todo. Resolve the ordinary unmerged index, then apply that todo. Concurrent ref changes still abort the update.
- Commit states are display-only and editing them has no effect: `🚧` means the commit is a todo, `📝` it has a note, `✔️` its tree passed checks, `↻` a lazy rebase is pending, `◌` an empty signature awaits signing, `◐` a signature is present but unverified, `○` means unsigned, and `🎁` means worktree state is stashed for that commit. Stashes follow rewritten commits automatically; dropping a stashed commit or combining multiple stashes into one result is rejected.
-->
"#;

const STATE_START: &str = "<!-- tix-rebase-state-v2\n";
const STATE_END: &str = "-->";
const STATE_CLOSE: &str = "\n-->";

pub(crate) struct Commit {
    pub id: ObjectId,
    pub parents: Vec<ObjectId>,
    pub info: String,
}

#[derive(Debug)]
pub(crate) struct Prepared {
    pub document: Vec<u8>,
    pub apply_unchanged: bool,
}

#[derive(Clone, Copy)]
pub(crate) enum OntoKind {
    UpdatedBase,
    Onto,
}

struct State {
    base: ObjectId,
    onto: ObjectId,
    tips: Vec<ObjectId>,
    scope: Vec<ObjectId>,
    marker_required: bool,
    checkout_allowed: bool,
    head_ref: Option<gix::refs::FullName>,
    edit_refs: bool,
    expected_refs: Vec<rebase::ExpectedRef>,
    resolved: Option<ObjectId>,
    continuation_sources: Vec<ObjectId>,
}

#[derive(Debug)]
pub(crate) struct Parsed {
    pub plan: rebase::Plan,
    pub tips: Vec<ObjectId>,
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
    commits: &[Commit],
    resolved_tips: &[ObjectId],
    onto_kind: OntoKind,
    show_change_ids: bool,
) -> Result<Prepared> {
    repo.find_commit(base)
        .context("could not find the selected rebase base")?;
    repo.find_commit(onto).context("could not find the rebase target")?;
    let head_state = repo.head()?;
    let head = head_state
        .id()
        .map(gix::Id::detach)
        .context("rebase todos require a born HEAD")?;
    let head_ref = repo
        .workdir()
        .is_some()
        .then(|| {
            head_state
                .referent_name()
                .filter(|name| name.category() == Some(gix::refs::Category::LocalBranch))
                .map(ToOwned::to_owned)
        })
        .flatten();
    let scope: Vec<_> = commits.iter().map(|commit| commit.id).collect();
    let scope_set: HashSet<_> = scope.iter().copied().collect();
    let by_id: HashMap<_, _> = commits.iter().map(|commit| (commit.id, commit)).collect();
    let marker_required = repo.workdir().is_some() && scope_set.contains(&head);
    let mut tips = scope_set.clone();
    for commit in commits {
        for parent in &commit.parents {
            tips.remove(parent);
        }
    }
    let tips = tips.into_iter().collect::<Vec<_>>();
    let mut cursor = marker_required.then_some(head);
    let mut has_pending = false;
    while let Some(id) = cursor {
        let commit = by_id.get(&id).context("the checkout ancestry is incomplete")?;
        if rebase::is_pending(&repo.find_commit(id)?.decode()?.into_owned()?) {
            has_pending = true;
            break;
        }
        cursor = commit
            .parents
            .first()
            .copied()
            .filter(|parent| scope_set.contains(parent));
    }
    let apply_unchanged = base != onto || has_pending;

    let mut children = HashMap::<ObjectId, Vec<ObjectId>>::new();
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
    let mut ref_points = scope.clone();
    ref_points.push(onto);
    if sections.is_empty() {
        ref_points.push(base);
    }
    ref_points.extend(sections.iter().map(|section| section.parent));
    ref_points.sort_unstable();
    ref_points.dedup();
    let expected_refs = rebase::capture_refs(repo, &ref_points, &tips)?;

    let source = short(repo, base, show_change_ids)?;
    let title = if base == onto {
        format!("# Rebase from `{source}`")
    } else {
        format!(
            "# Rebase from `{source}` onto `{}`",
            short(repo, onto, show_change_ids)?
        )
    };
    let state = State {
        base,
        onto,
        tips: if resolved_tips.is_empty() {
            tips
        } else {
            resolved_tips.to_vec()
        },
        scope: scope.clone(),
        marker_required,
        checkout_allowed: repo.workdir().is_some(),
        head_ref,
        edit_refs: true,
        expected_refs,
        resolved: None,
        continuation_sources: Vec::new(),
    };
    let mut document = unchanged_notice(base != onto, has_pending).as_bytes().to_vec();
    document.push(b'\n');
    document.extend_from_slice(title.as_bytes());
    document.extend_from_slice(b"\n\n");
    let anchor_kind = if base == onto {
        "base"
    } else {
        match onto_kind {
            OntoKind::UpdatedBase => "updated-base",
            OntoKind::Onto => "onto",
        }
    };
    let anchor_title = anchor_title(repo, onto)?;
    let mut body = Vec::new();
    let mut enrichments = crate::enrich::open(repo)?;
    let mut tree_enrichments = crate::enrich::open_tree(repo)?;
    let mut written_external_refs = HashSet::new();
    for (section_index, section) in sections.iter().enumerate() {
        if section_index > 0 {
            body.push(b'\n');
        }
        write_fork_heading(
            &mut body,
            repo,
            section.parent,
            (section.parent == onto).then_some((anchor_kind, anchor_title.as_str())),
            show_change_ids,
        )?;
        if !scope_set.contains(&section.parent) && written_external_refs.insert(section.parent) {
            write_refs_at(&mut body, &state.expected_refs, section.parent)?;
        }
        for id in &section.commits {
            let commit = by_id[id];
            let verb = if marker_required && *id == head {
                "@pick"
            } else {
                "pick"
            };
            let states = commit_states(repo, &mut enrichments, &mut tree_enrichments, *id)?;
            body.extend_from_slice(
                format!(
                    "`{verb} {}` {states}{}\n",
                    short(repo, *id, show_change_ids)?,
                    commit.info
                )
                .as_bytes(),
            );
            write_refs_at(&mut body, &state.expected_refs, *id)?;
        }
    }
    if sections.is_empty() {
        write_fork_heading(
            &mut body,
            repo,
            onto,
            Some((anchor_kind, anchor_title.as_str())),
            show_change_ids,
        )?;
        write_refs_at(&mut body, &state.expected_refs, onto)?;
        if base != onto {
            write_refs_at(&mut body, &state.expected_refs, base)?;
        }
    }
    write_bottom_up(&mut document, &body)?;
    document.extend_from_slice(HELP.as_bytes());
    write_state(&mut document, &state);
    Ok(Prepared {
        document,
        apply_unchanged,
    })
}

pub(crate) fn prepare_continuation(
    repo: &gix::Repository,
    plan: &rebase::Plan,
    tips: Vec<ObjectId>,
    show_change_ids: bool,
) -> Result<Prepared> {
    let resolved = plan.steps.iter().find_map(|step| match step.commit {
        rebase::PlanCommit::Resolved(id) => Some(id),
        _ => None,
    });
    let state = State {
        base: plan.base,
        onto: plan.base,
        tips,
        scope: plan.scope.clone(),
        marker_required: plan.checkout.is_some(),
        checkout_allowed: repo.workdir().is_some(),
        head_ref: plan.checkout.as_ref().and_then(|checkout| checkout.reference.clone()),
        edit_refs: true,
        expected_refs: plan.expected_refs.clone(),
        resolved,
        continuation_sources: plan.steps.iter().flat_map(|step| step.squash.iter().copied()).collect(),
    };
    let mut document = b"<!-- Rebase help follows. Saving unchanged continues the materialized rebase; empty this file or remove the tix-rebase-state-v2 comment to cancel. -->\n# Continue materialized rebase\n\n".to_vec();
    let mut body = Vec::new();
    let mut enrichments = crate::enrich::open(repo)?;
    let mut tree_enrichments = crate::enrich::open_tree(repo)?;
    for (index, step) in plan.steps.iter().enumerate() {
        let continues = matches!(step.parent, rebase::PlanParent::Step(parent) if parent + 1 == index);
        if !continues {
            if index > 0 {
                body.push(b'\n');
            }
            let parent = match step.parent {
                rebase::PlanParent::Existing(id) => id,
                rebase::PlanParent::Step(parent) => match plan.steps[parent].commit {
                    rebase::PlanCommit::Pick(id) | rebase::PlanCommit::Copy(id) | rebase::PlanCommit::Resolved(id) => {
                        id
                    }
                    rebase::PlanCommit::Empty(_) => {
                        anyhow::bail!("a continuation fork cannot target an unwritten empty commit")
                    }
                },
            };
            let base_title = (parent == plan.base)
                .then(|| anchor_title(repo, parent))
                .transpose()?
                .unwrap_or_default();
            write_fork_heading(
                &mut body,
                repo,
                parent,
                (parent == plan.base).then_some(("base", base_title.as_str())),
                show_change_ids,
            )?;
            if matches!(step.parent, rebase::PlanParent::Existing(_)) {
                write_plan_refs_at(&mut body, &plan.expected_refs, step.parent)?;
            }
        }
        let marker = if plan
            .checkout
            .as_ref()
            .is_some_and(|checkout| checkout.target == rebase::PlanParent::Step(index))
        {
            "@"
        } else {
            ""
        };
        match step.commit {
            rebase::PlanCommit::Pick(id) | rebase::PlanCommit::Copy(id) | rebase::PlanCommit::Resolved(id) => {
                let value = if matches!(step.commit, rebase::PlanCommit::Resolved(_)) {
                    let hash = ObjectId::null(id.kind()).to_string();
                    if show_change_ids {
                        format!(
                            "{hash} {}",
                            crate::change_id::for_commit(repo, id)?.to_reverse_hex_with_len(hash.len())
                        )
                    } else {
                        hash
                    }
                } else {
                    short(repo, id, show_change_ids)?
                };
                let title = anchor_title(repo, id)?;
                body.extend_from_slice(
                    format!(
                        "`{marker}pick {value}` {}{}\n",
                        commit_states(repo, &mut enrichments, &mut tree_enrichments, id)?,
                        title
                    )
                    .as_bytes(),
                );
            }
            rebase::PlanCommit::Empty(ref title) => {
                body.extend_from_slice(format!("`{marker}empty {}`\n", title.to_str_lossy()).as_bytes());
            }
        }
        for id in &step.squash {
            let title = anchor_title(repo, *id)?;
            body.extend_from_slice(
                format!(
                    "`squash {}` {}{}\n",
                    short(repo, *id, show_change_ids)?,
                    commit_states(repo, &mut enrichments, &mut tree_enrichments, *id)?,
                    title
                )
                .as_bytes(),
            );
        }
        write_plan_refs_at(&mut body, &plan.expected_refs, rebase::PlanParent::Step(index))?;
    }
    write_bottom_up(&mut document, &body)?;
    document.extend_from_slice(HELP.as_bytes());
    write_state(&mut document, &state);
    Ok(Prepared {
        document,
        apply_unchanged: true,
    })
}

fn unchanged_notice(base_updated: bool, has_pending: bool) -> &'static str {
    match (base_updated, has_pending) {
        (false, false) => {
            "<!-- Rebase help follows. Saving unchanged is a no-op; empty this file or remove the tix-rebase-state-v2 comment to cancel. -->"
        }
        (false, true) => {
            "<!-- Rebase help follows. Pending commits on the @ ancestry make saving unchanged apply this todo: that ancestry is replayed now and other forks stay lazy. Empty this file or remove the tix-rebase-state-v2 comment to cancel. -->"
        }
        (true, false) => {
            "<!-- Rebase help follows. Saving unchanged rebases onto the updated base; empty this file or remove the tix-rebase-state-v2 comment to cancel. -->"
        }
        (true, true) => {
            "<!-- Rebase help follows. Saving unchanged rebases onto the updated base and applies pending commits on the @ ancestry: that ancestry is replayed now and other forks stay lazy. Empty this file or remove the tix-rebase-state-v2 comment to cancel. -->"
        }
    }
}

fn write_state(out: &mut Vec<u8>, state: &State) {
    out.extend_from_slice(STATE_START.as_bytes());
    out.extend_from_slice(format!("base {}\nonto {}\n", state.base, state.onto).as_bytes());
    for tip in &state.tips {
        out.extend_from_slice(format!("tip {tip}\n").as_bytes());
    }
    for id in &state.scope {
        out.extend_from_slice(format!("scope {id}\n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "marker-required {}\ncheckout-allowed {}\n",
            state.marker_required, state.checkout_allowed
        )
        .as_bytes(),
    );
    if let Some(name) = &state.head_ref {
        out.extend_from_slice(b"head-ref ");
        out.extend_from_slice(gix::quote::ansi_c::quote(name.as_bstr()).as_ref());
        out.push(b'\n');
    }
    out.extend_from_slice(format!("edit-refs {}\n", state.edit_refs).as_bytes());
    for reference in &state.expected_refs {
        let name = gix::quote::ansi_c::quote(reference.name.as_bstr());
        let old = reference.old.map_or_else(|| "-".into(), |id| id.to_string());
        out.extend_from_slice(
            format!(
                "ref {} {} {} {} {}\n",
                old,
                reference.target,
                reference.follows_tip,
                reference.editable,
                name.to_str_lossy()
            )
            .as_bytes(),
        );
    }
    if let Some(id) = state.resolved {
        out.extend_from_slice(format!("resolved {id}\n").as_bytes());
    }
    for id in &state.continuation_sources {
        out.extend_from_slice(format!("continuation-source {id}\n").as_bytes());
    }
    out.extend_from_slice(STATE_END.as_bytes());
    out.push(b'\n');
}

fn write_refs_at(out: &mut Vec<u8>, refs: &[rebase::ExpectedRef], target: ObjectId) -> Result<()> {
    let names = refs
        .iter()
        .filter(|reference| reference.editable && reference.target == target)
        .map(|reference| &reference.name)
        .collect::<Vec<_>>();
    write_ref_line(out, refs, names)
}

fn write_plan_refs_at(out: &mut Vec<u8>, refs: &[rebase::ExpectedRef], target: rebase::PlanParent) -> Result<()> {
    let names = refs
        .iter()
        .filter(|reference| reference.placement == Some(target))
        .map(|reference| &reference.name)
        .collect::<Vec<_>>();
    write_ref_line(out, refs, names)
}

fn write_ref_line(out: &mut Vec<u8>, refs: &[rebase::ExpectedRef], mut names: Vec<&gix::refs::FullName>) -> Result<()> {
    if names.is_empty() {
        return Ok(());
    }
    names.sort();
    out.push(b'(');
    for (index, name) in names.into_iter().enumerate() {
        if index > 0 {
            out.extend_from_slice(b", ");
        }
        let display = ref_display_name(name, refs);
        out.extend_from_slice(gix::quote::ansi_c::quote(display.as_bstr()).as_ref());
    }
    out.extend_from_slice(b")\n");
    Ok(())
}

fn ref_display_name(name: &gix::refs::FullName, refs: &[rebase::ExpectedRef]) -> BString {
    let short = name.shorten();
    if refs
        .iter()
        .filter(|candidate| candidate.editable && candidate.name.shorten() == short)
        .count()
        > 1
    {
        name.as_bstr().to_owned()
    } else {
        short.to_owned()
    }
}

fn commit_states(
    repo: &gix::Repository,
    enrichments: &mut gix::note::Platform,
    tree_enrichments: &mut gix::note::Platform,
    id: ObjectId,
) -> Result<String> {
    let commit = repo
        .find_commit(id)
        .context("could not load a commit state for the rebase todo")?
        .decode()
        .context("could not decode a commit state for the rebase todo")?
        .into_owned()
        .context("could not own a commit state for the rebase todo")?;
    let pending = commit.extra_headers.iter().any(|(name, _)| name == "tix-rebase-parent");
    let mut empty_signature = false;
    let mut signature = false;
    for (name, value) in &commit.extra_headers {
        if name != "gpgsig" && name != "gpgsig-sha256" {
            continue;
        }
        if value.is_empty() {
            empty_signature = true;
        } else {
            signature = true;
        }
    }
    let stashed = repo
        .try_find_reference(super::stash::reference(id)?.as_ref())?
        .is_some();
    let enrichment =
        crate::change_id::for_commit(repo, id).and_then(|change_id| crate::enrich::load(enrichments, change_id));
    let enrichment = match enrichment {
        Ok(enrichment) => enrichment,
        Err(err) => {
            tracing::warn!(commit_id = %id, error = %err, "ignored malformed tix enrichment");
            crate::enrich::Enrichment::default()
        }
    };
    let tree_enrichment =
        crate::enrich::tree_id(repo, id).and_then(|tree_id| crate::enrich::load_tree(tree_enrichments, tree_id));
    let tree_enrichment = match tree_enrichment {
        Ok(enrichment) => enrichment,
        Err(err) => {
            tracing::warn!(commit_id = %id, error = %err, "ignored malformed tix tree enrichment");
            crate::enrich::TreeEnrichment::default()
        }
    };
    let marker = crate::enrich::marker(enrichment.todo, enrichment.note.is_some(), tree_enrichment.checks_pass);
    let mut out = Vec::with_capacity(6);
    if !marker.is_empty() {
        out.push(marker);
    }
    if pending {
        out.push("↻");
    }
    if empty_signature {
        out.push("◌");
    }
    if signature {
        out.push("◐");
    }
    if !empty_signature && !signature {
        out.push("○");
    }
    if stashed {
        out.push("🎁");
    }
    Ok(format!("{} ", out.join(" ")))
}

fn anchor_title(repo: &gix::Repository, id: ObjectId) -> Result<String> {
    let message = repo
        .find_commit(id)
        .context("could not load the rebase anchor")?
        .message_raw()
        .context("could not decode the rebase anchor message")?
        .to_owned();
    let mut notes = repo.notes().context("could not open Git notes for the rebase anchor")?;
    let has_notes = !notes.get(id).context("could not load rebase anchor notes")?.is_empty();
    let mut out = String::new();
    if crate::history::contains_agent_marker(&message) {
        out.push_str("[A] ");
    }
    if has_notes {
        out.push_str("[N] ");
    }
    out.push_str(
        &gix::objs::commit::MessageRef::from_bytes(&message)
            .summary()
            .to_str_lossy(),
    );
    Ok(out)
}

fn write_fork_heading(
    out: &mut Vec<u8>,
    repo: &gix::Repository,
    id: ObjectId,
    annotation: Option<(&str, &str)>,
    show_change_ids: bool,
) -> Result<()> {
    out.extend_from_slice(format!("fork {}", short(repo, id, show_change_ids)?).as_bytes());
    if let Some((kind, title)) = annotation {
        out.extend_from_slice(format!(" ({kind}) {title}").as_bytes());
    }
    out.push(b'\n');
    Ok(())
}

fn write_bottom_up(out: &mut Vec<u8>, body: &[u8]) -> Result<()> {
    let body = std::str::from_utf8(body).context("generated rebase todo is not UTF-8")?;
    let width = body
        .lines()
        .map(|line| {
            let width = Line::raw(line).width();
            if line.starts_with("fork ") { width + 10 } else { width }
        })
        .max()
        .unwrap_or_default();
    for line in body.lines().rev() {
        if line.starts_with("fork ") {
            let label_width = Line::raw(line).width();
            let rails = width.saturating_sub(label_width + 2).max(8);
            let left = rails / 2;
            let right = rails - left;
            out.extend_from_slice(format!("{} {line} {}\n", "─".repeat(left), "─".repeat(right)).as_bytes());
        } else {
            out.extend_from_slice(line.as_bytes());
            out.push(b'\n');
        }
    }
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

fn short(repo: &gix::Repository, id: ObjectId, show_change_id: bool) -> Result<String> {
    if show_change_id {
        crate::change_id::display_short(repo, id).context("could not format a rebase todo ID")
    } else {
        Ok(id
            .attach(repo)
            .shorten()
            .context("could not shorten a rebase todo ID")?
            .to_string())
    }
}

fn parse_state(repo: &gix::Repository, input: &str) -> Result<Option<State>> {
    let Some(start) = input.find(STATE_START) else {
        if input.contains("<!-- tix-rebase-state-") {
            anyhow::bail!("the rebase todo uses an unsupported state version");
        }
        return Ok(None);
    };
    let body = &input[start + STATE_START.len()..];
    let end = body
        .find(STATE_CLOSE)
        .context("the rebase state anchor is not closed")?;
    if body[end + STATE_CLOSE.len()..].contains(STATE_START) {
        anyhow::bail!("the rebase todo contains more than one state anchor");
    }
    let mut base = None;
    let mut onto = None;
    let mut tips = Vec::new();
    let mut scope = Vec::new();
    let mut marker_required = None;
    let mut checkout_allowed = None;
    let mut head_ref = None;
    let mut edit_refs = false;
    let mut expected_refs = Vec::new();
    let mut resolved = None;
    let mut continuation_sources = Vec::new();
    for line in body[..end].lines() {
        let (key, value) = line.split_once(' ').context("a rebase state line has no value")?;
        match key {
            "base" => {
                if base.replace(ObjectId::from_hex(value.as_bytes())?).is_some() {
                    anyhow::bail!("the rebase state has more than one base");
                }
            }
            "onto" => {
                if onto.replace(ObjectId::from_hex(value.as_bytes())?).is_some() {
                    anyhow::bail!("the rebase state has more than one onto target");
                }
            }
            "tip" => tips.push(ObjectId::from_hex(value.as_bytes())?),
            "scope" => scope.push(ObjectId::from_hex(value.as_bytes())?),
            "marker-required" => {
                if marker_required.replace(value.parse()?).is_some() {
                    anyhow::bail!("the rebase state repeats marker-required");
                }
            }
            "checkout-allowed" => {
                if checkout_allowed.replace(value.parse()?).is_some() {
                    anyhow::bail!("the rebase state repeats checkout-allowed");
                }
            }
            "head-ref" => {
                let encoded = value.as_bytes().as_bstr();
                let (name, consumed) = gix::quote::ansi_c::undo(encoded)
                    .map_err(gix::Exn::into_error)
                    .context("could not unquote the recorded HEAD ref")?;
                if !encoded[consumed..].trim().is_empty() {
                    anyhow::bail!("the recorded HEAD ref has trailing data");
                }
                let name = gix::refs::FullName::try_from(name.as_ref()).context("the recorded HEAD ref is invalid")?;
                if head_ref.replace(name).is_some() {
                    anyhow::bail!("the rebase state repeats its HEAD ref");
                }
            }
            "edit-refs" => edit_refs = value.parse()?,
            "ref" => {
                let (old, value) = value.split_once(' ').context("a captured ref has no target")?;
                let (target, value) = value.split_once(' ').context("a captured ref has no follow mode")?;
                let old = (old != "-").then(|| ObjectId::from_hex(old.as_bytes())).transpose()?;
                let target = ObjectId::from_hex(target.as_bytes())?;
                let (follows_tip, value) = value.split_once(' ').context("a captured ref has no name")?;
                let follows_tip = follows_tip.parse()?;
                let (editable, name) = value
                    .split_once(' ')
                    .and_then(|(editable, name)| editable.parse::<bool>().ok().map(|editable| (editable, name)))
                    .unwrap_or((false, value));
                let encoded_name = name.as_bytes().as_bstr();
                let (name, consumed) = gix::quote::ansi_c::undo(encoded_name)
                    .map_err(gix::Exn::into_error)
                    .context("could not unquote a captured ref name")?;
                if !encoded_name[consumed..].trim().is_empty() {
                    anyhow::bail!("a captured ref name has trailing data");
                }
                let name = gix::refs::FullName::try_from(name.as_ref()).context("a captured ref name is invalid")?;
                expected_refs.push(rebase::ExpectedRef {
                    name,
                    old,
                    target,
                    new: old,
                    follows_tip,
                    editable,
                    placement: None,
                });
            }
            "resolved" => {
                if resolved.replace(ObjectId::from_hex(value.as_bytes())?).is_some() {
                    anyhow::bail!("the rebase state repeats its resolved conflict");
                }
            }
            "continuation-source" => continuation_sources.push(ObjectId::from_hex(value.as_bytes())?),
            _ => anyhow::bail!("unsupported rebase state field {key:?}"),
        }
    }
    let state = State {
        base: base.context("the rebase state has no base")?,
        onto: onto.context("the rebase state has no onto target")?,
        tips,
        scope,
        marker_required: marker_required.context("the rebase state has no marker requirement")?,
        checkout_allowed: checkout_allowed.context("the rebase state has no checkout capability")?,
        head_ref,
        edit_refs,
        expected_refs,
        resolved,
        continuation_sources,
    };
    validate_state(repo, &state)?;
    Ok(Some(state))
}

fn validate_state(repo: &gix::Repository, state: &State) -> Result<()> {
    repo.find_commit(state.base)
        .context("could not find the recorded rebase base")?;
    repo.find_commit(state.onto)
        .context("could not find the recorded rebase target")?;
    let scope: HashSet<_> = state.scope.iter().copied().collect();
    let continuation_sources: HashSet<_> = state.continuation_sources.iter().copied().collect();
    if scope.len() != state.scope.len() {
        anyhow::bail!("the rebase state contains duplicate scope commits");
    }
    if state.tips.iter().copied().collect::<HashSet<_>>().len() != state.tips.len() {
        anyhow::bail!("the rebase state contains duplicate tips");
    }
    let mut refs = HashSet::new();
    for reference in &state.expected_refs {
        if !refs.insert(reference.name.as_bstr()) {
            anyhow::bail!("the rebase state contains duplicate refs");
        }
        if !scope.contains(&reference.target) && reference.target != state.base && reference.target != state.onto {
            anyhow::bail!("a captured ref does not logically point into the rebase scope");
        }
    }
    if let Some(name) = &state.head_ref
        && !state
            .expected_refs
            .iter()
            .any(|reference| reference.editable && reference.name == *name)
    {
        anyhow::bail!("the recorded HEAD ref is not editable");
    }
    for tip in &state.tips {
        repo.find_commit(*tip).context("could not find a recorded rebase tip")?;
    }
    for id in &state.scope {
        let commit = repo
            .find_commit(*id)
            .context("could not find a recorded scope commit")?;
        let parent = commit
            .parent_ids()
            .next()
            .map(gix::Id::detach)
            .context("a recorded scope commit has no parent")?;
        if parent != state.base && !scope.contains(&parent) && !continuation_sources.contains(id) {
            anyhow::bail!("a recorded scope commit is disconnected from the rebase base");
        }
    }
    if state.resolved.is_some_and(|id| !scope.contains(&id)) {
        anyhow::bail!("the resolved conflict is outside the rebase scope");
    }
    if !continuation_sources.is_subset(&scope) {
        anyhow::bail!("a continuation source is outside the rebase scope");
    }
    Ok(())
}

pub(crate) fn parse(repo: &gix::Repository, edited: &[u8]) -> Result<Option<Parsed>> {
    repo.head()?.id().context("rebase todos require a born HEAD")?;
    let input = std::str::from_utf8(edited).context("the rebase todo is not UTF-8")?;
    let Some(mut state) = parse_state(repo, input)? else {
        return Ok(None);
    };
    let scope: HashSet<_> = state.scope.iter().copied().collect();
    let mut picked = HashMap::<ObjectId, usize>::new();
    let mut steps = Vec::<rebase::PlanStep>::new();
    let mut cursor = None;
    let mut checkout_target = None;
    let mut explicit_checkout_reference = None;
    let mut command_marker = false;
    let mut ref_targets = HashMap::new();
    let mut sections = 0;
    let mut section_has_commit = false;
    let mut section_last_step = None;
    let mut in_comment = false;
    let mut editable = Vec::new();
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
        editable.push(line);
    }

    for line in editable.into_iter().rev() {
        if line.starts_with('─') && line.ends_with('─') {
            let target = line
                .trim_matches('─')
                .trim()
                .strip_prefix("fork ")
                .context("a fork separator needs a fork ID")?;
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
            section_last_step = None;
            continue;
        }
        if line.starts_with('(') && line.ends_with(')') {
            let target = cursor.context("a reference line must follow a fork or command")?;
            for (marked, value) in parse_ref_line(line)? {
                let name = resolve_ref_name(repo, &mut state.expected_refs, value.as_bstr())?;
                if ref_targets.insert(name.clone(), target).is_some() {
                    anyhow::bail!("a reference is placed more than once");
                }
                if marked {
                    if !state.checkout_allowed || repo.workdir().is_none() {
                        anyhow::bail!("the rebase todo cannot select a checkout without a worktree");
                    }
                    if explicit_checkout_reference.replace((name, target)).is_some() {
                        anyhow::bail!("the rebase todo contains more than one @ reference");
                    }
                }
            }
            section_has_commit = true;
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
            if std::mem::replace(&mut command_marker, true) {
                anyhow::bail!("the rebase todo contains more than one @ command");
            }
            if !state.checkout_allowed || repo.workdir().is_none() {
                anyhow::bail!("the rebase todo cannot select a checkout without a worktree");
            }
        }
        if verb == "squash" {
            let index = section_last_step.context("a squash must follow a command in the same fork")?;
            let id = resolve_commit(
                repo,
                value.split_whitespace().next().context("a squash needs a commit ID")?,
            )?;
            if !scope.contains(&id) {
                anyhow::bail!("a squash is outside the editable history");
            }
            if picked.insert(id, index).is_some() {
                anyhow::bail!("a commit is picked more than once");
            }
            steps[index].squash.push(id);
            if marked {
                let target = rebase::PlanParent::Step(index);
                if checkout_target.is_some_and(|checkout| checkout != target) {
                    anyhow::bail!("the @ command and @ reference point to different results");
                }
                checkout_target = Some(target);
            }
            section_has_commit = true;
            continue;
        }
        let parent = cursor.context("the first todo command must follow a fork heading")?;
        let commit = match verb {
            "pick" => {
                let value = value.split_whitespace().next().context("a pick needs a commit ID")?;
                let resolved_id = state.resolved;
                let full_null = resolved_id.is_some_and(|id| {
                    value.len() == id.kind().len_in_bytes() * 2 && value.bytes().all(|byte| byte == b'0')
                });
                let (id, resolved) = if full_null {
                    (
                        resolved_id.context("a null pick has no materialized conflict state")?,
                        true,
                    )
                } else {
                    (resolve_commit(repo, value)?, false)
                };
                if !scope.contains(&id) {
                    anyhow::bail!("a pick is outside the editable history");
                }
                if picked.contains_key(&id) {
                    anyhow::bail!("a commit is picked more than once");
                }
                if resolved {
                    rebase::PlanCommit::Resolved(id)
                } else {
                    rebase::PlanCommit::Pick(id)
                }
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
        if let rebase::PlanCommit::Pick(id) | rebase::PlanCommit::Resolved(id) = commit {
            picked.insert(id, index);
        }
        steps.push(rebase::PlanStep {
            parent,
            commit,
            squash: Vec::new(),
        });
        cursor = Some(rebase::PlanParent::Step(index));
        section_last_step = Some(index);
        if marked {
            let target = rebase::PlanParent::Step(index);
            if checkout_target.is_some_and(|checkout| checkout != target) {
                anyhow::bail!("the @ command and @ reference point to different results");
            }
            checkout_target = Some(target);
        }
        section_has_commit = true;
    }
    if sections == 0 {
        anyhow::bail!("the rebase todo has no fork heading");
    }
    if sections > 1 && !section_has_commit {
        anyhow::bail!("the last fork section contains no commits");
    }
    if state.marker_required && checkout_target.is_none() {
        anyhow::bail!("the current checkout marker must be retained");
    }
    let checkout_reference = match (checkout_target, explicit_checkout_reference) {
        (Some(target), Some((name, reference_target))) => {
            if target != reference_target {
                anyhow::bail!("the @ command and @ reference point to different results");
            }
            Some(name)
        }
        (None, Some(_)) => anyhow::bail!("an @ reference requires an @ command at the same result"),
        (Some(target), None) => state
            .head_ref
            .take()
            .filter(|name| ref_targets.get(name) == Some(&target)),
        (None, None) => None,
    };
    if state.edit_refs {
        for reference in &mut state.expected_refs {
            if reference.editable {
                reference.new = None;
                reference.placement = ref_targets.remove(&reference.name);
            }
        }
    }
    let checkout = checkout_target.map(|target| rebase::PlanCheckout {
        target,
        reference: checkout_reference,
    });
    Ok(Some(Parsed {
        plan: rebase::Plan {
            base: state.onto,
            scope: state.scope,
            steps,
            checkout,
            expected_refs: state.expected_refs,
        },
        tips: state.tips,
    }))
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

fn parse_ref_line(line: &str) -> Result<Vec<(bool, BString)>> {
    let body = line
        .strip_prefix('(')
        .and_then(|line| line.strip_suffix(')'))
        .context("a reference line must be enclosed in parentheses")?;
    let mut ranges = Vec::new();
    let mut start = 0;
    let mut quoted = false;
    let mut escaped = false;
    for (index, byte) in body.bytes().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        match byte {
            b'\\' if quoted => escaped = true,
            b'"' => quoted = !quoted,
            b',' if !quoted => {
                ranges.push(&body[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    if quoted || escaped {
        anyhow::bail!("a quoted reference name is not closed");
    }
    ranges.push(&body[start..]);
    let mut out = Vec::with_capacity(ranges.len());
    for item in ranges {
        let item = item.trim();
        if item.is_empty() {
            anyhow::bail!("a reference line contains an empty name");
        }
        let (marked, item) = item.strip_prefix('@').map_or((false, item), |item| (true, item));
        let encoded = item.as_bytes().as_bstr();
        let (name, consumed) = gix::quote::ansi_c::undo(encoded)
            .map_err(gix::Exn::into_error)
            .context("could not unquote a reference name")?;
        if !encoded[consumed..].trim().is_empty() {
            anyhow::bail!("a reference name has trailing data");
        }
        if name.is_empty() {
            anyhow::bail!("a reference name is empty");
        }
        out.push((marked, name.into_owned()));
    }
    Ok(out)
}

fn resolve_ref_name(
    repo: &gix::Repository,
    refs: &mut Vec<rebase::ExpectedRef>,
    input: &gix::bstr::BStr,
) -> Result<gix::refs::FullName> {
    let mut matches = refs
        .iter()
        .filter(|reference| reference.editable && ref_display_name(&reference.name, refs).as_bstr() == input)
        .map(|reference| reference.name.clone());
    if let Some(name) = matches.next() {
        if matches.next().is_some() {
            anyhow::bail!("the shortened reference name is ambiguous");
        }
        return Ok(name);
    }
    let full = if input.starts_with(b"refs/") {
        input.to_owned()
    } else {
        let mut full = BString::from("refs/heads/");
        full.extend_from_slice(input);
        full
    };
    let name = gix::refs::FullName::try_from(full).context("the todo contains an invalid reference name")?;
    if name.as_bstr().starts_with(crate::history::PIN_PREFIX)
        || name.as_bstr().starts_with(crate::history::STASH_PREFIX)
        || name.as_bstr().starts_with(crate::history::REVIEW_PREFIX)
        || super::undo::is_queue_ref(name.as_bstr())
        || matches!(
            name.category(),
            Some(gix::refs::Category::Tag | gix::refs::Category::RemoteBranch)
        )
    {
        anyhow::bail!("the todo cannot edit this reference namespace");
    }
    if refs.iter().any(|reference| reference.name == name) {
        anyhow::bail!("the todo cannot edit a hidden reference");
    }
    let old = repo
        .try_find_reference(name.as_ref())?
        .map(|reference| {
            reference
                .try_id()
                .map(gix::Id::detach)
                .context("an existing symbolic reference outside the editable history cannot be moved")
        })
        .transpose()?;
    refs.push(rebase::ExpectedRef {
        name: name.clone(),
        old,
        target: old.unwrap_or(repo.head_id()?.detach()),
        new: old,
        follows_tip: false,
        editable: true,
        placement: None,
    });
    Ok(name)
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    fn repo() -> gix_testtools::Result<(gix_testtools::tempfile::TempDir, gix::Repository)> {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["core.abbrev=7", "user.name=todo author", "user.email=todo@example.com"],
        )?;
        Ok((fixture, repo))
    }

    #[test]
    fn unchanged_notices_cover_every_reason_to_apply_or_cancel() {
        for (updated, pending, expected) in [
            (false, false, "Saving unchanged is a no-op"),
            (
                false,
                true,
                "Pending commits on the @ ancestry make saving unchanged apply this todo",
            ),
            (true, false, "Saving unchanged rebases onto the updated base"),
            (
                true,
                true,
                "Saving unchanged rebases onto the updated base and applies pending commits on the @ ancestry",
            ),
        ] {
            let notice = unchanged_notice(updated, pending);
            assert!(notice.contains(expected), "the notice explains its execution mode");
            assert!(
                notice.contains("remove the tix-rebase-state-v2 comment to cancel"),
                "every notice explains explicit cancellation"
            );
        }
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
                    info: "2000-01-03 author tip".into(),
                },
                Commit {
                    id: middle,
                    parents: vec![base],
                    info: "2000-01-02 author middle * _ [markdown] <view> `code` \\ raw".into(),
                },
            ],
        ))
    }

    fn prepare_test(
        repo: &gix::Repository,
        base: ObjectId,
        onto: ObjectId,
        commits: &[Commit],
        _head: Option<ObjectId>,
    ) -> Result<Prepared> {
        prepare(repo, base, onto, commits, &[], OntoKind::UpdatedBase, true)
    }

    fn parse_plan(repo: &gix::Repository, document: &[u8]) -> Result<rebase::Plan> {
        Ok(parse(repo, document)?.context("the test todo was cancelled")?.plan)
    }

    fn with_state(prepared: &Prepared, commands: &str) -> Vec<u8> {
        let document = std::str::from_utf8(&prepared.document).expect("generated todo is UTF-8");
        let start = document.find(STATE_START).expect("generated todo has state");
        let end = document[start..].find(STATE_CLOSE).expect("generated state is closed") + start + STATE_CLOSE.len();
        let mut bottom_up = Vec::new();
        write_bottom_up(&mut bottom_up, commands.as_bytes()).expect("test todo commands are UTF-8");
        let bottom_up = std::str::from_utf8(&bottom_up).expect("rendered test todo is UTF-8");
        format!("{}\n{bottom_up}", &document[start..end]).into_bytes()
    }

    #[test]
    fn markdown_flows_from_tip_to_base_and_uses_repository_abbreviations() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let (base, middle, tip, commits) = commits(&repo)?;
        repo.reference(
            super::super::stash::reference(middle)?,
            tip,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "test todo stash marker",
        )?;
        let prepared = prepare_test(&repo, base, base, &commits, Some(tip))?;
        assert!(!prepared.apply_unchanged);
        let document = String::from_utf8(prepared.document.clone())?;
        assert!(
            document.starts_with(unchanged_notice(false, false)),
            "the first line explains that saving unchanged is a no-op"
        );
        assert!(document.contains(STATE_START), "the todo carries its transaction state");
        assert!(document.contains(&format!(
            "# Rebase from `{}`",
            crate::change_id::display_short(&repo, base)?
        )));
        assert!(document.contains(&format!(
            "fork {} (base) base",
            crate::change_id::display_short(&repo, base)?
        )));
        let middle = document.find("`pick ").expect("the oldest pick is shown");
        let tip = document.find("`@pick ").expect("HEAD is marked");
        let base = document.find("fork ").expect("the base separator is shown");
        assert!(tip < middle && middle < base, "the todo grows upward from its base");
        let separator = document
            .lines()
            .find(|line| line.contains("fork "))
            .expect("separator is present");
        assert!(separator.starts_with('─') && separator.ends_with('─'));
        let plan = &document[..document.find("# Rebase todo help").expect("help is present")];
        let width = plan
            .lines()
            .filter(|line| line.starts_with('`') || line.starts_with('(') || line.starts_with('─'))
            .map(|line| Line::raw(line).width())
            .max()
            .expect("the editable plan has lines");
        assert_eq!(
            Line::raw(separator).width(),
            width,
            "the separator spans the widest plan line"
        );
        let left = separator.chars().take_while(|ch| *ch == '─').count();
        let right = separator.chars().rev().take_while(|ch| *ch == '─').count();
        assert!(
            left >= 4 && right >= 4 && left.abs_diff(right) <= 1,
            "the label is centered"
        );
        assert!(
            document.contains("middle * _ [markdown] <view> `code` \\ raw"),
            "display metadata is emitted verbatim"
        );
        assert!(
            document.find("# Rebase todo help").expect("help is present") > tip,
            "complete instructions follow the editable todo"
        );
        assert!(
            document.find(STATE_START).expect("state is present")
                > document.find("# Rebase todo help").expect("help is present"),
            "transaction state follows the complete help"
        );
        assert!(document.ends_with("-->\n"), "the trailing state is a Markdown comment");
        assert!(
            document.contains("○"),
            "unsigned commits carry the documented status symbol"
        );
        assert!(
            document.contains("🎁"),
            "stashed commits carry a display-only gift marker"
        );
        let mut edited_symbols = document.clone();
        let marker = edited_symbols.find("🎁").expect("the command carries a stash marker");
        edited_symbols.replace_range(marker..marker + "🎁".len(), "changed-state");
        parse_plan(&repo, edited_symbols.as_bytes())?;
        let plan = parse_plan(&repo, &prepared.document)?;
        assert_eq!(
            plan.checkout.as_ref().and_then(|checkout| checkout.reference.as_ref()),
            Some(&"refs/heads/main".try_into()?),
            "the generated @ command retains the implicitly attached branch"
        );
        Ok(())
    }

    #[test]
    fn enrichment_markers_precede_commit_states_in_initial_and_continuation_todos() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let (base, middle, tip, commits) = commits(&repo)?;
        crate::enrich::ensure_todo(&repo, middle, true)?;
        crate::enrich::set_note(&repo, middle, Some(b"follow up"))?;
        crate::enrich::ensure_checks_pass(&repo, middle, true)?;

        let id = crate::change_id::display_short(&repo, middle)?;
        let prepared = prepare_test(&repo, base, base, &commits, Some(tip))?;
        let document = String::from_utf8(prepared.document)?;
        assert!(
            document.contains(&format!("`pick {id}` 🚧📝✔️ ○ 2000-01-02")),
            "commit and tree enrichments precede the unsigned signature state"
        );
        assert!(
            document.contains("`🚧` means the commit is a todo, `📝` it has a note, `✔️` its tree passed checks"),
            "the embedded legend explains enrichment states"
        );

        repo.reference(
            super::super::stash::reference(middle)?,
            tip,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "test enriched todo stash ordering",
        )?;
        let prepared = prepare_continuation(
            &repo,
            &rebase::Plan {
                base,
                scope: vec![middle],
                steps: vec![rebase::PlanStep {
                    parent: rebase::PlanParent::Existing(base),
                    commit: rebase::PlanCommit::Pick(middle),
                    squash: Vec::new(),
                }],
                checkout: None,
                expected_refs: Vec::new(),
            },
            vec![middle],
            true,
        )?;
        let document = String::from_utf8(prepared.document)?;
        assert!(
            document.contains(&format!("`pick {id}` 🚧📝✔️ ○ 🎁 middle")),
            "continuation todos retain enrichment ordering before stash state"
        );
        Ok(())
    }

    #[test]
    fn malformed_enrichments_do_not_prevent_todo_generation() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let (base, middle, tip, commits) = commits(&repo)?;
        let change_id = crate::change_id::for_commit(&repo, middle)?;
        let reference: gix::refs::FullName = crate::enrich::REF_NAME.try_into()?;
        repo.notes()?
            .replace_at_ref(reference.as_ref(), ObjectId::from(change_id), b"[commit")?;
        let tree_id = crate::enrich::tree_id(&repo, middle)?;
        let reference: gix::refs::FullName = crate::enrich::TREE_REF_NAME.try_into()?;
        repo.notes()?.replace_at_ref(reference.as_ref(), tree_id, b"[tree")?;

        let prepared = prepare_test(&repo, base, base, &commits, Some(tip))?;
        let document = String::from_utf8(prepared.document)?;
        let line = document
            .lines()
            .find(|line| line.contains("2000-01-02 author middle"))
            .expect("the malformed enrichment commit remains in the todo");
        assert!(line.contains(" ○ "), "ordinary commit states remain visible");
        for marker in ["🚧", "📝", "✔️"] {
            assert!(!line.contains(marker), "malformed enrichments are ignored");
        }
        Ok(())
    }

    #[test]
    fn state_round_trips_non_utf8_ref_names_and_controls_cancellation() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let (base, middle, _tip, _commits) = commits(&repo)?;
        let name = gix::refs::FullName::try_from(BString::from(vec![
            b'r', b'e', b'f', b's', b'/', b'h', b'e', b'a', b'd', b's', b'/', 0xff,
        ]))?;
        let state = State {
            base,
            onto: base,
            tips: vec![middle],
            scope: vec![middle],
            marker_required: false,
            checkout_allowed: true,
            head_ref: Some(name.clone()),
            edit_refs: true,
            expected_refs: vec![rebase::ExpectedRef {
                name: name.clone(),
                old: Some(middle),
                target: middle,
                new: Some(middle),
                follows_tip: true,
                editable: true,
                placement: None,
            }],
            resolved: None,
            continuation_sources: Vec::new(),
        };
        let mut document = Vec::new();
        write_state(&mut document, &state);
        let document = String::from_utf8(document)?;
        assert!(
            document.contains(r#""refs/heads/\377""#),
            "non-UTF-8 names use Git quoting"
        );
        let parsed = parse_state(&repo, &document)?.context("state is present")?;
        assert_eq!(parsed.expected_refs[0].name, name, "quoted names round-trip losslessly");
        let old = document.replacen("tix-rebase-state-v2", "tix-rebase-state-v1", 1);
        let err = match parse_state(&repo, &old) {
            Ok(_) => panic!("v1 order would be ambiguous under v2 semantics"),
            Err(err) => err,
        };
        assert!(format!("{err:#}").contains("unsupported state version"));
        assert_eq!(
            parsed.head_ref,
            Some(name),
            "the attached branch round-trips losslessly"
        );

        assert!(parse(&repo, b"")?.is_none(), "empty input cancels");
        assert!(parse(&repo, b"pick deadbeef")?.is_none(), "removing the anchor cancels");
        assert!(
            parse(&repo, b"<!-- tix-rebase-state-v2\n-->").is_err(),
            "an unsupported present anchor is rejected"
        );
        Ok(())
    }

    #[test]
    fn an_unchanged_todo_replays_pending_commits_with_normal_plan_semantics() -> gix_testtools::Result {
        let (fixture, repo) = repo()?;
        let (base, middle, old_tip, _) = commits(&repo)?;
        let graph = super::super::loaded_graph(&repo)?;
        let mut commit = repo.find_commit(middle)?.decode()?.into_owned()?;
        commit.tree = repo.find_commit(base)?.tree_id()?.detach();
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["checkout", "-q", "--detach", &base.to_string()])
                .status()?
                .success(),
            "the pending stack is prepared away from the current checkout"
        );
        let marked_outcome = rebase::perform(
            &repo,
            &graph,
            rebase::Edit::Replace { target: middle, commit },
            rebase::Signature::InvalidateExisting,
            rebase::Tree::LeaveAsIsAndMark,
        )?
        .complete()?;
        let marked = marked_outcome
            .selected
            .expect("the pending replacement selects its rewritten commit");
        let tip = marked_outcome.map(old_tip).context("the pending tip is retained")?;
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["checkout", "-q", "main"])
                .status()?
                .success(),
            "the pending branch is checked out before preparing its todo"
        );
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
        let prepared = prepare_test(&repo, base, base, &commits, Some(tip))?;
        assert!(
            prepared.apply_unchanged,
            "pending commits make an unchanged todo actionable"
        );
        assert!(
            prepared.document.starts_with(unchanged_notice(false, true).as_bytes()),
            "the first line explains why the unchanged todo remains actionable"
        );
        let document = prepared.document.clone();
        let plan = parse_plan(&repo, &document)?;
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
    fn pending_commits_outside_the_checkout_ancestry_do_not_apply_an_unchanged_todo() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let (base, middle, tip, mut commits) = commits(&repo)?;
        let mut sibling = repo.find_commit(tip)?.decode()?.into_owned()?;
        sibling.parents = [middle].into_iter().collect();
        sibling.message = "pending sibling".into();
        sibling
            .extra_headers
            .push(("tix-rebase-parent".into(), middle.to_hex().to_string().into()));
        let sibling = repo.write_object(&sibling)?.detach();
        commits.push(Commit {
            id: sibling,
            parents: vec![middle],
            info: "pending sibling".into(),
        });

        let prepared = prepare_test(&repo, base, base, &commits, Some(tip))?;
        assert!(
            !prepared.apply_unchanged,
            "pending commits on another fork must not replay the clean checkout ancestry"
        );
        assert!(
            prepared.document.starts_with(unchanged_notice(false, false).as_bytes()),
            "the first line identifies an unchanged todo as a no-op"
        );
        assert!(
            prepared
                .document
                .windows("↻".len())
                .any(|window| window == "↻".as_bytes()),
            "the pending sibling remains visible in the todo"
        );
        Ok(())
    }

    #[test]
    fn descendant_forks_stay_terse() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let (base, middle, tip, mut commits) = commits(&repo)?;
        let mut sibling = repo.find_commit(tip)?.decode()?.into_owned()?;
        sibling.parents = [middle].into_iter().collect();
        sibling.message = "sibling".into();
        let sibling = repo.write_object(&sibling)?.detach();
        commits.insert(
            0,
            Commit {
                id: sibling,
                parents: vec![middle],
                info: "sibling title".into(),
            },
        );

        let prepared = prepare_test(&repo, base, base, &commits, Some(tip))?;
        let document = String::from_utf8(prepared.document.clone())?;
        assert!(document.contains(&format!(
            "fork {} (base) base",
            crate::change_id::display_short(&repo, base)?
        )));
        assert!(
            document.contains(&format!("fork {} ", crate::change_id::display_short(&repo, middle)?)),
            "a fork within the editable tree has no external-anchor annotation"
        );
        let plan = parse_plan(&repo, document.as_bytes())?;
        assert_eq!(plan.steps.len(), 3, "display annotations do not alter the plan");
        Ok(())
    }

    #[test]
    fn shared_updated_base_refs_are_written_once_during_review() -> gix_testtools::Result {
        let (fixture, repo) = repo()?;
        let (_old_base, base, reviewed, _) = commits(&repo)?;
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["switch", "-q", "-c", "topic"])
                .status()?
                .success(),
            "the review return branch is prepared"
        );
        let graph = super::super::loaded_graph(&repo)?;
        drop(repo);
        let started = super::super::review::start(fixture.path(), false, &graph, reviewed, base)?;
        assert!(started.checkout_error.is_none(), "the review checkout succeeds");

        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["core.abbrev=7", "user.name=todo author", "user.email=todo@example.com"],
        )?;
        let mut updated = repo.find_commit(base)?.decode()?.into_owned()?;
        updated.parents = [base].into_iter().collect();
        updated.message = "updated base".into();
        let updated = repo.write_object(&updated)?.detach();
        repo.reference(
            "refs/heads/main",
            updated,
            gix::refs::transaction::PreviousValue::ExistingMustMatch(gix::refs::Target::Object(reviewed)),
            "advance the hidden base",
        )?;

        let prepared = prepare_test(
            &repo,
            base,
            updated,
            &[
                Commit {
                    id: started.commit,
                    parents: vec![base],
                    info: "review".into(),
                },
                Commit {
                    id: reviewed,
                    parents: vec![base],
                    info: "reviewed".into(),
                },
            ],
            Some(started.commit),
        )?;
        let document = String::from_utf8(prepared.document.clone())?;
        assert_eq!(
            document.lines().filter(|line| *line == "(main)").count(),
            1,
            "a mutable ref at a shared fork target is emitted once"
        );
        let plan = parse_plan(&repo, &prepared.document)?;
        assert!(
            plan.expected_refs.iter().any(|reference| {
                reference.name == started.reference && !reference.editable && reference.target == reviewed
            }),
            "the active review remains part of the rebase transaction"
        );
        Ok(())
    }

    #[test]
    fn update_todo_roots_the_stack_at_the_hidden_tip_and_labels_only_that_heading() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let (base, middle, tip, commits) = commits(&repo)?;
        let mut commit = repo.find_commit(base)?.decode()?.into_owned()?;
        commit.parents = [base].into_iter().collect();
        commit.message = "updated * _ [hidden] <base> `raw` \\ base\n\n<!-- agent -->".into();
        let onto = repo.write_object(&commit)?.detach();
        repo.notes()?.replace("refs/notes/commits", onto, "anchor note")?;

        let prepared = prepare_test(&repo, base, onto, &commits, Some(tip))?;
        assert!(
            prepared.apply_unchanged,
            "moving the base makes an unchanged editor document actionable"
        );
        assert!(
            prepared.document.starts_with(unchanged_notice(true, false).as_bytes()),
            "the first line explains that the unchanged todo updates its base"
        );
        let document = String::from_utf8(prepared.document.clone())?;
        assert!(
            document.contains(&format!(
                "# Rebase from `{}` onto `{}`",
                crate::change_id::display_short(&repo, base)?,
                crate::change_id::display_short(&repo, onto)?
            )),
            "the update target is explicit in the document title"
        );
        assert!(
            document.contains(&format!(
                "fork {} (updated-base) [A] [N] updated * _ [hidden] <base> `raw` \\ base",
                crate::change_id::display_short(&repo, onto)?
            )),
            "the unfamiliar fork target carries its raw UI title"
        );
        assert_eq!(
            document.matches("updated * _ [hidden] <base> `raw` \\ base").count(),
            1,
            "only the new update target is labelled"
        );

        let plan = parse_plan(&repo, document.as_bytes())?;
        assert_eq!(plan.base, onto);
        assert_eq!(plan.steps[0].parent, rebase::PlanParent::Existing(onto));
        let graph = super::super::loaded_graph(&repo)?;
        let outcome = rebase::perform_plan(&repo, &graph, plan)?.complete()?;
        let rewritten_middle = outcome.map(middle).expect("the middle commit is retained");
        assert_eq!(
            repo.find_commit(rewritten_middle)?
                .parent_ids()
                .next()
                .map(gix::Id::detach),
            Some(onto),
            "saving the unchanged update todo rebases the stack onto the hidden tip"
        );
        Ok(())
    }

    #[test]
    fn update_todo_moves_a_branch_with_no_commits_to_the_new_base() -> gix_testtools::Result {
        let (fixture, repo) = repo()?;
        let (base, onto, _tip, _commits) = commits(&repo)?;
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["switch", "-q", "-c", "empty", &base.to_string()])
                .status()?
                .success(),
            "the fixture starts a branch without commits above its base"
        );

        let prepared = prepare_test(&repo, base, onto, &[], Some(base))?;
        assert!(
            prepared.apply_unchanged,
            "moving an empty stack's base makes the unchanged todo actionable"
        );
        let document = String::from_utf8(prepared.document)?;
        assert!(
            document.lines().any(|line| line == "(empty)"),
            "the branch at the old base moves with the generated todo"
        );
        let plan = parse_plan(&repo, document.as_bytes())?;
        assert!(plan.steps.is_empty(), "updating an empty stack creates no commits");
        assert_eq!(
            plan.expected_refs
                .iter()
                .find(|reference| reference.name == "refs/heads/empty")
                .and_then(|reference| reference.placement),
            Some(rebase::PlanParent::Existing(onto)),
            "the current branch is placed at the updated base"
        );

        let graph = super::super::loaded_graph(&repo)?;
        rebase::perform_plan(&repo, &graph, plan)?.complete()?;
        assert_eq!(
            repo.head_id()?.detach(),
            onto,
            "the checked-out branch advances to the updated base"
        );
        Ok(())
    }

    #[test]
    fn parses_reordering_forks_empty_commits_and_a_moved_checkout() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let (base, middle, tip, commits) = commits(&repo)?;
        let prepared = prepare_test(&repo, base, base, &commits, Some(tip))?;
        let edited = format!(
            "# Rebase\n\nfork {}\n`pick {}` ignored\n@empty a new checkpoint\n\nfork {}\n@pick {}\n",
            base.to_hex_with_len(7),
            tip.to_hex_with_len(7),
            tip.to_hex_with_len(7),
            middle.to_hex_with_len(7),
        );
        let edited = with_state(&prepared, &edited);
        let err = parse(&repo, &edited).expect_err("two checkout markers are invalid");
        assert!(format!("{err:#}").contains("more than one @"));

        let prepared = prepare_test(&repo, base, base, &commits, Some(tip))?;
        let edited = format!(
            "fork {}\npick {} ignored display metadata\nempty a new checkpoint\n\nfork {}\n@pick {}\n",
            base.to_hex_with_len(7),
            tip.to_hex_with_len(7),
            tip.to_hex_with_len(7),
            middle.to_hex_with_len(7),
        );
        let edited = with_state(&prepared, &edited);
        let plan = parse_plan(&repo, &edited)?;
        assert_eq!(plan.steps.len(), 3);
        assert_eq!(
            plan.checkout.as_ref().map(|checkout| checkout.target),
            Some(rebase::PlanParent::Step(2))
        );
        assert_eq!(plan.steps[2].parent, rebase::PlanParent::Step(0));
        assert!(matches!(&plan.steps[1].commit, rebase::PlanCommit::Empty(title) if title == b"a new checkpoint"));
        Ok(())
    }

    #[test]
    fn squash_above_a_command_folds_into_it_and_may_carry_checkout() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let (base, middle, tip, commits) = commits(&repo)?;
        let prepared = prepare_test(&repo, base, base, &commits, Some(tip))?;
        let edited = with_state(
            &prepared,
            &format!(
                "fork {}\npick {}\n`@squash {}` ignored display metadata\n\nfork {}\nempty side\n",
                base.to_hex_with_len(7),
                middle.to_hex_with_len(7),
                tip.to_hex_with_len(7),
                tip.to_hex_with_len(7),
            ),
        );
        let plan = parse_plan(&repo, &edited)?;
        assert_eq!(plan.steps.len(), 2, "squash does not produce another commit");
        assert_eq!(plan.steps[0].squash, [tip]);
        assert_eq!(
            plan.checkout.as_ref().map(|checkout| checkout.target),
            Some(rebase::PlanParent::Step(0)),
            "the squash marker selects the folded result"
        );
        assert_eq!(
            plan.steps[1].parent,
            rebase::PlanParent::Step(0),
            "the squashed ID resolves to the folded result as a fork target"
        );

        let invalid = with_state(
            &prepared,
            &format!(
                "fork {}\n@squash {}\npick {}\n",
                base.to_hex_with_len(7),
                tip.to_hex_with_len(7),
                middle.to_hex_with_len(7),
            ),
        );
        let err = parse(&repo, &invalid).expect_err("a fork cannot begin with squash");
        assert!(format!("{err:#}").contains("same fork"));
        Ok(())
    }

    #[test]
    fn continuation_todos_round_trip_the_resolved_index_and_remaining_squashes() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let (base, middle, tip, _) = commits(&repo)?;
        let branch: gix::refs::FullName = "refs/heads/continued".try_into()?;
        let prepared = prepare_continuation(
            &repo,
            &rebase::Plan {
                base,
                scope: vec![middle, tip],
                steps: vec![rebase::PlanStep {
                    parent: rebase::PlanParent::Existing(base),
                    commit: rebase::PlanCommit::Resolved(middle),
                    squash: vec![tip],
                }],
                checkout: Some(rebase::PlanCheckout {
                    target: rebase::PlanParent::Step(0),
                    reference: Some(branch.clone()),
                }),
                expected_refs: vec![rebase::ExpectedRef {
                    name: branch.clone(),
                    old: None,
                    target: middle,
                    new: Some(middle),
                    follows_tip: false,
                    editable: true,
                    placement: Some(rebase::PlanParent::Step(0)),
                }],
            },
            vec![middle],
            true,
        )?;
        assert!(
            prepared
                .document
                .starts_with(b"<!-- Rebase help follows. Saving unchanged continues the materialized rebase"),
            "the continuation explains that saving unchanged resumes it"
        );
        let document = String::from_utf8(prepared.document.clone())?;
        assert!(document.contains("(continued)"), "continuation refs are not marked");
        assert!(
            !document.contains("(@continued)"),
            "HEAD attachment stays in transaction state"
        );
        assert!(document.contains(&"0".repeat(40)), "the conflict uses the full null ID");
        assert!(
            document.contains(&format!("`squash {}`", crate::change_id::display_short(&repo, tip)?)),
            "unapplied squash sources remain editable"
        );
        let plan = parse_plan(&repo, &prepared.document)?;
        assert!(matches!(plan.steps[0].commit, rebase::PlanCommit::Resolved(id) if id == middle));
        assert_eq!(plan.steps[0].squash, [tip]);
        assert_eq!(
            plan.checkout.as_ref().and_then(|checkout| checkout.reference.as_ref()),
            Some(&branch),
            "the continuation retains its attached checkout"
        );
        assert!(
            plan.expected_refs.iter().any(|reference| reference.name == branch
                && reference.old.is_none()
                && reference.placement == Some(rebase::PlanParent::Step(0))),
            "a pending branch creation retains its nonexistence check and placement"
        );
        Ok(())
    }

    #[test]
    fn unchanged_checkout_marker_cannot_be_removed() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let (base, _middle, tip, commits) = commits(&repo)?;
        let prepared = prepare_test(&repo, base, base, &commits, Some(tip))?;
        let edited = format!("fork {}\n", base.to_hex_with_len(7));
        let edited = with_state(&prepared, &edited);
        let err = parse(&repo, &edited).expect_err("HEAD must be moved before its pick is dropped");
        assert!(format!("{err:#}").contains("checkout marker"));

        Ok(())
    }

    #[test]
    fn reference_lines_move_create_delete_and_detach_head() -> gix_testtools::Result {
        let (fixture, _) = repo()?;
        crate::test_repository::disable_autocrlf(fixture.path())?;
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["core.abbrev=7", "user.name=todo author", "user.email=todo@example.com"],
        )?;
        let (base, middle, tip, commits) = commits(&repo)?;
        let prepared = prepare_test(&repo, base, base, &commits, Some(tip))?;
        let generated = String::from_utf8(prepared.document.clone())?;
        assert!(
            generated.contains("(main, refs/patches/tip)"),
            "the generated todo shows the attached branch as an ordinary ref:\n{generated}"
        );
        assert!(!generated.contains("@main"), "existing HEAD attachment is implicit");
        assert_eq!(
            parse_plan(&repo, &prepared.document)?
                .checkout
                .and_then(|checkout| checkout.reference),
            Some("refs/heads/main".try_into()?),
            "an unchanged todo retains the original attachment"
        );

        let explicit = generated.replace("(main, refs/patches/tip)", "(@main, refs/patches/tip)");
        assert_eq!(
            parse_plan(&repo, explicit.as_bytes())?
                .checkout
                .and_then(|checkout| checkout.reference),
            Some("refs/heads/main".try_into()?),
            "adding @ explicitly enforces the same attachment"
        );

        let mismatched = with_state(
            &prepared,
            &format!(
                "fork {}\npick {}\n(@main)\n@pick {}\n",
                base.to_hex_with_len(7),
                middle.to_hex_with_len(7),
                tip.to_hex_with_len(7),
            ),
        );
        let err = parse(&repo, &mismatched).expect_err("an explicit attachment must agree with @pick");
        assert!(format!("{err:#}").contains("different results"));

        let edited = with_state(
            &prepared,
            &format!(
                "fork {}\npick {}\n(new-1, main)\n@pick {}\n",
                base.to_hex_with_len(7),
                middle.to_hex_with_len(7),
                tip.to_hex_with_len(7),
            ),
        );
        let plan = parse_plan(&repo, &edited)?;
        assert!(
            plan.checkout
                .as_ref()
                .is_some_and(|checkout| checkout.reference.is_none()),
            "moving the implicit HEAD branch away from @ requests a detached checkout"
        );
        let graph = super::super::loaded_graph(&repo)?;
        let outcome = rebase::perform_plan(&repo, &graph, plan)?.complete()?;
        super::super::time_travel::checkout_plan(repo.git_dir(), false, &outcome, &[], false)?;

        assert!(repo.head()?.referent_name().is_none(), "HEAD is detached");
        assert_eq!(
            repo.find_reference("refs/heads/new-1")?.id(),
            outcome.map(middle).context("the middle commit is retained")?,
            "the new branch line points at the following command below it"
        );
        assert!(
            repo.try_find_reference("refs/patches/middle")?.is_none()
                && repo.try_find_reference("refs/patches/tip")?.is_none(),
            "omitted generated refs are deleted"
        );
        assert_eq!(
            std::fs::read_to_string(fixture.path().join("tip"))?,
            "tip\n",
            "the detached checkout keeps the selected tree"
        );
        Ok(())
    }

    #[test]
    fn reference_lines_import_out_of_scope_refs_and_may_attach_head() -> gix_testtools::Result {
        let (fixture, repo) = repo()?;
        let (base, middle, tip, commits) = commits(&repo)?;
        for name in ["refs/heads/outside", "refs/patches/attach"] {
            repo.reference(
                name,
                base,
                gix::refs::transaction::PreviousValue::MustNotExist,
                "create out-of-scope todo ref",
            )?;
        }
        let prepared = prepare_test(&repo, base, base, &commits, Some(tip))?;
        let edited = with_state(
            &prepared,
            &format!(
                "fork {}\npick {}\n(outside)\n@pick {}\n(@refs/patches/attach)\n",
                base.to_hex_with_len(7),
                middle.to_hex_with_len(7),
                tip.to_hex_with_len(7),
            ),
        );
        let plan = parse_plan(&repo, &edited)?;
        let graph = super::super::loaded_graph(&repo)?;
        let outcome = rebase::perform_plan(&repo, &graph, plan)?.complete()?;
        let selected = outcome.selected.context("the todo retains its checkout")?;
        super::super::time_travel::checkout_plan(repo.git_dir(), false, &outcome, &[], false)?;

        assert_eq!(
            repo.find_reference("refs/heads/outside")?.id(),
            outcome.map(middle).context("the middle commit is retained")?,
            "an unmarked out-of-scope ref moves like a generated ref"
        );
        assert_eq!(
            repo.find_reference("refs/patches/attach")?.id(),
            selected,
            "the marked out-of-scope ref moves to the selected result"
        );
        assert_eq!(
            repo.head()?.referent_name().expect("HEAD is attached"),
            "refs/patches/attach",
            "HEAD attaches to an editable ref outside refs/heads"
        );
        assert_eq!(
            std::fs::read_to_string(fixture.path().join("tip"))?,
            "tip\n",
            "the selected worktree tree remains checked out"
        );
        assert_eq!(
            gix_testtools::repository::snapshot(fixture.path())?.index_tree,
            Some(repo.find_commit(selected)?.tree_id()?.detach()),
            "the index matches the attached commit"
        );
        Ok(())
    }

    #[test]
    fn rewritten_detached_head_is_not_pinned_before_checkout() -> gix_testtools::Result {
        let (fixture, repo) = repo()?;
        let (base, _middle, tip, commits) = commits(&repo)?;
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["checkout", "--quiet", "--detach", &tip.to_string()])
                .status()?
                .success(),
            "the fixture HEAD can be detached"
        );
        let prepared = prepare_test(&repo, base, base, &commits, Some(tip))?;
        let edited = with_state(
            &prepared,
            &format!(
                "fork {}\n@pick {}\n(main, refs/patches/tip)\n",
                base.to_hex_with_len(7),
                tip.to_hex_with_len(7),
            ),
        );
        let plan = parse_plan(&repo, &edited)?;
        let graph = super::super::loaded_graph(&repo)?;
        let outcome = rebase::perform_plan(&repo, &graph, plan)?.complete()?;
        let selected = outcome.selected.context("the rewritten todo retains @")?;
        assert_ne!(selected, tip, "dropping the middle commit rewrites the checked-out tip");

        super::super::time_travel::checkout_plan(repo.git_dir(), false, &outcome, &[], false)?;

        assert_eq!(repo.head_id()?, selected, "HEAD reaches the rewritten successor");
        assert!(
            crate::history::all_pins(&repo)?.iter().all(|pin| pin.id != tip),
            "the superseded detached HEAD is not retained through a pin"
        );
        Ok(())
    }

    #[test]
    fn deleting_the_current_branch_is_deferred_until_head_detaches() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let (base, _middle, tip, commits) = commits(&repo)?;
        let prepared = prepare_test(&repo, base, base, &commits, Some(tip))?;
        let edited = with_state(
            &prepared,
            &format!("fork {}\n@pick {}\n", base.to_hex_with_len(7), tip.to_hex_with_len(7)),
        );
        let plan = parse_plan(&repo, &edited)?;
        let graph = super::super::loaded_graph(&repo)?;
        let outcome = rebase::perform_plan(&repo, &graph, plan)?.complete()?;
        assert!(
            repo.try_find_reference("refs/heads/main")?.is_some(),
            "the checked-out branch remains until checkout succeeds"
        );
        super::super::time_travel::checkout_plan(repo.git_dir(), false, &outcome, &[], false)?;
        assert!(repo.head()?.referent_name().is_none(), "HEAD is detached");
        assert!(
            repo.try_find_reference("refs/heads/main")?.is_none(),
            "the departed current branch is deleted"
        );
        Ok(())
    }

    #[test]
    fn todos_reject_an_unborn_head() -> gix_testtools::Result {
        let (fixture, repo) = repo()?;
        let (base, _middle, _tip, commits) = commits(&repo)?;
        let prepared = prepare_test(&repo, base, base, &commits, None)?;
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["symbolic-ref", "HEAD", "refs/heads/unborn"])
                .status()?
                .success(),
            "HEAD becomes unborn while the todo is open"
        );
        assert!(
            prepare_test(&repo, base, base, &commits, None).is_err(),
            "generation rejects an unborn HEAD"
        );
        let err = parse(&repo, &prepared.document).expect_err("application rejects an unborn HEAD");
        assert!(format!("{err:#}").contains("born HEAD"));
        Ok(())
    }
}

use anyhow::{Context, Result};
use gix::{
    ObjectId,
    bstr::{BString, ByteSlice},
    config::File,
    hash::ChangeId,
    refs::FullName,
};

pub(crate) const REF_NAME: &str = "refs/worktree/tix/enrich";
pub(crate) const TREE_REF_NAME: &str = "refs/worktree/tix/enrich-tree";

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Enrichment {
    pub todo: bool,
    pub note: Option<BString>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct TreeEnrichment {
    pub checks_pass: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Headers {
    pub todo: bool,
    pub message: Option<BString>,
}

pub(crate) fn marker(todo: bool, note: bool, checks_pass: bool) -> &'static str {
    match (todo, note, checks_pass) {
        (true, true, true) => "🚧📝✔️",
        (true, true, false) => "🚧📝",
        (true, false, true) => "🚧✔️",
        (true, false, false) => "🚧",
        (false, true, true) => "📝✔️",
        (false, true, false) => "📝",
        (false, false, true) => "✔️",
        (false, false, false) => "",
    }
}

pub(crate) fn open(repo: &gix::Repository) -> Result<gix::note::Platform<'_>> {
    open_at(repo, REF_NAME)
}

pub(crate) fn open_tree(repo: &gix::Repository) -> Result<gix::note::Platform<'_>> {
    open_at(repo, TREE_REF_NAME)
}

fn open_at<'repo>(repo: &'repo gix::Repository, reference: &str) -> Result<gix::note::Platform<'repo>> {
    repo.notes()
        .context("could not open tix enrichments")?
        .with_refs([reference])
        .context("could not select the tix enrich reference")
}

pub(crate) fn load(notes: &mut gix::note::Platform, change_id: ChangeId) -> Result<Enrichment> {
    let config = load_config(notes, ObjectId::from(change_id))?;
    let Some(config) = config else {
        return Ok(Enrichment::default());
    };
    Ok(Enrichment {
        todo: config
            .boolean("commit.todo")
            .map_err(gix::Exn::into_error)
            .context("commit.todo is not a boolean")?
            .unwrap_or(false),
        note: config.string("commit.note").filter(|note| !note.is_empty()),
    })
}

pub(crate) fn load_tree(notes: &mut gix::note::Platform, tree_id: ObjectId) -> Result<TreeEnrichment> {
    let Some(config) = load_config(notes, tree_id)? else {
        return Ok(TreeEnrichment::default());
    };
    Ok(TreeEnrichment {
        checks_pass: config
            .boolean("tree.checks-pass")
            .map_err(gix::Exn::into_error)
            .context("tree.checks-pass is not a boolean")?
            .unwrap_or(false),
    })
}

fn load_config(notes: &mut gix::note::Platform, object_id: ObjectId) -> Result<Option<File>> {
    let found = notes.get(object_id).context("could not load the tix enrichment")?;
    found
        .first()
        .map(|note| {
            File::try_from(note.blob.data.as_bstr()).context("could not parse the tix enrichment as Git config")
        })
        .transpose()
}

pub(crate) fn tree_id(repo: &gix::Repository, commit_id: ObjectId) -> Result<ObjectId> {
    repo.find_commit(commit_id)
        .context("could not find the enriched commit")?
        .tree_id()
        .context("could not read the enriched commit tree")
        .map(gix::Id::detach)
}

pub(crate) fn toggle(repo: &gix::Repository, commit_id: ObjectId) -> Result<Enrichment> {
    update(repo, commit_id, |config| {
        let enabled = !config
            .boolean("commit.todo")
            .map_err(gix::Exn::into_error)
            .context("commit.todo is not a boolean")?
            .unwrap_or(false);
        set_todo(config, enabled)
    })
}

pub(crate) fn ensure_todo(repo: &gix::Repository, commit_id: ObjectId, enabled: bool) -> Result<Enrichment> {
    let current = load(&mut open(repo)?, crate::change_id::for_commit(repo, commit_id)?)?;
    if current.todo == enabled {
        return Ok(current);
    }
    update(repo, commit_id, |config| set_todo(config, enabled))
}

fn set_todo(config: &mut File, enabled: bool) -> Result<()> {
    config
        .section_mut_or_create_new("commit", None)
        .context("could not create the commit enrichment section")?
        .set("todo", if enabled { "true" } else { "false" })
        .context("could not update commit.todo")?;
    Ok(())
}

pub(crate) fn set_note(repo: &gix::Repository, commit_id: ObjectId, note: Option<&[u8]>) -> Result<Enrichment> {
    update(repo, commit_id, |config| {
        let mut section = config
            .section_mut_or_create_new("commit", None)
            .context("could not create the commit enrichment section")?;
        match note {
            Some(note) => {
                section.set("note", note).context("could not update commit.note")?;
            }
            None => {
                section.remove("note");
            }
        }
        Ok(())
    })
}

pub(crate) fn toggle_checks_pass(repo: &gix::Repository, commit_id: ObjectId) -> Result<TreeEnrichment> {
    let tree_id = tree_id(repo, commit_id)?;
    update_tree(repo, tree_id, |config| {
        let enabled = !config
            .boolean("tree.checks-pass")
            .map_err(gix::Exn::into_error)
            .context("tree.checks-pass is not a boolean")?
            .unwrap_or(false);
        set_checks_pass(config, enabled)
    })
}

pub(crate) fn ensure_checks_pass(repo: &gix::Repository, commit_id: ObjectId, enabled: bool) -> Result<TreeEnrichment> {
    let tree_id = tree_id(repo, commit_id)?;
    let current = load_tree(&mut open_tree(repo)?, tree_id)?;
    if current.checks_pass == enabled {
        return Ok(current);
    }
    update_tree(repo, tree_id, |config| set_checks_pass(config, enabled))
}

fn set_checks_pass(config: &mut File, enabled: bool) -> Result<()> {
    config
        .section_mut_or_create_new("tree", None)
        .context("could not create the tree enrichment section")?
        .set("checks-pass", if enabled { "true" } else { "false" })
        .context("could not update tree.checks-pass")?;
    Ok(())
}

pub(crate) fn apply_headers(
    repo: &gix::Repository,
    commit_id: ObjectId,
    headers: &Headers,
) -> Result<Option<Enrichment>> {
    let Some((object, data, desired)) = prepare_headers(repo, commit_id, headers)? else {
        return Ok(None);
    };
    let reference: FullName = REF_NAME.try_into().expect("the tix enrich reference is valid");
    open(repo)?
        .replace_at_ref(reference.as_ref(), object, data)
        .context("could not write the tix enrichment")?;
    Ok(Some(desired))
}

pub(crate) fn prepare_headers(
    repo: &gix::Repository,
    commit_id: ObjectId,
    headers: &Headers,
) -> Result<Option<(ObjectId, BString, Enrichment)>> {
    let change_id = crate::change_id::for_commit(repo, commit_id)?;
    let mut notes = open(repo)?;
    let current = load(&mut notes, change_id)?;
    let note = match (
        headers.message.as_ref().map(|message| message.as_bstr()),
        current.note.as_ref().map(|note| note.as_bstr()),
    ) {
        (None, _) => None,
        (Some(title), Some(existing)) => {
            let parsed = gix::objs::commit::MessageRef::from_bytes(existing);
            if parsed.summary().as_ref() == title {
                Some(existing.to_owned())
            } else {
                let mut message = BString::from(title);
                if let Some(body) = parsed.body {
                    message.extend_from_slice(b"\n\n");
                    message.extend_from_slice(body);
                }
                Some(message)
            }
        }
        (Some(title), None) => Some(title.to_owned()),
    };
    let desired = Enrichment {
        todo: headers.todo,
        note,
    };
    if desired == current {
        return Ok(None);
    }
    let object = ObjectId::from(change_id);
    let mut config = load_config(&mut notes, object)?.unwrap_or_default();
    let mut section = config
        .section_mut_or_create_new("commit", None)
        .context("could not create the commit enrichment section")?;
    section
        .set("todo", if desired.todo { "true" } else { "false" })
        .context("could not update commit.todo")?;
    match desired.note.as_ref().map(|note| note.as_bstr()) {
        Some(note) => {
            section.set("note", note).context("could not update commit.note")?;
        }
        None => {
            section.remove("note");
        }
    }
    drop(section);
    Ok(Some((object, config.to_bstring(), desired)))
}

fn update(
    repo: &gix::Repository,
    commit_id: ObjectId,
    edit: impl FnOnce(&mut File) -> Result<()>,
) -> Result<Enrichment> {
    let change_id = crate::change_id::for_commit(repo, commit_id)?;
    let mut notes = open(repo)?;
    let mut config = load_config(&mut notes, ObjectId::from(change_id))?.unwrap_or_default();
    edit(&mut config)?;
    let reference: FullName = REF_NAME.try_into().expect("the tix enrich reference is valid");
    notes
        .replace_at_ref(reference.as_ref(), ObjectId::from(change_id), config.to_bstring())
        .context("could not write the tix enrichment")?;
    load(&mut notes, change_id)
}

fn update_tree(
    repo: &gix::Repository,
    tree_id: ObjectId,
    edit: impl FnOnce(&mut File) -> Result<()>,
) -> Result<TreeEnrichment> {
    let mut notes = open_tree(repo)?;
    let mut config = load_config(&mut notes, tree_id)?.unwrap_or_default();
    edit(&mut config)?;
    let reference: FullName = TREE_REF_NAME
        .try_into()
        .expect("the tix tree enrich reference is valid");
    notes
        .replace_at_ref(reference.as_ref(), tree_id, config.to_bstring())
        .context("could not write the tix tree enrichment")?;
    load_tree(&mut notes, tree_id)
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    #[test]
    fn toggling_preserves_other_fields() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=todo author", "user.email=todo@example.com"],
        )?;
        let id = repo.head_id()?.detach();
        let change_id = crate::change_id::for_commit(&repo, id)?;
        let reference: FullName = REF_NAME.try_into()?;
        repo.notes()?.replace_at_ref(
            reference.as_ref(),
            ObjectId::from(change_id),
            b"[commit]\n\ttodo = true\n\towner = me\n",
        )?;

        assert!(!toggle(&repo, id)?.todo);
        let mut notes = open(&repo)?;
        let note = notes
            .get(ObjectId::from(change_id))?
            .into_iter()
            .next()
            .expect("the toggled note exists");
        let config = File::try_from(note.blob.data.as_bstr())?;
        assert_eq!(config.boolean("commit.todo")?, Some(false));
        assert_eq!(
            config.string("commit.owner").as_ref().map(|value| value.as_bstr()),
            Some(b"me".as_bstr())
        );
        Ok(())
    }

    #[test]
    fn todo_follows_a_rewrite_by_change_id() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=todo author", "user.email=todo@example.com"],
        )?;
        let original = repo.head_id()?.detach();
        assert!(toggle(&repo, original)?.todo);

        let mut commit = repo.find_commit(original)?.decode()?.into_owned()?;
        commit.message = "rewritten".into();
        crate::change_id::inherit(&repo, &mut commit, original)?;
        let rewritten = repo.write_object(&commit)?.detach();
        let change_id = crate::change_id::for_commit(&repo, rewritten)?;
        assert!(
            load(&mut open(&repo)?, change_id)?.todo,
            "the rewritten commit shares the todo"
        );
        Ok(())
    }

    #[test]
    fn checks_pass_follows_only_the_exact_tree_and_preserves_other_fields() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=checks author", "user.email=checks@example.com"],
        )?;
        let original = repo.head_id()?.detach();
        let tree = tree_id(&repo, original)?;
        let reference: FullName = TREE_REF_NAME.try_into()?;
        repo.notes()?.replace_at_ref(
            reference.as_ref(),
            tree,
            b"[tree]\n\tchecks-pass = false\n\towner = me\n",
        )?;

        assert!(toggle_checks_pass(&repo, original)?.checks_pass);
        let mut notes = open_tree(&repo)?;
        let note = notes.get(tree)?.into_iter().next().expect("the tree enrichment exists");
        let config = File::try_from(note.blob.data.as_bstr())?;
        assert_eq!(
            config.string("tree.owner").as_ref().map(|value| value.as_bstr()),
            Some(b"me".as_bstr())
        );

        let mut rewritten = repo.find_commit(original)?.decode()?.into_owned()?;
        rewritten.message = "same tree".into();
        let rewritten = repo.write_object(&rewritten)?.detach();
        assert!(
            load_tree(&mut open_tree(&repo)?, tree_id(&repo, rewritten)?)?.checks_pass,
            "a message-only rewrite retains the tree marker"
        );
        let changed_tree = tree_id(&repo, repo.rev_parse_single("HEAD~1")?.detach())?;
        assert!(
            !load_tree(&mut open_tree(&repo)?, changed_tree)?.checks_pass,
            "a different tree has no marker"
        );
        Ok(())
    }

    #[test]
    fn notes_and_todos_are_independent() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=todo author", "user.email=todo@example.com"],
        )?;
        let id = repo.head_id()?.detach();
        let message = b"Follow up\n\nExplain *why*.\n";

        let enrichment = set_note(&repo, id, Some(message))?;
        assert!(!enrichment.todo, "saving a note leaves todo disabled");
        assert_eq!(
            enrichment.note.as_ref().map(|note| note.as_bstr()),
            Some(message.as_bstr())
        );

        let enrichment = toggle(&repo, id)?;
        assert!(enrichment.todo, "the ordinary todo action enables todo");
        assert_eq!(
            enrichment.note.as_ref().map(|note| note.as_bstr()),
            Some(message.as_bstr()),
            "toggling todo preserves its note"
        );

        let enrichment = set_note(&repo, id, None)?;
        assert!(enrichment.todo, "emptying the editor preserves todo");
        assert!(enrichment.note.is_none(), "emptying the editor deletes the note");
        Ok(())
    }

    #[test]
    fn commit_headers_edit_only_the_message_title() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=header author", "user.email=header@example.com"],
        )?;
        let id = repo.head_id()?.detach();
        set_note(&repo, id, Some(b"Old title\n\nbody stays byte-for-byte\n"))?;

        let unchanged = apply_headers(
            &repo,
            id,
            &Headers {
                todo: false,
                message: Some("Old title".into()),
            },
        )?;
        assert!(unchanged.is_none(), "an unchanged title preserves the complete message");
        let changed = apply_headers(
            &repo,
            id,
            &Headers {
                todo: true,
                message: Some("New title".into()),
            },
        )?
        .expect("the title and todo changed");
        assert!(changed.todo);
        assert_eq!(
            changed.note.as_ref().map(|note| note.as_bstr()),
            Some(b"New title\n\nbody stays byte-for-byte\n".as_bstr())
        );

        let removed = apply_headers(&repo, id, &Headers::default())?.expect("removing the title changes the message");
        assert!(!removed.todo);
        assert!(removed.note.is_none(), "removing the title removes its body as well");
        Ok(())
    }

    #[test]
    fn malformed_enrichments_are_not_overwritten() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=todo author", "user.email=todo@example.com"],
        )?;
        let id = repo.head_id()?.detach();
        let change_id = crate::change_id::for_commit(&repo, id)?;
        let reference: FullName = REF_NAME.try_into()?;
        repo.notes()?
            .replace_at_ref(reference.as_ref(), ObjectId::from(change_id), b"[commit")?;

        assert!(
            load(&mut open(&repo)?, change_id).is_err(),
            "display can diagnose malformed enrichments"
        );
        assert!(
            toggle(&repo, id).is_err(),
            "mutation does not replace malformed enrichments"
        );
        let tree = tree_id(&repo, id)?;
        let reference: FullName = TREE_REF_NAME.try_into()?;
        repo.notes()?.replace_at_ref(reference.as_ref(), tree, b"[tree")?;
        assert!(
            toggle_checks_pass(&repo, id).is_err(),
            "tree mutation does not replace malformed enrichments"
        );
        Ok(())
    }

    #[test]
    fn enrichments_are_private_to_each_worktree() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let linked_path = fixture.path().join("linked");
        let status = Command::new("git")
            .current_dir(fixture.path())
            .args(["worktree", "add", "-q", "--detach"])
            .arg(&linked_path)
            .arg("HEAD")
            .status()?;
        assert!(status.success(), "git creates the linked worktree");
        let config = ["user.name=todo author", "user.email=todo@example.com"];
        let main = crate::test_repository::open_with(fixture.path(), config)?;
        let linked = crate::test_repository::open_with(&linked_path, config)?;
        let id = main.head_id()?.detach();
        let change_id = crate::change_id::for_commit(&main, id)?;

        assert!(toggle(&main, id)?.todo);
        assert!(
            !load(&mut open(&linked)?, change_id)?.todo,
            "main enrichments do not leak to linked worktrees"
        );
        assert!(toggle(&linked, id)?.todo);
        assert!(!toggle(&main, id)?.todo);
        assert!(
            load(&mut open(&linked)?, change_id)?.todo,
            "linked enrichments survive main-worktree changes"
        );
        let tree = tree_id(&main, id)?;
        assert!(toggle_checks_pass(&main, id)?.checks_pass);
        assert!(
            !load_tree(&mut open_tree(&linked)?, tree)?.checks_pass,
            "tree enrichments are also worktree-local"
        );
        Ok(())
    }
}

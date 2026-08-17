use anyhow::{Context, Result};
use gix::{
    ObjectId,
    bstr::{BString, ByteSlice},
    config::File,
    hash::ChangeId,
    refs::FullName,
};

pub(crate) const REF_NAME: &str = "refs/worktree/tix/enrich";

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Enrichment {
    pub todo: bool,
    pub note: Option<BString>,
}

pub(crate) fn marker(todo: bool, note: bool) -> &'static str {
    match (todo, note) {
        (true, true) => "🚧📝",
        (true, false) => "🚧",
        (false, true) => "📝",
        (false, false) => "",
    }
}

pub(crate) fn open(repo: &gix::Repository) -> Result<gix::note::Platform<'_>> {
    repo.notes()
        .context("could not open tix enrichments")?
        .with_refs([REF_NAME])
        .context("could not select the tix enrich reference")
}

pub(crate) fn load(notes: &mut gix::note::Platform, change_id: ChangeId) -> Result<Enrichment> {
    let config = load_config(notes, change_id)?;
    let Some(config) = config else {
        return Ok(Enrichment::default());
    };
    Ok(Enrichment {
        todo: config
            .boolean("commit.todo")
            .context("commit.todo is not a boolean")?
            .unwrap_or(false),
        note: config.string("commit.note").filter(|note| !note.is_empty()),
    })
}

fn load_config(notes: &mut gix::note::Platform, change_id: ChangeId) -> Result<Option<File>> {
    let found = notes
        .get(ObjectId::from(change_id))
        .context("could not load the tix enrichment")?;
    found
        .first()
        .map(|note| {
            File::try_from(note.blob.data.as_bstr()).context("could not parse the tix enrichment as Git config")
        })
        .transpose()
}

pub(crate) fn toggle(repo: &gix::Repository, commit_id: ObjectId) -> Result<Enrichment> {
    update(repo, commit_id, |config| {
        let enabled = !config
            .boolean("commit.todo")
            .context("commit.todo is not a boolean")?
            .unwrap_or(false);
        config
            .section_mut_or_create_new("commit", None)
            .context("could not create the commit enrichment section")?
            .set("todo", if enabled { "true" } else { "false" })
            .context("could not update commit.todo")?;
        Ok(())
    })
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

fn update(
    repo: &gix::Repository,
    commit_id: ObjectId,
    edit: impl FnOnce(&mut File) -> Result<()>,
) -> Result<Enrichment> {
    let change_id = crate::change_id::for_commit(repo, commit_id)?;
    let mut notes = open(repo)?;
    let mut config = load_config(&mut notes, change_id)?.unwrap_or_default();
    edit(&mut config)?;
    let reference: FullName = REF_NAME.try_into().expect("the tix enrich reference is valid");
    notes
        .add_to_ref(reference.as_ref(), ObjectId::from(change_id), config.to_bstring())
        .context("could not write the tix enrichment")?;
    load(&mut notes, change_id)
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
        repo.notes()?.add_to_ref(
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
            .add_to_ref(reference.as_ref(), ObjectId::from(change_id), b"[commit")?;

        assert!(
            load(&mut open(&repo)?, change_id).is_err(),
            "display can diagnose malformed enrichments"
        );
        assert!(
            toggle(&repo, id).is_err(),
            "mutation does not replace malformed enrichments"
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
        Ok(())
    }
}

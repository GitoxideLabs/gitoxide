use gix::{
    ObjectId, Result,
    bstr::{BStr, BString, ByteSlice},
    config::File,
    error::{OptionExt, ResultExt, message},
    hash::ChangeId,
    refs::FullName,
};

pub(crate) const REF_NAME: &str = "refs/worktree/tix/enrich";
pub(crate) const TREE_REF_NAME: &str = "refs/worktree/tix/enrich-tree";
pub(crate) const PATCH_REF_NAME: &str = "refs/worktree/tix/enrich-patch";

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Enrichment {
    pub todo: bool,
    pub note: Option<BString>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct TreeEnrichment {
    /// One result per tree: `None` is unmarked, an empty message is a plain check mark.
    pub checks_pass: Option<BString>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct PatchEnrichment {
    pub refackiewed: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Headers {
    pub todo: bool,
    pub message: Option<BString>,
}

pub(crate) fn marker(todo: bool, note: bool, checks_pass: Option<&BStr>, refackiewed: bool) -> String {
    format!(
        "{}{}{}{}",
        if todo { "🚧" } else { "" },
        if note { "📝" } else { "" },
        checks_pass.map(checks_pass_marker).unwrap_or_default(),
        if refackiewed { "✨" } else { "" },
    )
}

pub(crate) fn checks_pass_marker(message: &BStr) -> &str {
    crate::emoji::leading(message).unwrap_or("✔️")
}

pub(crate) fn checks_pass_message(message: &BStr) -> &BStr {
    match crate::emoji::leading(message) {
        Some(emoji) => message[emoji.len()..].trim_start().as_bstr(),
        None => message,
    }
}

pub(crate) fn open(repo: &gix::Repository) -> Result<gix::note::Platform<'_>> {
    open_at(repo, REF_NAME)
}

pub(crate) fn open_tree(repo: &gix::Repository) -> Result<gix::note::Platform<'_>> {
    open_at(repo, TREE_REF_NAME)
}

pub(crate) fn open_patch(repo: &gix::Repository) -> Result<gix::note::Platform<'_>> {
    open_at(repo, PATCH_REF_NAME)
}

fn open_at<'repo>(repo: &'repo gix::Repository, reference: &str) -> Result<gix::note::Platform<'repo>> {
    repo.notes()
        .or_raise(|| message("could not open tix enrichments"))?
        .with_refs([reference])
        .or_raise(|| message("could not select the tix enrich reference"))
}

pub(crate) fn load(notes: &mut gix::note::Platform, change_id: ChangeId) -> Result<Enrichment> {
    let config = load_config(notes, ObjectId::from(change_id))?;
    let Some(config) = config else {
        return Ok(Enrichment::default());
    };
    Ok(Enrichment {
        todo: config
            .boolean("commit.todo")
            .or_raise(|| message("commit.todo is not a boolean"))?
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
            .or_raise(|| message("tree.checks-pass is not a boolean"))?
            .unwrap_or(false)
            .then(|| config.string("tree.checks-pass-message").unwrap_or_default()),
    })
}

pub(crate) fn load_patch(
    notes: &mut gix::note::Platform,
    change_id: ChangeId,
    patch_id: crate::patch_id::PatchId,
) -> Result<PatchEnrichment> {
    let Some(config) = load_config(notes, ObjectId::from(change_id))? else {
        return Ok(PatchEnrichment::default());
    };
    patch_enrichment(&config, patch_id)
}

pub(crate) fn load_patch_for_commit(
    repo: &gix::Repository,
    notes: &mut gix::note::Platform,
    commit_id: ObjectId,
) -> Result<PatchEnrichment> {
    let object = repo.find_commit(commit_id)?;
    match crate::patch_id::for_display(repo, &object.decode()?)? {
        Some(patch_id) => load_patch(notes, crate::change_id::for_commit(repo, commit_id)?, patch_id),
        None => Ok(PatchEnrichment::default()),
    }
}

fn patch_enrichment(config: &File, patch_id: crate::patch_id::PatchId) -> Result<PatchEnrichment> {
    let subsection = format!("v1:{patch_id}");
    Ok(PatchEnrichment {
        refackiewed: config
            .boolean_by("patch", Some(subsection.as_bytes().as_bstr()), "refackiewed")
            .or_raise(|| message("patch.refackiewed is not a boolean"))?
            .unwrap_or(false),
    })
}

fn load_config(notes: &mut gix::note::Platform, object_id: ObjectId) -> Result<Option<File>> {
    let found = notes
        .get(object_id)
        .or_raise(|| message("could not load the tix enrichment"))?;
    found
        .first()
        .map(|note| {
            File::try_from(note.blob.data.as_bstr())
                .or_raise(|| message("could not parse the tix enrichment as Git config"))
        })
        .transpose()
}

pub(crate) fn tree_id(repo: &gix::Repository, commit_id: ObjectId) -> Result<ObjectId> {
    Ok(repo.find_commit(commit_id)?.tree_id()?.detach())
}

pub(crate) fn toggle(repo: &gix::Repository, commit_id: ObjectId) -> Result<Enrichment> {
    update(repo, commit_id, |config| {
        let enabled = !config
            .boolean("commit.todo")
            .or_raise(|| message("commit.todo is not a boolean"))?
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
        .section_mut_or_create_new("commit", None)?
        .set("todo", if enabled { "true" } else { "false" })?;
    Ok(())
}

pub(crate) fn set_note(repo: &gix::Repository, commit_id: ObjectId, note: Option<&[u8]>) -> Result<Enrichment> {
    update(repo, commit_id, |config| {
        let mut section = config.section_mut_or_create_new("commit", None)?;
        match note {
            Some(note) => {
                section.set("note", note)?;
            }
            None => {
                section.remove("note");
            }
        }
        Ok(())
    })
}

pub(crate) fn toggle_checks_pass(repo: &gix::Repository, commit_id: ObjectId) -> Result<TreeEnrichment> {
    let current = load_tree(&mut open_tree(repo)?, tree_id(repo, commit_id)?)?;
    set_checks_pass(repo, commit_id, current.checks_pass.is_none().then_some(b"".as_slice()))
}

pub(crate) fn set_checks_pass(
    repo: &gix::Repository,
    commit_id: ObjectId,
    message: Option<&[u8]>,
) -> Result<TreeEnrichment> {
    let tree_id = tree_id(repo, commit_id)?;
    let current = load_tree(&mut open_tree(repo)?, tree_id)?;
    if message.is_some() && current.checks_pass.as_ref().map(|value| value.as_slice()) == message {
        return Ok(current);
    }
    update_tree(repo, tree_id, |config| {
        for key in ["tree.checks-pass", "tree.checks-pass-message"] {
            if config.string_implicit(key).is_some() {
                config.raw_values_mut(key)?.delete_all();
            }
        }
        if let Some(message) = message {
            let mut section = config.section_mut_or_create_new("tree", None)?;
            section.set("checks-pass", "true")?;
            if !message.is_empty() {
                section.set("checks-pass-message", message)?;
            }
        }
        Ok(())
    })
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
        .or_raise(|| message("could not write the tix enrichment"))?;
    Ok(Some(desired))
}

pub(crate) fn prepare_refackiewed(
    repo: &gix::Repository,
    commit_id: ObjectId,
    enabled: bool,
) -> Result<Option<(ObjectId, BString, PatchEnrichment)>> {
    let patch_id = crate::patch_id::for_commit(repo, commit_id)?
        .ok_or_raise(|| message("the commit needs a current patch ID before it can be refackiewed"))?;
    let change_id = crate::change_id::for_commit(repo, commit_id)?;
    let object = ObjectId::from(change_id);
    let mut config = load_config(&mut open_patch(repo)?, object)?.unwrap_or_default();
    let current = patch_enrichment(&config, patch_id)?;
    if current.refackiewed == enabled {
        return Ok(None);
    }
    let subsection = format!("v1:{patch_id}");
    config
        .section_mut_or_create_new("patch", Some(subsection.as_bytes().as_bstr()))
        .or_raise(|| message("could not create the patch enrichment section"))?
        .set("refackiewed", if enabled { "true" } else { "false" })
        .or_raise(|| message("could not update patch.refackiewed"))?;
    Ok(Some((
        object,
        config.to_bstring(),
        PatchEnrichment { refackiewed: enabled },
    )))
}

pub(crate) fn ensure_refackiewed(
    repo: &gix::Repository,
    commit_id: ObjectId,
    enabled: bool,
) -> Result<PatchEnrichment> {
    if let Some((object, data, _)) = prepare_refackiewed(repo, commit_id, enabled)? {
        let reference: FullName = PATCH_REF_NAME
            .try_into()
            .expect("the tix patch enrich reference is valid");
        open_patch(repo)?
            .replace_at_ref(reference.as_ref(), object, data)
            .or_raise(|| message("could not write the tix patch enrichment"))?;
    }
    Ok(PatchEnrichment { refackiewed: enabled })
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
    let mut section = config.section_mut_or_create_new("commit", None)?;
    section.set("todo", if desired.todo { "true" } else { "false" })?;
    match desired.note.as_ref().map(|note| note.as_bstr()) {
        Some(note) => {
            section.set("note", note)?;
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
        .or_raise(|| message("could not write the tix enrichment"))?;
    load(&mut notes, change_id)
}

fn update_tree(
    repo: &gix::Repository,
    tree_id: ObjectId,
    edit: impl FnOnce(&mut File) -> Result<()>,
) -> Result<TreeEnrichment> {
    let mut notes = open_tree(repo)?;
    let mut config = load_config(&mut notes, tree_id)?.unwrap_or_default();
    let before = config.to_bstring();
    edit(&mut config)?;
    let after = config.to_bstring();
    if before != after {
        let reference: FullName = TREE_REF_NAME
            .try_into()
            .expect("the tix tree enrich reference is valid");
        notes
            .replace_at_ref(reference.as_ref(), tree_id, after)
            .or_raise(|| message("could not write the tix tree enrichment"))?;
    }
    load_tree(&mut notes, tree_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_patch_id(repo: &gix::Repository, commit_id: ObjectId) -> Result<ObjectId> {
        let mut commit = repo.find_commit(commit_id)?.decode()?.into_owned()?;
        crate::change_id::inherit(repo, &mut commit, commit_id)?;
        crate::patch_id::refresh(repo, &mut commit)?;
        Ok(repo.write_object(&commit)?.detach())
    }

    #[test]
    fn refackiewed_versions_are_change_scoped_and_preserve_other_fields() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let commit_id = with_patch_id(&repo, repo.head_id()?.detach())?;
        let change_id = crate::change_id::for_commit(&repo, commit_id)?;
        let patch_id = crate::patch_id::for_commit(&repo, commit_id)?.expect("the final commit has a patch ID");
        let other_patch_id: crate::patch_id::PatchId = "g".repeat(patch_id.to_string().len()).parse()?;
        assert_ne!(
            patch_id, other_patch_id,
            "the fixture patch differs from the synthetic old version"
        );
        ensure_todo(&repo, commit_id, true)?;
        set_note(&repo, commit_id, Some(b"keep the commit note"))?;
        let reference: FullName = PATCH_REF_NAME.try_into()?;
        repo.notes()?.replace_at_ref(
            reference.as_ref(),
            ObjectId::from(change_id),
            format!(
                "[patch \"v1:{patch_id}\"]\n\trefackiewed = false\n\towner = me\n\
                 [patch \"v1:{other_patch_id}\"]\n\trefackiewed = true\n"
            ),
        )?;

        assert!(ensure_refackiewed(&repo, commit_id, true)?.refackiewed);
        let first = repo.find_reference(PATCH_REF_NAME)?.id().detach();
        assert!(ensure_refackiewed(&repo, commit_id, true)?.refackiewed);
        assert_eq!(
            repo.find_reference(PATCH_REF_NAME)?.id(),
            first,
            "setting an existing mark does not rewrite its notes commit"
        );
        assert!(load_patch(&mut open_patch(&repo)?, change_id, other_patch_id)?.refackiewed);
        let unrelated = crate::change_id::for_commit(&repo, repo.rev_parse_single("HEAD~1")?.detach())?;
        assert!(
            !load_patch(&mut open_patch(&repo)?, unrelated, patch_id)?.refackiewed,
            "an identical patch ID does not share approval across different Tix changes"
        );
        let config =
            load_config(&mut open_patch(&repo)?, ObjectId::from(change_id))?.expect("the patch enrichment was written");
        let subsection = format!("v1:{patch_id}");
        assert_eq!(
            config
                .string_by("patch", Some(subsection.as_bytes().as_bstr()), "owner")
                .as_ref()
                .map(|value| value.as_bstr()),
            Some(b"me".as_bstr()),
            "patch updates preserve unrelated keys"
        );
        assert!(!ensure_refackiewed(&repo, commit_id, false)?.refackiewed);
        assert!(
            load_patch(&mut open_patch(&repo)?, change_id, other_patch_id)?.refackiewed,
            "clearing one patch version preserves approval of older versions"
        );
        assert_eq!(
            load(&mut open(&repo)?, change_id)?,
            Enrichment {
                todo: true,
                note: Some("keep the commit note".into()),
            },
            "patch approvals do not change commit enrichments"
        );
        Ok(())
    }

    #[test]
    fn malformed_patch_enrichments_are_not_overwritten() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let commit_id = with_patch_id(&repo, repo.head_id()?.detach())?;
        let change_id = crate::change_id::for_commit(&repo, commit_id)?;
        let patch_id = crate::patch_id::for_commit(&repo, commit_id)?.expect("the final commit has a patch ID");
        let reference: FullName = PATCH_REF_NAME.try_into()?;
        for data in [
            "[patch".to_owned(),
            format!("[patch \"v1:{patch_id}\"]\nrefackiewed = invalid\n"),
        ] {
            repo.notes()?
                .replace_at_ref(reference.as_ref(), ObjectId::from(change_id), data)?;
            let before = repo.find_reference(PATCH_REF_NAME)?.id().detach();
            assert!(
                ensure_refackiewed(&repo, commit_id, true).is_err(),
                "invalid configuration or a malformed boolean blocks patch mutation"
            );
            assert_eq!(
                repo.find_reference(PATCH_REF_NAME)?.id(),
                before,
                "malformed data stays intact"
            );
        }
        Ok(())
    }

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

        assert!(toggle_checks_pass(&repo, original)?.checks_pass.is_some());
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
            load_tree(&mut open_tree(&repo)?, tree_id(&repo, rewritten)?)?
                .checks_pass
                .is_some(),
            "a message-only rewrite retains the tree marker"
        );
        let changed_tree = tree_id(&repo, repo.rev_parse_single("HEAD~1")?.detach())?;
        assert!(
            load_tree(&mut open_tree(&repo)?, changed_tree)?.checks_pass.is_none(),
            "a different tree has no marker"
        );
        Ok(())
    }

    #[test]
    fn checks_pass_replaces_one_result_and_clears_its_message() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let commit_id = repo.head_id()?.detach();
        let tree_id = tree_id(&repo, commit_id)?;
        let message = "⚡ Fast checks in 42s

Compile: 30s
Tests: 12s
";
        for message in [message, "🔬 Thorough checks", "false", ""] {
            assert_eq!(
                set_checks_pass(&repo, commit_id, Some(message.as_bytes()))?.checks_pass,
                Some(message.into()),
                "setting a message replaces the single result, including literal booleans and empty text"
            );
            let reference = repo.find_reference(TREE_REF_NAME)?.id().detach();
            set_checks_pass(&repo, commit_id, Some(message.as_bytes()))?;
            assert_eq!(
                repo.find_reference(TREE_REF_NAME)?.id(),
                reference,
                "repeating the same result leaves notes history untouched"
            );
        }
        set_checks_pass(&repo, commit_id, Some(message.as_bytes()))?;
        assert!(toggle_checks_pass(&repo, commit_id)?.checks_pass.is_none());
        let config = load_config(&mut open_tree(&repo)?, tree_id)?.expect("the tree note exists");
        assert!(
            config.string("tree.checks-pass-message").is_none(),
            "clearing removes the message too"
        );
        assert_eq!(
            toggle_checks_pass(&repo, commit_id)?.checks_pass,
            Some(BString::default()),
            "toggling back on does not resurrect an old result message"
        );
        assert_eq!(
            checks_pass_message(message.as_bytes().as_bstr()),
            message.as_bytes()["⚡ ".len()..].as_bstr(),
            "the gutter emoji is removed only from displayed text"
        );
        Ok(())
    }

    #[test]
    fn replacing_checks_pass_removes_duplicate_and_orphaned_values() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let commit_id = repo.head_id()?.detach();
        let tree_id = tree_id(&repo, commit_id)?;
        let reference: FullName = TREE_REF_NAME.try_into()?;
        for message in [Some(b"".as_slice()), None] {
            for enabled in ["true", "false"] {
                repo.notes()?.replace_at_ref(
                    reference.as_ref(),
                    tree_id,
                    format!(
                        "[tree]
    checks-pass = true
    checks-pass-message = stale
    owner = me
[tree]
    checks-pass = {enabled}
    checks-pass-message = current
"
                    ),
                )?;
                assert_eq!(
                    set_checks_pass(&repo, commit_id, message)?.checks_pass,
                    message.map(Into::into)
                );
                let config = load_config(&mut open_tree(&repo)?, tree_id)?.expect("the enrichment exists");
                assert!(
                    config.string("tree.checks-pass-message").is_none(),
                    "no older message can reappear"
                );
                assert_eq!(
                    config.string("tree.owner"),
                    Some("me".into()),
                    "unrelated metadata survives"
                );
                let before = repo.find_reference(TREE_REF_NAME)?.id().detach();
                set_checks_pass(&repo, commit_id, message)?;
                assert_eq!(
                    repo.find_reference(TREE_REF_NAME)?.id(),
                    before,
                    "repeating the operation is a no-op"
                );
            }
        }
        repo.notes()?.replace_at_ref(
            reference.as_ref(),
            tree_id,
            b"[tree]
    checks-pass
    checks-pass-message
    owner = me
",
        )?;
        assert!(set_checks_pass(&repo, commit_id, None)?.checks_pass.is_none());
        let config = load_config(&mut open_tree(&repo)?, tree_id)?.expect("the enrichment exists");
        assert!(
            config.string_implicit("tree.checks-pass").is_none(),
            "implicit flags are removed too"
        );
        assert!(
            config.string_implicit("tree.checks-pass-message").is_none(),
            "implicit messages are removed too"
        );
        assert_eq!(
            config.string("tree.owner"),
            Some("me".into()),
            "clearing preserves unrelated values"
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
        let status = gix_testtools::git_command(fixture.path())
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
        assert!(toggle_checks_pass(&main, id)?.checks_pass.is_some());
        assert!(
            load_tree(&mut open_tree(&linked)?, tree)?.checks_pass.is_none(),
            "tree enrichments are also worktree-local"
        );
        let commit_id = with_patch_id(&main, id)?;
        let patch_id = crate::patch_id::for_commit(&main, commit_id)?.expect("the final commit has a patch ID");
        assert!(ensure_refackiewed(&main, commit_id, true)?.refackiewed);
        assert!(
            !load_patch(&mut open_patch(&linked)?, change_id, patch_id)?.refackiewed,
            "patch approvals remain private even though their commit headers are shared"
        );
        Ok(())
    }
}

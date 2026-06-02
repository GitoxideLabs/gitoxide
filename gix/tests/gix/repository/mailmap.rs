use gix_date::parse::TimeBuf;

use crate::{Result, named_repo};

fn signature(name: &str, email: &str) -> gix_actor::Signature {
    gix_actor::Signature {
        name: name.into(),
        email: email.into(),
        time: gix_date::parse_header("42 +0800").expect("static input parses"),
    }
}

#[test]
fn empty_when_no_mailmap_present() -> Result {
    let repo = named_repo("make_basic_repo.sh")?;
    let snapshot = repo.open_mailmap();
    assert!(
        snapshot.entries().is_empty(),
        "a repo without any .mailmap or mailmap.* config yields an empty snapshot"
    );

    let mut into = gix_mailmap::Snapshot::default();
    repo.open_mailmap_into(&mut into)
        .expect("with no mailmap sources, no IO error is collected");
    assert!(
        into.entries().is_empty(),
        "open_mailmap_into mirrors open_mailmap when there are no sources"
    );
    Ok(())
}

#[test]
fn reads_mailmap_from_worktree_root() -> Result {
    let repo = named_repo("make_mailmap_repo.sh")?;
    let snapshot = repo.open_mailmap();
    assert_eq!(
        snapshot.entries().len(),
        1,
        "the single entry from the worktree .mailmap is loaded"
    );

    let mut buf = TimeBuf::default();
    let resolved = snapshot
        .try_resolve(signature("Old Name", "proper@example.com").to_ref(&mut buf))
        .expect("the entry rewrites the display name when the email matches");
    assert_eq!(
        resolved.name, "Proper Name",
        "the mapped name from .mailmap replaces the original"
    );
    assert_eq!(
        resolved.email, "proper@example.com",
        "the email is preserved (this entry only changes the name)"
    );
    Ok(())
}

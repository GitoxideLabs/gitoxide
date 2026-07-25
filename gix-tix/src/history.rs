use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    sync::atomic::{AtomicBool, Ordering},
};

use anyhow::{Context, Result};
use gix::{
    ObjectId,
    bstr::{BStr, BString, ByteSlice, ByteVec},
    objs::commit::ref_iter::Token,
};

use crate::app::{Attribution, AttributionKind, Author, Commit, LoadedCommits};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Decoration {
    pub name: BString,
    pub kind: DecorationKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DecorationKind {
    Head,
    Local,
    Remote,
    Tag,
    AnnotatedTag,
    Special,
}

pub(crate) type Decorations = HashMap<ObjectId, Vec<Decoration>>;
#[derive(Default)]
pub(crate) struct Authors {
    strings: HashSet<&'static [u8]>,
    authors: HashMap<(&'static BStr, &'static BStr), &'static Author>,
}
const COMMIT_BATCH_SIZE: usize = 1024;

#[derive(Debug)]
pub(crate) enum Event {
    Decorations(Decorations),
    Commits(LoadedCommits),
    Complete,
    Cancelled,
}

pub(crate) fn load(
    repo: &gix::Repository,
    revisions: &[OsString],
    hidden_revisions: &[OsString],
    authors: &mut Authors,
    cancelled: &AtomicBool,
    mut emit: impl FnMut(Event) -> bool,
) -> Result<()> {
    let Some(tips) = resolve_tips(repo, revisions)? else {
        emit(Event::Decorations(decorations(repo)?));
        emit(Event::Complete);
        return Ok(());
    };
    let hidden_tips = resolve_revisions(repo, hidden_revisions, "hidden ")?;

    if !emit(Event::Decorations(decorations(repo)?)) {
        return Ok(());
    }
    let walk = repo
        .rev_walk(tips)
        .with_hidden(hidden_tips)
        .sorting(gix::revision::walk::Sorting::ByCommitTime(Default::default()))
        .all()
        .context("could not start revision walk")?;
    let mut rows = Vec::with_capacity(COMMIT_BATCH_SIZE);
    let mut attributions = Vec::with_capacity(COMMIT_BATCH_SIZE);
    for info in walk {
        if cancelled.load(Ordering::Relaxed) {
            emit(Event::Cancelled);
            return Ok(());
        }
        let info = info.context("could not traverse revision history")?;
        let object = info.object().context("could not read commit")?;
        let mut committer_time = None;
        let mut author = None;
        let attribution_start = attributions.len();
        let mut title = None;
        for token in object.iter() {
            match token.context("could not decode commit")? {
                Token::Author { signature } => {
                    let signature = signature.trim();
                    author = Some(authors.intern_author(signature.name, signature.email));
                }
                Token::Committer { signature } => {
                    committer_time = Some(signature.time().context("could not decode committer time")?);
                }
                Token::Message(message) => {
                    let message = gix::objs::commit::MessageRef::from_bytes(message);
                    title = Some(message.summary().into_owned());
                    if let Some(body) = message.body() {
                        for trailer in body.trailers() {
                            let Some(kind) = attribution_kind(&trailer) else {
                                continue;
                            };
                            let mut value: &[u8] = trailer.value.as_ref();
                            let Ok(identity) = gix::actor::IdentityRef::from_bytes_consuming(&mut value) else {
                                continue;
                            };
                            if !value.trim().is_empty() {
                                continue;
                            }
                            let identity = identity.trim();
                            attributions.push(Attribution {
                                kind,
                                author: authors.intern_author(identity.name, identity.email),
                            });
                        }
                    }
                }
                _ => {}
            }
        }
        rows.push(Commit {
            id: info.id,
            parent_ids: info.parent_ids,
            lane: 0..0,
            committer_time: committer_time.context("commit has no committer time")?,
            author: author.context("commit has no author")?,
            attributions: attribution_start..attributions.len(),
            title: title.context("commit has no message")?,
        });
        if rows.len() == COMMIT_BATCH_SIZE
            && !emit(Event::Commits(LoadedCommits {
                rows: std::mem::replace(&mut rows, Vec::with_capacity(COMMIT_BATCH_SIZE)),
                attributions: std::mem::replace(&mut attributions, Vec::with_capacity(COMMIT_BATCH_SIZE)),
            }))
        {
            return Ok(());
        }
    }
    if !rows.is_empty() && !emit(Event::Commits(LoadedCommits { rows, attributions })) {
        return Ok(());
    }
    emit(Event::Complete);
    Ok(())
}

pub(crate) fn count_up_to(
    repo: &gix::Repository,
    revisions: &[OsString],
    hidden_revisions: &[OsString],
    limit: usize,
) -> Result<usize> {
    let Some(tips) = resolve_tips(repo, revisions)? else {
        return Ok(0);
    };
    let hidden_tips = resolve_revisions(repo, hidden_revisions, "hidden ")?;
    let walk = repo
        .rev_walk(tips)
        .with_hidden(hidden_tips)
        .sorting(gix::revision::walk::Sorting::ByCommitTime(Default::default()))
        .all()
        .context("could not start revision walk")?;
    let mut count = 0;
    for info in walk.take(limit) {
        info.context("could not traverse revision history")?;
        count += 1;
    }
    Ok(count)
}

fn resolve_tips(repo: &gix::Repository, revisions: &[OsString]) -> Result<Option<Vec<ObjectId>>> {
    if revisions.is_empty() {
        repo.head()
            .context("could not read HEAD")?
            .try_peel_to_id()
            .context("could not resolve HEAD")
            .map(|id| id.map(|id| vec![id.detach()]))
    } else {
        resolve_revisions(repo, revisions, "").map(Some)
    }
}

fn attribution_kind(trailer: &gix::objs::commit::message::body::TrailerRef<'_>) -> Option<AttributionKind> {
    if trailer.is_co_authored_by() {
        Some(AttributionKind::CoAuthor)
    } else if trailer.is_reviewed_by() {
        Some(AttributionKind::Reviewed)
    } else if trailer.is_acked_by() {
        Some(AttributionKind::Acked)
    } else if trailer.is_tested_by() {
        Some(AttributionKind::Tested)
    } else if trailer.is_signed_off_by() {
        Some(AttributionKind::SignedOff)
    } else {
        None
    }
}

fn resolve_revisions(repo: &gix::Repository, revisions: &[OsString], kind: &str) -> Result<Vec<ObjectId>> {
    revisions
        .iter()
        .map(|revision| {
            let revision = gix::path::os_str_into_bstr(revision)
                .with_context(|| format!("{kind}revision {} is not valid UTF-8", revision.to_string_lossy()))?;
            repo.rev_parse_single(revision)
                .with_context(|| format!("could not resolve {kind}revision {revision}"))?
                .object()
                .with_context(|| format!("could not read {kind}revision"))?
                .peel_to_kind(gix::object::Kind::Commit)
                .with_context(|| format!("{kind}revision does not resolve to a commit"))
                .map(|object| object.id)
        })
        .collect()
}

impl Authors {
    fn intern_author(&mut self, name: &[u8], email: &[u8]) -> &'static Author {
        let name = self.intern_string(name);
        let email = self.intern_string(email);
        self.authors.entry((name, email)).or_insert_with(|| {
            let author: &'static Author = Box::leak(Box::new(Author { name, email }));
            author
        })
    }

    fn intern_string(&mut self, value: &[u8]) -> &'static BStr {
        match self.strings.get(value) {
            Some(value) => value.as_bstr(),
            None => {
                let value: &'static [u8] = Box::leak(value.to_vec().into_boxed_slice());
                self.strings.insert(value);
                value.as_bstr()
            }
        }
    }
}

fn decorations(repo: &gix::Repository) -> Result<Decorations> {
    let mut out = Decorations::new();
    for reference in repo
        .references()
        .context("could not open references")?
        .all()
        .context("could not iterate references")?
    {
        let mut reference = reference.map_err(|err| anyhow::anyhow!("could not read reference: {err}"))?;
        let mut kind = decoration_kind(reference.name().as_bstr());
        if kind == DecorationKind::Tag {
            let annotated = match reference.try_id() {
                Some(id) => id.header().context("could not inspect tag")?.kind() == gix::objs::Kind::Tag,
                None => false,
            };
            if annotated {
                kind = DecorationKind::AnnotatedTag;
            }
        }
        let Ok(id) = reference.peel_to_id() else {
            continue;
        };
        let id = id.detach();
        let mut name = reference.name().shorten().to_owned();
        if matches!(kind, DecorationKind::Tag | DecorationKind::AnnotatedTag) {
            name.insert_str(0, "tag: ");
        }
        out.entry(id).or_default().push(Decoration { name, kind });
    }
    if let Some(id) = repo
        .head()
        .context("could not read HEAD")?
        .try_peel_to_id()
        .context("could not peel HEAD")?
    {
        out.entry(id.detach()).or_default().push(Decoration {
            name: "HEAD".into(),
            kind: DecorationKind::Head,
        });
    }
    Ok(out)
}

fn decoration_kind(name: &[u8]) -> DecorationKind {
    if name.starts_with(b"refs/heads/") {
        DecorationKind::Local
    } else if name.starts_with(b"refs/tags/") {
        DecorationKind::Tag
    } else if name.starts_with(b"refs/remotes/") {
        DecorationKind::Remote
    } else {
        DecorationKind::Special
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, process::Command};

    use super::*;
    use crate::app::AttributionKind;

    fn fixture() -> gix_testtools::Result<std::path::PathBuf> {
        gix_testtools::scripted_fixture_read_only("history.sh")
    }

    fn loaded(path: &std::path::Path, revisions: &[&str], hidden_revisions: &[&str]) -> Result<Vec<Event>> {
        let mut events = Vec::new();
        let mut authors = Authors::default();
        let repo = gix::open(path)?;
        load(
            &repo,
            &revisions.iter().map(OsString::from).collect::<Vec<_>>(),
            &hidden_revisions.iter().map(OsString::from).collect::<Vec<_>>(),
            &mut authors,
            &AtomicBool::new(false),
            |event| {
                events.push(event);
                true
            },
        )?;
        Ok(events)
    }

    #[test]
    fn walks_the_same_reachable_set_as_git_for_multiple_tips() -> gix_testtools::Result {
        let fixture = fixture()?;
        let events = loaded(&fixture, &["main", "topic"], &[])?;
        let actual: HashSet<_> = events
            .iter()
            .flat_map(|event| match event {
                Event::Commits(batch) => batch.rows.iter().map(|row| row.id.to_hex().to_string()).collect(),
                _ => Vec::new(),
            })
            .collect();
        let output = Command::new("git")
            .current_dir(&fixture)
            .args(["rev-list", "main", "topic", "--"])
            .output()?;
        assert!(
            output.status.success(),
            "git rev-list provides the reference result: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let expected = String::from_utf8(output.stdout)?.lines().map(str::to_owned).collect();
        assert_eq!(actual, expected, "all commits reachable from either tip are shown once");
        assert!(matches!(events.last(), Some(Event::Complete)), "the walk completes");
        let (topic, attributions) = events
            .iter()
            .filter_map(|event| match event {
                Event::Commits(batch) => batch
                    .rows
                    .iter()
                    .find(|row| row.title == "topic")
                    .map(|row| (row, &batch.attributions)),
                _ => None,
            })
            .next()
            .expect("the topic commit is reachable");
        assert_eq!(
            topic.author.name, "Codex",
            "history loading retains the raw name despite the configured mailmap"
        );
        assert_eq!(topic.author.email, "Codex@OpenAI.com", "the author email is retained");
        assert!(
            topic.author.is_bot(),
            "well-known bot email addresses identify bot authors"
        );
        assert_eq!(
            attributions[topic.attributions.clone()]
                .iter()
                .map(|attribution| { (attribution.kind, attribution.author.name, attribution.author.is_bot(),) })
                .collect::<Vec<_>>(),
            [
                (AttributionKind::CoAuthor, b"Human Coauthor".as_bstr(), false),
                (AttributionKind::CoAuthor, b"Claude".as_bstr(), true),
                (AttributionKind::Reviewed, b"Reviewer".as_bstr(), false),
                (AttributionKind::Acked, b"Acknowledger".as_bstr(), false),
                (AttributionKind::Tested, b"Tester".as_bstr(), false),
                (AttributionKind::SignedOff, b"Signer".as_bstr(), false),
            ],
            "known attribution trailers retain their order and malformed identities are omitted"
        );
        assert_eq!(
            topic.committer_time.format_or_unix(gix::date::time::format::SHORT),
            "2000-01-04",
            "the committer date is retained"
        );
        Ok(())
    }

    #[test]
    fn hides_tips_and_every_commit_reachable_from_them() -> gix_testtools::Result {
        let fixture = fixture()?;
        let events = loaded(&fixture, &["topic"], &["main"])?;
        let actual: HashSet<_> = events
            .iter()
            .flat_map(|event| match event {
                Event::Commits(batch) => batch.rows.iter().map(|row| row.id.to_hex().to_string()).collect(),
                _ => Vec::new(),
            })
            .collect();
        let output = Command::new("git")
            .current_dir(&fixture)
            .args(["rev-list", "topic", "--not", "main", "--"])
            .output()?;
        assert!(
            output.status.success(),
            "git rev-list provides the reference result: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let expected = String::from_utf8(output.stdout)?.lines().map(str::to_owned).collect();
        assert_eq!(actual, expected, "hidden tips use Git's exclusion semantics");
        let repo = gix::open(&fixture)?;
        let revisions = [OsString::from("topic")];
        let hidden = [OsString::from("main")];
        assert_eq!(
            count_up_to(&repo, &revisions, &hidden, 1)?,
            actual.len().min(1),
            "the screen-size probe stops at its limit"
        );
        assert_eq!(
            count_up_to(&repo, &revisions, &hidden, usize::MAX)?,
            actual.len(),
            "the screen-size probe uses the same hidden history"
        );
        assert!(
            matches!(events.last(), Some(Event::Complete)),
            "the filtered walk completes"
        );
        Ok(())
    }

    #[test]
    fn reports_decorations_and_honours_cancellation() -> gix_testtools::Result {
        let fixture = fixture()?;
        let events = loaded(&fixture, &["main"], &[])?;
        let Event::Decorations(decorations) = &events[0] else {
            panic!("decorations are sent first")
        };
        assert!(
            decorations
                .values()
                .flatten()
                .any(|decoration| { decoration.name == "tag: v1" && decoration.kind == DecorationKind::AnnotatedTag }),
            "annotated tags decorate their commit"
        );
        assert!(
            decorations
                .values()
                .flatten()
                .all(|decoration| decoration.name != "origin/HEAD"),
            "dangling symbolic references are omitted"
        );

        let mut cancelled = Vec::new();
        let mut authors = Authors::default();
        let repo = gix::open(&fixture)?;
        load(&repo, &[], &[], &mut authors, &AtomicBool::new(true), |event| {
            cancelled.push(event);
            true
        })?;
        assert!(
            matches!(cancelled.as_slice(), [Event::Decorations(_), Event::Cancelled]),
            "cancellation preserves decorations and stops before commits"
        );
        Ok(())
    }

    #[test]
    fn classifies_reference_kinds() {
        assert_eq!(decoration_kind(b"refs/heads/main"), DecorationKind::Local);
        assert_eq!(decoration_kind(b"refs/tags/v1"), DecorationKind::Tag);
        assert_eq!(decoration_kind(b"refs/remotes/origin/main"), DecorationKind::Remote);
        assert_eq!(decoration_kind(b"refs/patches/main/patch"), DecorationKind::Special);
        assert_eq!(decoration_kind(b"refs/stash"), DecorationKind::Special);
    }

    #[test]
    fn interns_raw_author_identities() {
        let mut authors = Authors::default();

        let first = authors.intern_author(b"author\xff", b"one@example.com");
        let second = authors.intern_author(b"author\xff", b"one@example.com");
        let other = authors.intern_author(b"author\xff", b"two@example.com");

        assert!(std::ptr::eq(first, second), "equal identities share one allocation");
        assert!(!std::ptr::eq(first, other), "different emails remain distinct");
        assert_eq!(authors.authors.len(), 2);
        assert_eq!(first.name, b"author\xff".as_bstr(), "Git names remain byte strings");
    }
}

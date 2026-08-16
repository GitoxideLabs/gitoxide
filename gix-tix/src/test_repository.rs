use std::path::Path;

const DEFAULT_OVERRIDES: &[&str] = &[
    "user.name=author",
    "user.email=author@example.com",
    "gitoxide.commit.authorDate=2001-01-01T00:00:00 +0000",
    "gitoxide.commit.committerDate=2001-01-01T00:00:00 +0000",
    "commit.gpgSign=false",
    "core.editor=:",
];

pub(crate) fn open(path: impl AsRef<Path>) -> Result<gix::Repository, gix::open::Error> {
    open_with(path, std::iter::empty::<String>())
}

pub(crate) fn open_with<I, S>(path: impl AsRef<Path>, overrides: I) -> Result<gix::Repository, gix::open::Error>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut config: Vec<String> = DEFAULT_OVERRIDES.iter().map(ToString::to_string).collect();
    config.extend(overrides.into_iter().map(Into::into));
    gix::open_opts(
        path.as_ref().to_owned(),
        gix::open::Options::isolated().config_overrides(config),
    )
}

#[test]
fn defaults_are_deterministic_and_case_specific_overrides_win() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_read_only("history.sh")?;
    let repo = open_with(fixture, ["user.name=reviewer"])?;
    let author = repo.author().expect("the default author is configured")?.to_owned()?;
    let committer = repo
        .committer()
        .expect("the default committer is configured")?
        .to_owned()?;
    assert_eq!(author.name, "reviewer", "a case-specific override wins");
    assert_eq!(author.email, "author@example.com");
    assert_eq!(author.time.seconds, 978_307_200);
    assert_eq!(committer.name, "reviewer");
    assert_eq!(committer.time.seconds, 978_307_200);
    assert!(repo.commit_signing_options_if_enabled()?.is_none());
    assert_eq!(repo.editor(), Some(":".into()));
    Ok(())
}

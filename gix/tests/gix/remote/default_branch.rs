use crate::{Result, remote};

fn default_branch(repo: &gix::Repository, remote_name: &str) -> Result<Option<String>> {
    Ok(repo
        .find_remote(remote_name)?
        .default_branch()?
        .map(|name| name.as_bstr().to_string()))
}

#[test]
fn maps_the_remote_head_to_the_branch_on_the_remote() -> Result {
    let repo = remote::repo("remote-default-branch");
    let baseline = std::fs::read_to_string(remote::repo_path("remote-default-branch").join("baseline.git"))?;
    let remote_head_target = baseline
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("ref: "))
        .and_then(|line| line.strip_suffix("\tHEAD"))
        .expect("`git ls-remote --symref` shows what the remote `HEAD` points to");

    assert_eq!(
        default_branch(&repo, "origin")?.as_deref(),
        Some(remote_head_target),
        "`git clone` records the branch that the remote `HEAD` points to"
    );
    assert_eq!(
        default_branch(&repo, "other-head")?.as_deref(),
        Some("refs/heads/a"),
        "`git remote set-head` changes the record without looking at the remote, whose `HEAD` still points to `main`"
    );
    assert_eq!(
        default_branch(&repo, "renamed")?.as_deref(),
        Some("refs/heads/main"),
        "the fetch refspec maps the remote-tracking branch back to the branch on the remote, even if their names differ"
    );
    assert_eq!(
        default_branch(&repo, "team/origin")?.as_deref(),
        Some("refs/heads/main"),
        "remote names may contain slashes, which also nest their remote-tracking branches"
    );
    assert_eq!(
        default_branch(&repo, "dangling")?.as_deref(),
        Some("refs/heads/main"),
        "a `HEAD` whose remote-tracking branch was deleted still maps by name, like `git symbolic-ref` still prints it"
    );
    Ok(())
}

#[test]
fn fails_if_more_than_one_remote_reference_maps_to_the_remote_head_target() -> Result {
    let repo = remote::repo("remote-default-branch");
    let err = repo
        .find_remote("ambiguous")?
        .default_branch()
        .expect_err("`refs/heads/main` and `refs/tags/main` both map to `refs/remotes/ambiguous/main`");
    let message = err.to_string();
    for ref_name in ["refs/heads/main", "refs/tags/main"] {
        assert!(message.contains(ref_name), "the error mentions `{ref_name}`: {message}");
    }
    Ok(())
}

#[test]
fn is_unknown_without_a_symbolic_remote_head_that_fetch_refspecs_map() -> Result {
    let repo = remote::repo("remote-default-branch");
    for (remote_name, reason) in [
        ("no-head", "there is no `refs/remotes/no-head/HEAD`"),
        ("detached", "`refs/remotes/detached/HEAD` isn't a symbolic reference"),
        (
            "unmapped",
            "no fetch refspec maps a remote reference to `refs/remotes/unmapped/main` anymore",
        ),
    ] {
        assert_eq!(default_branch(&repo, remote_name)?, None, "{reason}");
    }

    let anonymous = repo.remote_at("https://example.com/repo")?.with_refspecs(
        Some("+refs/heads/*:refs/remotes/origin/*"),
        gix::remote::Direction::Fetch,
    )?;
    assert_eq!(
        anonymous.default_branch()?,
        None,
        "without a name there is no `refs/remotes/<name>/HEAD`, even with the fetch refspecs of `origin`"
    );

    let repo = remote::repo("missing-urls");
    for remote_name in ["https://fallback.example/repo", "example.com:repo"] {
        assert_eq!(
            default_branch(&repo, remote_name)?,
            None,
            "`{remote_name}` can't be part of a reference name, so it can't have remote-tracking branches"
        );
    }
    Ok(())
}

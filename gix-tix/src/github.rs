use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use gix::{
    ObjectId, Result,
    bstr::{BString, ByteSlice},
    error::{ErrorExt, OptionExt, ResultExt, bail, message},
};
use serde::{Deserialize, Serialize};

use crate::{edit, history};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Repository {
    pub full_name: String,
    pub remote: BString,
    pub ssh: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct PullRequest {
    pub number: u64,
    pub title: String,
    #[serde(rename = "html_url")]
    pub url: String,
    pub draft: bool,
    pub head: Head,
    pub base: Head,
    pub maintainer_can_modify: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct Head {
    pub sha: String,
    #[serde(rename = "ref")]
    pub branch: String,
    pub repo: Option<HeadRepository>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct HeadRepository {
    pub full_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct Snapshot {
    version: u32,
    pub repository: String,
    pub fetched_at: u64,
    pub pull_requests: Vec<PullRequest>,
}

fn valid_repository(name: &str) -> bool {
    let parts: Vec<_> = name.split('/').collect();
    parts.len() == 2
        && parts.iter().all(|part| {
            !part.is_empty()
                && !matches!(*part, "." | "..")
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
        })
}

fn github_url(url: &gix::url::Url) -> Option<String> {
    if !matches!(url.scheme, gix::url::Scheme::Https | gix::url::Scheme::Ssh)
        || !url.host.as_ref()?.eq_ignore_ascii_case("github.com")
    {
        return None;
    }
    let path = url.path.to_str().ok()?.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    valid_repository(path).then(|| path.to_owned())
}

pub(crate) fn repository(repo: &gix::Repository) -> Result<Repository> {
    let preferred = repo
        .head_name()?
        .and_then(|branch| {
            repo.branch_remote_name(branch.shorten(), gix::remote::Direction::Fetch)
                .map(|name| name.as_bstr().to_owned())
        })
        .or_else(|| repo.remote_default_name(gix::remote::Direction::Fetch));
    let mut candidates = Vec::new();
    if let Some(name) = &preferred
        && let Ok(remote) = repo.find_fetch_remote(Some(name.as_bstr()))
        && let Some(url) = remote.url(gix::remote::Direction::Fetch)
        && let Some(full_name) = github_url(url)
    {
        return Ok(Repository {
            full_name,
            remote: name.clone(),
            ssh: url.scheme == gix::url::Scheme::Ssh,
        });
    }
    for name in repo.remote_names() {
        let remote = repo.find_remote(name.as_bstr())?;
        let Some(url) = remote.url(gix::remote::Direction::Fetch) else {
            continue;
        };
        let Some(full_name) = github_url(url) else { continue };
        let candidate = Repository {
            full_name,
            remote: name.clone(),
            ssh: url.scheme == gix::url::Scheme::Ssh,
        };
        if !candidates
            .iter()
            .any(|other: &Repository| other.full_name.eq_ignore_ascii_case(&candidate.full_name))
        {
            candidates.push(candidate);
        }
    }
    if candidates.len() == 1 {
        return Ok(candidates.remove(0));
    }
    bail!(message(if candidates.is_empty() {
        "PRs require a github.com fetch remote"
    } else {
        "multiple GitHub repositories are configured; select one with the active branch's fetch remote"
    }));
}

impl PullRequest {
    pub(crate) fn commit_id(&self) -> Result<ObjectId> {
        ObjectId::from_hex(self.head.sha.as_bytes())
            .or_raise(|| message!("PR #{} has an invalid head commit ID", self.number))
    }

    pub(crate) fn label(&self) -> BString {
        format!(
            "PR #{}{} · {}",
            self.number,
            if self.draft { " draft" } else { "" },
            self.title
                .chars()
                .map(|ch| if ch.is_control() { ' ' } else { ch })
                .collect::<String>()
        )
        .into()
    }

    fn validate(&self, repository: &str) -> Result<()> {
        gix::error::ensure!(
            self.number != 0
                && self
                    .url
                    .eq_ignore_ascii_case(&format!("https://github.com/{repository}/pull/{}", self.number)),
            "invalid GitHub PR identity"
        );
        gix::error::ensure!(
            self.base
                .repo
                .as_ref()
                .is_some_and(|repo| repo.full_name.eq_ignore_ascii_case(repository)),
            "PR base belongs to another repository"
        );
        for head in [&self.head, &self.base] {
            ObjectId::from_hex(head.sha.as_bytes()).or_raise(|| message("invalid PR commit ID"))?;
            gix::refs::FullName::try_from(format!("refs/heads/{}", head.branch))
                .or_raise(|| message("invalid PR branch name"))?;
            if let Some(repo) = &head.repo {
                gix::error::ensure!(valid_repository(&repo.full_name), "invalid PR repository name");
            }
        }
        Ok(())
    }
}

impl Snapshot {
    fn from_response(repository: String, bytes: &[u8]) -> Result<Self> {
        let mut pull_requests = Vec::new();
        let mut found_page = false;
        for page in serde_json::Deserializer::from_slice(bytes).into_iter::<Vec<PullRequest>>() {
            pull_requests.extend(page.or_raise(|| message("invalid GitHub PR response"))?);
            found_page = true;
        }
        gix::error::ensure!(found_page, "GitHub returned no PR response");
        Self::new(repository, pull_requests)
    }
    pub(crate) fn new(repository: String, pull_requests: Vec<PullRequest>) -> Result<Self> {
        let snapshot = Self {
            version: 1,
            repository,
            fetched_at: SystemTime::now().duration_since(UNIX_EPOCH).or_error()?.as_secs(),
            pull_requests,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    fn validate(&self) -> Result<()> {
        gix::error::ensure!(
            self.version == 1 && valid_repository(&self.repository),
            "unsupported or invalid PR cache"
        );
        let mut seen = std::collections::HashSet::new();
        for pr in &self.pull_requests {
            pr.validate(&self.repository)?;
            gix::error::ensure!(seen.insert(pr.number), "duplicate PR in GitHub response");
        }
        Ok(())
    }

    fn path(repo: &gix::Repository, repository: &str) -> Result<PathBuf> {
        Ok(gix::path::realpath(repo.common_dir())
            .or_raise(|| message("could not resolve the shared PR cache directory"))?
            .join("tix/github")
            .join(repository)
            .join("pulls.json"))
    }

    pub(crate) fn load(repo: &gix::Repository, repository: &Repository) -> Result<Option<Self>> {
        let bytes = match std::fs::read(Self::path(repo, &repository.full_name)?) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => bail!(err.and_raise(message("could not read PR cache"))),
        };
        let mut snapshot: Self = serde_json::from_slice(&bytes).or_raise(|| message("could not parse PR cache"))?;
        snapshot.validate()?;
        gix::error::ensure!(
            snapshot.repository == repository.full_name,
            "PR cache belongs to another repository"
        );
        snapshot.pull_requests.retain(|pr| {
            pr.commit_id()
                .is_ok_and(|commit_id| repo.find_commit(commit_id).is_ok())
        });
        Ok(Some(snapshot))
    }

    fn save(&self, repo: &gix::Repository) -> Result<()> {
        let path = Self::path(repo, &self.repository)?;
        let mut lock = gix::lock::File::acquire_to_update_resource(
            &path,
            gix::lock::acquire::Fail::Immediately,
            Some(gix::path::realpath(repo.common_dir()).or_error()?),
            0,
        )
        .or_raise(|| message("could not lock PR cache"))?;
        serde_json::to_writer_pretty(&mut lock, self).or_raise(|| message("could not write PR cache"))?;
        lock.write_all(b"\n").or_error()?;
        lock.commit().or_raise(|| message("could not publish PR cache"))?;
        Ok(())
    }

    pub(crate) fn commit_ids(&self) -> impl Iterator<Item = ObjectId> + '_ {
        self.pull_requests.iter().filter_map(|pr| pr.commit_id().ok())
    }

    pub(crate) fn ref_name(&self, number: u64) -> String {
        format!("refs/tix/github/{}/pull/{number}/head", self.repository)
    }

    pub(crate) fn fetch(&self, repo: &gix::Repository, remote: &BString) -> Result<()> {
        if !self.pull_requests.is_empty() {
            let mut command = crate::git_command(repo.workdir().unwrap_or(repo.git_dir()));
            command
                .args([
                    "fetch",
                    "--atomic",
                    "--no-tags",
                    "--no-write-fetch-head",
                    "--no-recurse-submodules",
                    "--",
                ])
                .arg(gix::path::from_bstr(remote)?.as_ref());
            for pr in &self.pull_requests {
                command.arg(format!("+refs/pull/{}/head:{}", pr.number, self.ref_name(pr.number)));
            }
            run(&mut command, "fetch GitHub PR heads")?;
            for pr in &self.pull_requests {
                let mut reference = repo.find_reference(self.ref_name(pr.number).as_str())?;
                let commit_id = reference.peel_to_id()?;
                gix::error::ensure!(
                    commit_id == pr.commit_id()?,
                    "PR #{} changed during fetch; refresh PRs again",
                    pr.number
                );
                repo.find_commit(commit_id)?;
            }
        }
        // shortcut: closed PR refs retain fetched objects; prune unpinned cache refs if reclamation becomes necessary.
        self.save(repo)
    }
}

fn run(command: &mut Command, action: &str) -> Result<Vec<u8>> {
    let output = command
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GH_PROMPT_DISABLED", "1")
        .stdin(Stdio::null())
        .output()
        .or_raise(|| message!("could not launch {action}"))?;
    gix::error::ensure!(
        output.status.success(),
        "could not {action}: {}",
        output.stderr.to_str_lossy().trim()
    );
    Ok(output.stdout)
}

pub(crate) fn refresh(path: &std::path::Path, bare: bool, repository: Repository) -> Result<Snapshot> {
    const QUERY: &str = r"query($owner: String!, $name: String!, $endCursor: String) {
        repository(owner: $owner, name: $name) {
            pullRequests(first: 100, states: OPEN, after: $endCursor) {
                nodes { number title url isDraft maintainerCanModify headRefOid headRefName
                    headRepository { nameWithOwner } baseRefOid baseRefName baseRepository { nameWithOwner } }
                pageInfo { hasNextPage endCursor }
            }
        }
    }";
    const FILTER: &str = r".data.repository.pullRequests.nodes | map({
        number, title, html_url: .url, draft: .isDraft, maintainer_can_modify: .maintainerCanModify,
        head: {sha: .headRefOid, ref: .headRefName, repo: (.headRepository | if . == null then null else {full_name: .nameWithOwner} end)},
        base: {sha: .baseRefOid, ref: .baseRefName, repo: (.baseRepository | {full_name: .nameWithOwner})}
    })";
    let (owner, name) = repository
        .full_name
        .split_once('/')
        .ok_or_raise(|| message("invalid GitHub repository"))?;
    let mut command = Command::new("gh");
    command.args([
        "api",
        "--hostname",
        "github.com",
        "graphql",
        "--paginate",
        "--jq",
        FILTER,
        "-f",
        &format!("query={QUERY}"),
        "-f",
        &format!("owner={owner}"),
        "-f",
        &format!("name={name}"),
    ]);
    #[cfg(test)]
    gix_testtools::configure_git_environment(&mut command, path);
    let bytes = run(&mut command, "list GitHub PRs (install gh and run gh auth login)")?;
    let snapshot = Snapshot::from_response(repository.full_name, &bytes)?;
    let repo = crate::open_repository(path, bare, false)?;
    snapshot.fetch(&repo, &repository.remote)?;
    Ok(snapshot)
}

fn remote_matches(
    repo: &gix::Repository,
    name: &gix::bstr::BStr,
    direction: gix::remote::Direction,
    repository: &str,
) -> bool {
    let Ok(remote) = repo.find_fetch_remote(Some(name)) else {
        return false;
    };
    let matches = |url: &gix::url::Url| github_url(url).is_some_and(|name| name.eq_ignore_ascii_case(repository));
    let mut urls = remote.urls(direction);
    urls.next().is_some_and(matches) && urls.all(matches)
}

pub(crate) fn pin(
    repo: &gix::Repository,
    snapshot: &Snapshot,
    pr: &PullRequest,
    ssh: bool,
) -> Result<(history::Pin, Vec<edit::undo::RefChange>)> {
    let _guard = edit::mutation_lock(repo)?;
    pr.validate(&snapshot.repository)?;
    let expected_commit_id = pr.commit_id()?;
    let mut cached = repo.find_reference(snapshot.ref_name(pr.number).as_str())?;
    gix::error::ensure!(
        cached.peel_to_id()? == expected_commit_id,
        "PR head changed; refresh PRs before pinning"
    );
    let writable = pr.head.repo.as_ref().filter(|head| {
        pr.maintainer_can_modify
            || head.full_name.eq_ignore_ascii_case(&snapshot.repository)
            || repo.remote_names().iter().any(|name| {
                remote_matches(repo, name.as_bstr(), gix::remote::Direction::Fetch, &head.full_name)
                    && remote_matches(repo, name.as_bstr(), gix::remote::Direction::Push, &head.full_name)
            })
    });
    let target_repository = writable.map_or(snapshot.repository.as_str(), |head| head.full_name.as_str());
    let merge = if writable.is_some() {
        format!("refs/heads/{}", pr.head.branch)
    } else {
        format!("refs/pull/{}/head", pr.number)
    };
    let remote = repo
        .remote_names()
        .into_iter()
        .find(|name| {
            remote_matches(repo, name.as_bstr(), gix::remote::Direction::Fetch, target_repository)
                && remote_matches(repo, name.as_bstr(), gix::remote::Direction::Push, target_repository)
        })
        .map_or_else(
            || {
                if ssh {
                    format!("git@github.com:{target_repository}.git")
                } else {
                    format!("https://github.com/{target_repository}.git")
                }
            },
            |name| name.to_str_lossy().into_owned(),
        );
    let owner = pr
        .head
        .repo
        .as_ref()
        .map_or(snapshot.repository.as_str(), |head| head.full_name.as_str())
        .split('/')
        .next()
        .expect("repository names have two components");
    let candidates = [
        pr.head.branch.clone(),
        format!("{owner}/{}", pr.head.branch),
        format!("pr-{}", pr.number),
    ];
    let mut selected = None;
    for branch in candidates {
        let name = gix::refs::FullName::try_from(format!("refs/heads/{branch}")).or_error()?;
        let reference = repo.try_find_reference(name.as_ref())?;
        let config = repo.config_snapshot();
        let configured_remote = repo.branch_remote_name(branch.as_str(), gix::remote::Direction::Fetch);
        let matching = configured_remote
            .as_ref()
            .is_some_and(|name| remote_matches(repo, name.as_bstr(), gix::remote::Direction::Fetch, target_repository))
            && repo
                .branch_remote_name(branch.as_str(), gix::remote::Direction::Push)
                .is_some_and(|name| {
                    remote_matches(repo, name.as_bstr(), gix::remote::Direction::Push, target_repository)
                })
            && config
                .string(format!("branch.{branch}.merge").as_str())
                .is_some_and(|value| value.as_bstr() == merge.as_bytes());
        if let Some(mut reference) = reference {
            if matching {
                selected = Some((name, reference.peel_to_id()?.detach(), false));
                break;
            }
        } else if matching
            || configured_remote.is_none()
                && config.string(format!("branch.{branch}.merge").as_str()).is_none()
                && config.string(format!("branch.{branch}.pushRemote").as_str()).is_none()
        {
            selected = Some((name, expected_commit_id, true));
            break;
        }
    }
    let (name, commit_id, create) = selected.ok_or_raise(|| {
        message("all PR branch names are already used; rename a conflicting local branch before pinning")
    })?;
    let branch = name.shorten();
    if create {
        let mut config = repo.config_file_mut(repo.common_dir().join("config"))?;
        {
            let mut section = config.section_mut_or_create_new("branch", Some(branch))?;
            for (key, value) in [
                ("remote", remote.as_str()),
                ("pushRemote", remote.as_str()),
                ("merge", merge.as_str()),
                ("tixPullRequest", snapshot.ref_name(pr.number).as_str()),
            ] {
                section.set(key, value)?;
            }
        }
        let reference = repo.reference(
            name.clone(),
            commit_id,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "tix PR branch",
        )?;
        if let Err(err) = config.commit() {
            reference
                .delete()
                .or_raise(|| message!("could not roll back the PR branch after configuration failed: {err:#}"))?;
            return Err(err);
        }
    }
    let (pin, _, pin_changes) = edit::time_travel::create_or_reuse_pin_reporting(
        repo,
        gix::refs::Target::Symbolic(name),
        commit_id,
        "tix PR pin",
    )?;
    Ok((pin, pin_changes))
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn pull_request(
        repo: &gix::Repository,
        number: u64,
        branch: &str,
    ) -> gix::error::TestResult<PullRequest> {
        let sha = repo.rev_parse_single("topic")?.to_hex().to_string();
        Ok(PullRequest {
            number,
            title: "Improve refs\nwith a tab\tand Unicode 🦀".into(),
            url: format!("https://github.com/base/project/pull/{number}"),
            draft: true,
            head: Head {
                sha: sha.clone(),
                branch: branch.into(),
                repo: Some(HeadRepository {
                    full_name: "contributor/project".into(),
                }),
            },
            base: Head {
                sha,
                branch: "main".into(),
                repo: Some(HeadRepository {
                    full_name: "base/project".into(),
                }),
            },
            maintainer_can_modify: true,
        })
    }

    fn prepare(repo: &gix::Repository, pr: PullRequest) -> gix::error::TestResult<Snapshot> {
        let snapshot = Snapshot::new("base/project".into(), vec![pr])?;
        repo.reference(
            snapshot.ref_name(snapshot.pull_requests[0].number).as_str(),
            snapshot.pull_requests[0].commit_id()?,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "test cached PR head",
        )?;
        Ok(snapshot)
    }

    #[test]
    fn github_urls_and_metadata_are_validated_without_using_titles_as_identity() -> gix::error::TestResult {
        for url in [
            "git@github.com:base/project.git",
            "https://github.com/base/project/",
            "ssh://git@github.com/base/project",
        ] {
            assert_eq!(
                github_url(&gix::url::parse(url)?),
                Some("base/project".into()),
                "SSH and HTTPS identify the same repository"
            );
        }
        for url in [
            "https://gitlab.com/base/project",
            "https://github.com/base/../project",
            "https://github.com/base/project/extra",
        ] {
            assert_eq!(
                github_url(&gix::url::parse(url)?),
                None,
                "only canonical github.com repositories are accepted"
            );
        }
        let fixture = gix_testtools::scripted_fixture_read_only("history.sh")?;
        let repo = crate::test_repository::open(fixture)?;
        let pr = pull_request(&repo, 12, "topic")?;
        let snapshot = Snapshot::new("base/project".into(), vec![pr.clone()])?;
        assert_eq!(snapshot.ref_name(12), "refs/tix/github/base/project/pull/12/head");
        assert!(
            !pr.label().contains(&b'\n') && !pr.label().contains(&b'\t'),
            "metadata cannot inject terminal control characters"
        );
        assert!(
            Snapshot::new("base/project".into(), vec![pr.clone(), pr.clone()]).is_err(),
            "duplicate PR numbers are rejected"
        );
        let mut bad = pr.clone();
        bad.head.branch = "../outside".into();
        assert!(
            Snapshot::new("base/project".into(), vec![bad]).is_err(),
            "invalid branch names fail before fetching or configuring Git"
        );
        let mut bad = pr;
        bad.base.repo = Some(HeadRepository {
            full_name: "other/project".into(),
        });
        assert!(
            Snapshot::new("base/project".into(), vec![bad]).is_err(),
            "a response from another base repository cannot be cached"
        );
        Ok(())
    }

    #[test]
    fn paginated_response_includes_every_open_pull_request() -> gix::error::TestResult {
        let fixture = gix_testtools::scripted_fixture_read_only("history.sh")?;
        let repo = crate::test_repository::open(fixture)?;
        let mut requests = Vec::new();
        for number in 1..=101 {
            requests.push(pull_request(&repo, number, "incoming")?);
        }
        let mut bytes = serde_json::to_vec(&requests[..100])?;
        bytes.push(b'\n');
        bytes.extend(serde_json::to_vec(&requests[100..])?);
        let snapshot = Snapshot::from_response("base/project".into(), &bytes)?;
        assert_eq!(
            snapshot.pull_requests.len(),
            101,
            "the second API page is retained without the CLI's default 30-PR limit"
        );
        assert_eq!(snapshot.pull_requests[100].number, 101);
        assert!(
            Snapshot::from_response("base/project".into(), b"{}").is_err(),
            "an invalid API response is never treated as an empty successful refresh"
        );
        assert!(
            Snapshot::from_response("base/project".into(), b"").is_err(),
            "a missing response is an error"
        );
        assert!(
            Snapshot::from_response("base/project".into(), b"[]")?
                .pull_requests
                .is_empty(),
            "an explicit empty page closes the previous PR overlay"
        );
        Ok(())
    }

    #[test]
    fn repository_selection_prefers_the_active_fetch_remote_and_ignores_mirrors() -> gix::error::TestResult {
        let fixture = gix_testtools::scripted_fixture_read_only("history.sh")?;
        let repo = crate::test_repository::open_with(
            fixture,
            [
                "remote.origin.url=https://github.com/base/project.git",
                "remote.mirror.url=https://codeberg.org/base/project",
                "remote.fork.url=git@github.com:contributor/project.git",
                "branch.main.remote=fork",
            ],
        )?;
        let selected = repository(&repo)?;
        assert_eq!(selected.full_name, "contributor/project");
        assert_eq!(selected.remote, "fork");
        assert!(selected.ssh);
        Ok(())
    }

    #[test]
    fn pin_creates_a_local_branch_with_fork_tracking_and_reuses_it_without_checkout() -> gix::error::TestResult {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let head_commit_id = repo.head_id()?.detach();
        let snapshot = prepare(&repo, pull_request(&repo, 12, "incoming")?)?;
        let pr = &snapshot.pull_requests[0];
        let (pin, changes) = super::pin(&repo, &snapshot, pr, false)?;
        assert_eq!(
            pin.target,
            gix::refs::Target::Symbolic("refs/heads/incoming".try_into()?)
        );
        assert_eq!(pin.id, pr.commit_id()?);
        assert_eq!(
            changes.len(),
            1,
            "undo tracks the pin while keeping the useful configured branch"
        );
        let repo = crate::test_repository::open(fixture.path())?;
        assert_eq!(
            repo.head_id()?,
            head_commit_id,
            "pinning leaves HEAD and the worktree alone"
        );
        let config = repo.config_snapshot();
        for key in ["remote", "pushRemote"] {
            assert_eq!(
                config
                    .string(format!("branch.incoming.{key}").as_str())
                    .expect("the PR branch is configured"),
                "https://github.com/contributor/project.git"
            );
        }
        assert_eq!(
            config
                .string("branch.incoming.merge")
                .expect("the upstream branch is recorded"),
            "refs/heads/incoming"
        );
        let (reused, changes) = super::pin(&repo, &snapshot, pr, false)?;
        assert_eq!(reused.name, pin.name, "the existing standard pin is reused");
        assert!(changes.is_empty());
        Ok(())
    }

    #[test]
    fn pin_preserves_colliding_branches_and_records_the_original_push_destination() -> gix::error::TestResult {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let original_commit_id = repo.head_id()?.detach();
        let snapshot = prepare(&repo, pull_request(&repo, 12, "main")?)?;
        let (pin, _) = super::pin(&repo, &snapshot, &snapshot.pull_requests[0], true)?;
        assert_eq!(
            pin.target,
            gix::refs::Target::Symbolic("refs/heads/contributor/main".try_into()?)
        );
        let repo = crate::test_repository::open(fixture.path())?;
        assert_eq!(
            repo.find_reference("refs/heads/main")?.id(),
            original_commit_id,
            "a colliding local branch is never reset"
        );
        let config = repo.config_snapshot();
        assert_eq!(
            config
                .string("branch.contributor/main.merge")
                .expect("the upstream keeps its original name"),
            "refs/heads/main"
        );
        assert_eq!(
            config
                .string("branch.contributor/main.pushRemote")
                .expect("the fork URL is recorded"),
            "git@github.com:contributor/project.git"
        );
        Ok(())
    }

    #[test]
    fn pin_does_not_reuse_a_fork_remote_that_pushes_to_another_repository() -> gix::error::TestResult {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repo = crate::test_repository::open_with(
            fixture.path(),
            [
                "remote.fork.url=https://github.com/contributor/project.git",
                "remote.fork.pushurl=https://github.com/other/project.git",
                "branch.topic.remote=fork",
                "branch.topic.pushRemote=fork",
                "branch.topic.merge=refs/heads/topic",
            ],
        )?;
        let original_commit_id = repo.rev_parse_single("topic")?.detach();
        let snapshot = prepare(&repo, pull_request(&repo, 12, "topic")?)?;
        let (pin, _) = super::pin(&repo, &snapshot, &snapshot.pull_requests[0], false)?;
        assert_eq!(
            pin.target,
            gix::refs::Target::Symbolic("refs/heads/contributor/topic".try_into()?),
            "a matching fetch URL cannot authorize reuse of a different push destination"
        );
        let repo = crate::test_repository::open(fixture.path())?;
        assert_eq!(repo.find_reference("refs/heads/topic")?.id(), original_commit_id);
        assert_eq!(
            repo.config_snapshot()
                .string("branch.contributor/topic.pushRemote")
                .expect("the fork push destination is configured"),
            "https://github.com/contributor/project.git",
            "a remote with an unrelated push URL is bypassed"
        );
        Ok(())
    }

    #[test]
    fn read_only_and_deleted_forks_pin_locally_without_claiming_push_access() -> gix::error::TestResult {
        for deleted in [false, true] {
            let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
            let repo = crate::test_repository::open(fixture.path())?;
            let mut pr = pull_request(&repo, 12, "main")?;
            pr.maintainer_can_modify = false;
            if deleted {
                pr.head.repo = None;
            }
            let snapshot = prepare(&repo, pr)?;
            let branch = if deleted { "base/main" } else { "contributor/main" };
            let (pin, _) = super::pin(&repo, &snapshot, &snapshot.pull_requests[0], false)?;
            assert_eq!(
                pin.target,
                gix::refs::Target::Symbolic(format!("refs/heads/{branch}").try_into()?),
                "collisions use the head owner when available, even for read-only forks"
            );
            let repo = crate::test_repository::open(fixture.path())?;
            assert_eq!(
                repo.config_snapshot()
                    .string(format!("branch.{branch}.merge").as_str())
                    .expect("read-only tracking is explicit"),
                "refs/pull/12/head"
            );
            assert!(
                crate::push_branch(
                    repo.git_dir(),
                    "https://github.com/base/project.git".into(),
                    branch.into(),
                    &[],
                    false
                )
                .is_err(),
                "read-only PR refs are rejected before contacting GitHub"
            );
        }
        Ok(())
    }

    #[test]
    fn stale_head_refuses_to_pin_and_a_config_lock_leaves_no_partial_branch() -> gix::error::TestResult {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let snapshot = prepare(&repo, pull_request(&repo, 12, "incoming")?)?;
        let mut stale = snapshot.pull_requests[0].clone();
        stale.head.sha = repo.head_id()?.to_hex().to_string();
        assert!(
            super::pin(&repo, &snapshot, &stale, false).is_err(),
            "a racing refresh must not pin stale metadata"
        );
        let _lock = gix::lock::Marker::acquire_to_hold_resource(
            repo.common_dir().join("config"),
            gix::lock::acquire::Fail::Immediately,
            None,
            0,
        )?;
        assert!(
            super::pin(&repo, &snapshot, &snapshot.pull_requests[0], false).is_err(),
            "configuration must be writable before branch publication"
        );
        assert!(
            repo.try_find_reference("refs/heads/incoming")?.is_none(),
            "a failed pin leaves no half-configured branch"
        );
        assert!(history::all_pins(&repo)?.is_empty());
        Ok(())
    }

    #[test]
    fn batch_fetch_and_cache_refresh_leave_normal_refs_and_pinned_branches_unchanged() -> gix::error::TestResult {
        let fixture = gix_testtools::scripted_fixture_writable("fetch.sh")?;
        let remote = crate::test_repository::open(fixture.path().join("remote"))?;
        let local = crate::test_repository::open(fixture.path().join("local"))?;
        let head_commit_id = remote.rev_parse_single("advanced")?.detach();
        let mut pr = PullRequest {
            number: 12,
            title: "Incoming change".into(),
            url: "https://github.com/base/project/pull/12".into(),
            draft: false,
            head: Head {
                sha: head_commit_id.to_hex().to_string(),
                branch: "incoming".into(),
                repo: Some(HeadRepository {
                    full_name: "contributor/project".into(),
                }),
            },
            base: Head {
                sha: head_commit_id.to_hex().to_string(),
                branch: "main".into(),
                repo: Some(HeadRepository {
                    full_name: "base/project".into(),
                }),
            },
            maintainer_can_modify: true,
        };
        remote.reference(
            "refs/pull/12/head",
            head_commit_id,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "test PR head",
        )?;
        remote.reference(
            "refs/pull/13/head",
            head_commit_id,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "test second PR",
        )?;
        let main_commit_id = local.head_id()?.detach();
        let mut requests = vec![pr.clone()];
        pr.number = 13;
        pr.url = "https://github.com/base/project/pull/13".into();
        requests.push(pr);
        let snapshot = Snapshot::new("base/project".into(), requests)?;
        snapshot.fetch(&local, &BString::from("origin"))?;
        let selected = Repository {
            full_name: "base/project".into(),
            remote: "origin".into(),
            ssh: false,
        };
        assert_eq!(
            Snapshot::load(&local, &selected)?.expect("the successful fetch persists metadata"),
            snapshot
        );
        assert_eq!(
            local.head_id()?,
            main_commit_id,
            "fetching PRs leaves ordinary history tips unchanged"
        );
        assert_eq!(
            local.find_reference(snapshot.ref_name(13).as_str())?.id(),
            head_commit_id,
            "all open PRs are fetched in one batch"
        );
        let mut racing = snapshot.clone();
        racing.pull_requests[0].head.sha = main_commit_id.to_hex().to_string();
        racing.pull_requests[0].title = "New metadata must not replace the last successful snapshot".into();
        assert!(
            racing.fetch(&local, &BString::from("origin")).is_err(),
            "a fetch/API race refuses to publish inconsistent metadata"
        );
        assert_eq!(
            Snapshot::load(&local, &selected)?.expect("the previous cache remains readable"),
            snapshot,
            "failed refreshes retain the last successful metadata"
        );
        assert!(
            history::ref_tree_revisions(&local, true)?
                .iter()
                .all(|name| !name.to_string_lossy().starts_with("refs/tix/github/")),
            "cached PR refs never become ordinary view tips"
        );
        let (pin, _) = super::pin(&local, &snapshot, &snapshot.pull_requests[0], false)?;
        Snapshot::new("base/project".into(), Vec::new())?.fetch(&local, &BString::from("origin"))?;
        assert!(
            Snapshot::load(&local, &selected)?
                .expect("an empty successful result is cached")
                .pull_requests
                .is_empty(),
            "closed PRs disappear after a successful refresh"
        );
        assert_eq!(
            local
                .find_reference(pin.target.try_name().expect("pins target local branches"))?
                .id(),
            head_commit_id,
            "closing a PR preserves its configured local branch"
        );
        assert_eq!(history::all_pins(&local)?.len(), 1);
        let linked = fixture.path().join("linked");
        assert!(
            gix_testtools::git_command(local.workdir().expect("the fixture has a worktree"))
                .args(["worktree", "add", "--detach"])
                .arg(&linked)
                .status()?
                .success(),
            "a disposable linked worktree exercises shared cache ownership"
        );
        let linked = crate::test_repository::open(linked)?;
        assert_eq!(
            Snapshot::path(&local, &selected.full_name)?,
            Snapshot::path(&linked, &selected.full_name)?,
            "PR metadata belongs to the common repository"
        );
        assert!(
            Snapshot::load(&linked, &selected)?
                .expect("the other worktree sees the same snapshot")
                .pull_requests
                .is_empty()
        );
        Ok(())
    }
}

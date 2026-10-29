use std::path::Path;

use gix::remote::Direction;
use gix_object::Exists;
use gix_testtools::TestResult;

fn repo(path: impl AsRef<Path>) -> gix::Result<gix::Repository> {
    gix::open_opts(path.as_ref(), gix::open::Options::isolated())
}

#[test]
fn local_ref_map_matches_git_without_git_in_path() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = gix_testtools::scripted_fixture_read_only("make_native_local_fetch_repo.sh")?.canonicalize()?;
    let local = repo(fixture.join("empty"))?;
    let source = fixture.join("source");
    let expected = gix_testtools::git_command(&fixture)
        .args(["ls-remote", "--symref"])
        .arg(&source)
        .output()?;
    assert!(expected.status.success());
    let expected = String::from_utf8(expected.stdout)?;
    let mut remote = local.remote_at(gix::path::into_bstr(source)?)?;
    remote.replace_refspecs(["+refs/heads/*:refs/remotes/origin/*"], Direction::Fetch)?;
    let guard = gix_testtools::Env::new().set("PATH", fixture.join("empty").to_string_lossy());
    let (map, _) = remote.ref_map(gix::progress::Discard, Default::default())?;
    let mut actual = Vec::new();
    for reference in &map.remote_refs {
        use gix::protocol::handshake::Ref;
        let (name, target_id, peeled_id) = reference.unpack();
        if let Ref::Symbolic { target, .. } = reference {
            actual.push(format!("ref: {target}\t{name}"));
        }
        if let Some(target_id) = target_id {
            actual.push(format!("{target_id}\t{name}"));
        }
        if let Some(peeled_id) = peeled_id {
            actual.push(format!("{peeled_id}\t{name}^{{}}"));
        }
    }
    let mut expected: Vec<_> = expected.lines().map(str::to_owned).collect();
    actual.sort();
    expected.sort();
    assert_eq!(actual, expected);
    assert!(map.mappings.iter().any(|mapping| {
        mapping
            .local
            .as_ref()
            .is_some_and(|name| name == "refs/remotes/origin/main")
    }));
    let (empty, _) = local
        .remote_at(gix::path::into_bstr(fixture.join("empty"))?)?
        .ref_map(gix::progress::Discard, Default::default())?;
    assert!(
        matches!(&empty.remote_refs[..], [gix::protocol::handshake::Ref::Unborn { full_ref_name, target }] if full_ref_name == "HEAD" && target == "refs/heads/unborn")
    );
    drop(guard);
    Ok(())
}

fn fetch(
    local: &gix::Repository,
    source: &Path,
    options: gix::remote::fetch::Options,
) -> gix::Result<gix::remote::fetch::Outcome> {
    let mut remote = local
        .remote_at(gix::path::into_bstr(source.to_owned())?)?
        .with_fetch_tags(gix::remote::fetch::Tags::None);
    remote.replace_refspecs(["+refs/heads/main:refs/remotes/origin/main"], Direction::Fetch)?;
    remote.fetch(
        options,
        gix::progress::Discard,
        &std::sync::atomic::AtomicBool::default(),
    )
}

#[test]
fn native_fetch_depth_and_unshallow_match_git_without_git_in_path() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = gix_testtools::scripted_fixture_read_only("make_native_local_fetch_repo.sh")?.canonicalize()?;
    let dir = gix_testtools::tempfile::tempdir()?;
    for name in ["native", "git"] {
        gix_testtools::git(dir.path(), &format!("init --bare {name}"))?;
    }
    let local = repo(dir.path().join("native"))?;
    let expected = dir.path().join("git");
    for depth in [Some(1), Some(2), None, Some(i32::MAX as u32)] {
        let mut command = gix_testtools::git_command(&expected);
        command.args(["fetch", "--no-tags", "--no-write-fetch-head"]);
        if let Some(depth) = depth {
            command.args(["--depth", &depth.to_string()]);
        }
        let output = command
            .arg(fixture.join("source"))
            .arg("+refs/heads/main:refs/remotes/origin/main")
            .output()?;
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        let path_guard = gix_testtools::Env::new().set("PATH", dir.path().to_string_lossy());
        let outcome = fetch(
            &local,
            &fixture.join("source"),
            gix::remote::fetch::Options {
                shallow: depth
                    .map(|depth| gix::remote::fetch::Shallow::DepthAtRemote(depth.try_into().expect("positive depth")))
                    .unwrap_or_default(),
                ..Default::default()
            },
        )?;
        assert_eq!(outcome.ref_map.mappings.len(), 1);
        drop(path_guard);
        assert_eq!(
            gix_testtools::git(local.git_dir(), "rev-list --count refs/remotes/origin/main")?,
            gix_testtools::git(&expected, "rev-list --count refs/remotes/origin/main")?
        );
        assert_eq!(
            std::fs::read(local.git_dir().join("shallow")).ok(),
            std::fs::read(expected.join("shallow")).ok()
        );
        gix_testtools::git(local.git_dir(), "fsck --strict --no-reflogs")?;
    }
    Ok(())
}

#[test]
fn native_fetch_dry_run_and_pruning_match_git() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_native_local_fetch_repo.sh")?.canonicalize()?;
    let dir = gix_testtools::tempfile::tempdir()?;
    gix_testtools::git(dir.path(), "init --bare")?;
    let local = repo(dir.path())?;
    let result = fetch(
        &local,
        &fixture.join("source"),
        gix::remote::fetch::Options {
            dry_run: true,
            ..Default::default()
        },
    )?;
    assert_eq!(result.ref_map.mappings.len(), 1);
    assert!(local.try_find_reference("refs/remotes/origin/main")?.is_none());
    assert!(
        !local
            .objects
            .exists(result.ref_map.mappings[0].remote.as_id().expect("born remote"))
    );
    fetch(&local, &fixture.join("source"), Default::default())?;
    let mut remote = local
        .remote_at(gix::path::into_bstr(fixture.join("empty"))?)?
        .with_fetch_tags(gix::remote::fetch::Tags::None);
    remote.replace_refspecs(["+refs/heads/main:refs/remotes/origin/main"], Direction::Fetch)?;
    let result = remote.fetch(
        gix::remote::fetch::Options {
            prune: true,
            missing_refs_allowed: true,
            ..Default::default()
        },
        gix::progress::Discard,
        &std::sync::atomic::AtomicBool::default(),
    )?;
    assert_eq!(result.pruned.len(), 1);
    assert!(local.try_find_reference("refs/remotes/origin/main")?.is_none());
    Ok(())
}

#[test]
fn native_fetch_tag_policies_match_git_at_shallow_boundaries() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_native_local_fetch_repo.sh")?.canonicalize()?;
    for (tags, option) in [
        (gix::remote::fetch::Tags::None, Some("--no-tags")),
        (gix::remote::fetch::Tags::Included, None),
        (gix::remote::fetch::Tags::All, Some("--tags")),
    ] {
        let dir = gix_testtools::tempfile::tempdir()?;
        for name in ["native", "git"] {
            gix_testtools::git(dir.path(), &format!("init --bare {name}"))?;
        }
        let local = repo(dir.path().join("native"))?;
        let expected = dir.path().join("git");
        let mut command = gix_testtools::git_command(&expected);
        command.args(["fetch", "--depth=1", "--no-write-fetch-head"]);
        if let Some(option) = option {
            command.arg(option);
        }
        let output = command
            .arg(fixture.join("source"))
            .arg("+refs/heads/main:refs/remotes/origin/main")
            .output()?;
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        let mut remote = local
            .remote_at(gix::path::into_bstr(fixture.join("source"))?)?
            .with_fetch_tags(tags);
        remote.replace_refspecs(["+refs/heads/main:refs/remotes/origin/main"], Direction::Fetch)?;
        remote.fetch(
            gix::remote::fetch::Options {
                shallow: gix::remote::fetch::Shallow::DepthAtRemote(1.try_into()?),
                ..Default::default()
            },
            gix::progress::Discard,
            &std::sync::atomic::AtomicBool::default(),
        )?;
        assert_eq!(
            gix_testtools::git(local.git_dir(), "show-ref")?,
            gix_testtools::git(&expected, "show-ref")?
        );
        let mut actual: Vec<_> = std::fs::read_to_string(local.git_dir().join("shallow"))?
            .lines()
            .map(str::to_owned)
            .collect();
        let mut expected: Vec<_> = std::fs::read_to_string(expected.join("shallow"))?
            .lines()
            .map(str::to_owned)
            .collect();
        actual.sort();
        expected.sort();
        assert_eq!(actual, expected);
        gix_testtools::git(local.git_dir(), "fsck --strict --no-reflogs")?;
    }
    Ok(())
}

#[test]
fn native_fetch_preserves_shallow_sources_and_original_objects() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_native_local_fetch_repo.sh")?.canonicalize()?;
    let dir = gix_testtools::tempfile::tempdir()?;
    for name in ["native", "git"] {
        gix_testtools::git(dir.path(), &format!("init --bare {name}"))?;
    }
    let local = repo(dir.path().join("native"))?;
    let expected = dir.path().join("git");
    let output = gix_testtools::git_command(&expected)
        .args(["fetch", "--update-shallow", "--no-tags", "--no-write-fetch-head"])
        .arg(fixture.join("shallow"))
        .arg("+refs/heads/main:refs/remotes/origin/main")
        .output()?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    fetch(&local, &fixture.join("shallow"), Default::default())?;
    assert_eq!(
        gix_testtools::git(local.git_dir(), "rev-list --count refs/remotes/origin/main")?,
        "2\n"
    );
    assert_eq!(
        gix_testtools::git(local.git_dir(), "log -1 --format=%s refs/remotes/origin/main")?,
        "commit 4\n"
    );
    assert_eq!(
        std::fs::read(local.git_dir().join("shallow"))?,
        std::fs::read(expected.join("shallow"))?
    );
    gix_testtools::git(local.git_dir(), "fsck --strict --no-reflogs")?;
    Ok(())
}

#[test]
fn relative_remote_paths_match_explicit_git_dir_from_another_working_directory() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = gix_testtools::scripted_fixture_read_only("make_native_local_fetch_repo.sh")?.canonicalize()?;
    let dir = gix_testtools::tempfile::tempdir()?;
    for name in ["native", "git"] {
        gix_testtools::git(dir.path(), &format!("init --bare {name}"))?;
        gix_testtools::git(dir.path().join(name), "remote add origin source")?;
    }
    let local = repo(dir.path().join("native"))?;
    let output = gix_testtools::git_command(&fixture)
        .arg("--git-dir")
        .arg(dir.path().join("git"))
        .args([
            "fetch",
            "--no-tags",
            "--no-write-fetch-head",
            "origin",
            "+refs/heads/main:refs/remotes/origin/main",
        ])
        .output()?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let _cwd = gix_testtools::set_current_dir(&fixture)?;
    let mut remote = local
        .find_remote("origin")?
        .with_fetch_tags(gix::remote::fetch::Tags::None);
    remote.replace_refspecs(["+refs/heads/main:refs/remotes/origin/main"], Direction::Fetch)?;
    remote.fetch(
        Default::default(),
        gix::progress::Discard,
        &std::sync::atomic::AtomicBool::default(),
    )?;
    let actual_id = local.find_reference("refs/remotes/origin/main")?.id().detach();
    let expected_id = repo(dir.path().join("git"))?
        .find_reference("refs/remotes/origin/main")?
        .id()
        .detach();
    assert_eq!(actual_id, expected_id);
    Ok(())
}

#[test]
fn remote_head_candidates_match_git_remote_show() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_remote_head_candidates.sh")?.canonicalize()?;
    let local = repo(fixture.join("local"))?;
    for case in ["symbolic", "ambiguous", "unique", "unborn"] {
        let mut remote = local
            .remote_at(gix::path::into_bstr(fixture.join(case))?)?
            .with_fetch_tags(gix::remote::fetch::Tags::None);
        remote.replace_refspecs(["HEAD", "+refs/heads/*:refs/remotes/origin/*"], Direction::Fetch)?;
        let (map, _) = remote.ref_map(gix::progress::Discard, Default::default())?;
        let mut actual: Vec<_> = map
            .remote_head_candidates()
            .map(|reference| reference.unpack().0.to_string())
            .collect();
        let output = std::fs::read_to_string(fixture.join(format!("{case}-remote-show")))?;
        let mut expected = Vec::new();
        let mut ambiguous = false;
        for line in output.lines() {
            if let Some(name) = line.strip_prefix("  HEAD branch: ") {
                if name != "(unknown)" {
                    expected.push(format!("refs/heads/{name}"));
                }
            } else if line.starts_with("  HEAD branch (remote HEAD is ambiguous,") {
                ambiguous = true;
            } else if ambiguous {
                if let Some(name) = line.strip_prefix("    ") {
                    expected.push(format!("refs/heads/{name}"));
                } else {
                    ambiguous = false;
                }
            }
        }
        actual.sort();
        expected.sort();
        assert_eq!(actual, expected, "HEAD candidates match Git for {case}");
    }
    Ok(())
}

#[test]
fn local_namespace_and_hidden_ref_advertisements_match_git() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_namespaced_local_fetch_repo.sh")?.canonicalize()?;
    let local = repo(fixture.join("empty"))?;
    for namespace in ["team/project", ""] {
        let expected = gix_testtools::git_command(&fixture)
            .env("GIT_NAMESPACE", namespace)
            .args(["ls-remote", "--symref"])
            .arg(fixture.join("source"))
            .output()?;
        assert!(
            expected.status.success(),
            "{}",
            String::from_utf8_lossy(&expected.stderr)
        );
        let remote = local
            .remote_at(gix::path::into_bstr(fixture.join("source"))?)?
            .with_command_environment([
                ("GIT_NAMESPACE".into(), "missing".into()),
                ("GIT_NAMESPACE".into(), namespace.into()),
            ]);
        let (map, _) = remote.ref_map(gix::progress::Discard, Default::default())?;
        let mut actual = Vec::new();
        for reference in &map.remote_refs {
            let (name, target_id, peeled_id) = reference.unpack();
            if let gix::protocol::handshake::Ref::Symbolic { target, .. } = reference {
                actual.push(format!("ref: {target}\t{name}"));
            }
            if let Some(target_id) = target_id {
                actual.push(format!("{target_id}\t{name}"));
            }
            if let Some(peeled_id) = peeled_id {
                actual.push(format!("{peeled_id}\t{name}^{{}}"));
            }
        }
        let expected = String::from_utf8(expected.stdout)?;
        let mut expected: Vec<_> = expected.lines().map(str::to_owned).collect();
        actual.sort();
        expected.sort();
        // Git can advertise the namespaced HEAD twice; compare the reference set.
        actual.dedup();
        expected.dedup();
        assert_eq!(
            actual, expected,
            "namespace {namespace:?} retains logical and physical hideRefs rules"
        );
    }
    Ok(())
}

#[test]
fn native_namespaced_fetch_matches_git_with_relative_url_and_empty_path() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = gix_testtools::scripted_fixture_read_only("make_namespaced_local_fetch_repo.sh")?.canonicalize()?;
    let dir = gix_testtools::tempfile::tempdir()?;
    for name in ["native", "git"] {
        gix_testtools::git(dir.path(), &format!("init --bare {name}"))?;
    }
    let output = gix_testtools::git_command(&fixture)
        .env("GIT_NAMESPACE", "team/project")
        .arg("--git-dir")
        .arg(dir.path().join("git"))
        .args([
            "fetch",
            "--no-tags",
            "--no-write-fetch-head",
            "source",
            "+refs/heads/*:refs/remotes/origin/*",
        ])
        .output()?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let local = repo(dir.path().join("native"))?;
    let mut remote = local
        .remote_at("source")?
        .with_fetch_tags(gix::remote::fetch::Tags::None)
        .with_command_environment([("GIT_NAMESPACE".into(), "team/project".into())]);
    remote.replace_refspecs(["+refs/heads/*:refs/remotes/origin/*"], Direction::Fetch)?;
    {
        let _cwd = gix_testtools::set_current_dir(&fixture)?;
        let _path = gix_testtools::Env::new().set("PATH", dir.path().to_string_lossy());
        remote.fetch(
            Default::default(),
            gix::progress::Discard,
            &std::sync::atomic::AtomicBool::default(),
        )?;
    }
    assert_eq!(
        gix_testtools::git(local.git_dir(), "show-ref")?,
        gix_testtools::git(dir.path().join("git"), "show-ref")?,
        "only visible namespaced references are fetched into the default local namespace"
    );
    gix_testtools::git(local.git_dir(), "fsck --strict --no-reflogs")?;
    Ok(())
}

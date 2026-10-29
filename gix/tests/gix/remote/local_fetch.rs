use std::path::Path;

use gix::remote::Direction;
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

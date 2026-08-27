use std::{
    ffi::OsStr,
    io::Write as _,
    path::{Path, PathBuf},
    process::Stdio,
};

use bstr::BString;
use gix_hash::{Kind, ObjectId};
use gix_reftable::{Limits, LogRecord, LogValue, RefRecord, RefValue, Table, Version, WriteOptions, Writer};

type TestResult<T = ()> = gix_testtools::Result<T>;

fn git_ok<I, S>(repo: Option<&Path>, args: I) -> TestResult<std::process::Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    gix_testtools::isolated_git_output_checked(repo, args)
}

fn create_migrated_repo(hash: &str, bulk_refs: usize) -> TestResult<tempfile::TempDir> {
    let temp = tempfile::tempdir()?;
    let object_format = format!("--object-format={hash}");
    git_ok(
        None,
        [
            OsStr::new("init"),
            OsStr::new("--quiet"),
            OsStr::new("--initial-branch=main"),
            OsStr::new(&object_format),
            temp.path().as_os_str(),
        ],
    )?;
    git_ok(
        Some(temp.path()),
        [
            "-c",
            "user.name=Reftable Test",
            "-c",
            "user.email=reftable@example.com",
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "initial",
        ],
    )?;
    git_ok(Some(temp.path()), ["branch", "topic"])?;
    git_ok(Some(temp.path()), ["branch", "deleted"])?;
    git_ok(
        Some(temp.path()),
        [
            "-c",
            "user.name=Reftable Test",
            "-c",
            "user.email=reftable@example.com",
            "tag",
            "-a",
            "v1",
            "-m",
            "annotated",
        ],
    )?;
    git_ok(
        Some(temp.path()),
        ["symbolic-ref", "refs/meta/current", "refs/heads/main"],
    )?;
    if bulk_refs != 0 {
        let commit = String::from_utf8(git_ok(Some(temp.path()), ["rev-parse", "HEAD"])?.stdout)?;
        let mut input = Vec::new();
        for idx in 0..bulk_refs {
            writeln!(input, "create refs/heads/bulk/{idx:05} {}", commit.trim())?;
        }
        let mut command = gix_testtools::isolated_git_command(Some(temp.path()));
        command
            .args(["update-ref", "--stdin"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn()?;
        child
            .stdin
            .take()
            .ok_or("git update-ref stdin is unavailable")?
            .write_all(&input)?;
        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(format!(
                "git update-ref --stdin failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
    }
    git_ok(Some(temp.path()), ["refs", "migrate", "--ref-format=reftable"])?;
    git_ok(Some(temp.path()), ["update-ref", "-d", "refs/heads/deleted"])?;
    let git_version = String::from_utf8(git_ok(None, ["--version"])?.stdout)?;
    let bulk_step = if bulk_refs == 0 {
        String::new()
    } else {
        format!("git -C <repo> rev-parse HEAD\ngit -C <repo> update-ref --stdin # {bulk_refs} create commands\n")
    };
    std::fs::write(
        temp.path().join("reftable-fixture.provenance"),
        format!(
            "{git_version}git init --quiet --initial-branch=main --object-format={hash} <repo>\n\
             git -C <repo> -c user.name=Reftable Test -c user.email=reftable@example.com commit --quiet --allow-empty -m initial\n\
             git -C <repo> branch topic\n\
             git -C <repo> branch deleted\n\
             git -C <repo> -c user.name=Reftable Test -c user.email=reftable@example.com tag -a v1 -m annotated\n\
             git -C <repo> symbolic-ref refs/meta/current refs/heads/main\n\
             {bulk_step}\
             git -C <repo> refs migrate --ref-format=reftable\n\
             git -C <repo> update-ref -d refs/heads/deleted\n"
        ),
    )?;
    Ok(temp)
}

fn table_files(repo: &Path) -> TestResult<Vec<PathBuf>> {
    let mut files = std::fs::read_dir(repo.join(".git/reftable"))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    files.retain(|path| path.extension() == Some(OsStr::new("ref")));
    files.sort();
    Ok(files)
}

fn rev_parse(repo: &Path, spec: &str) -> TestResult<ObjectId> {
    let output = git_ok(Some(repo), ["rev-parse", spec])?;
    Ok(ObjectId::from_hex(String::from_utf8(output.stdout)?.trim().as_bytes())?)
}

fn newest_ref<'a>(tables: &'a [Table], name: &[u8]) -> Option<&'a RefRecord> {
    tables.iter().rev().find_map(|table| table.find_ref(name))
}

fn parse_git_table(hash: &str, expected_kind: Kind, expected_version: Version, bulk_refs: usize) -> TestResult {
    let repo = create_migrated_repo(hash, bulk_refs)?;
    let commit_id = rev_parse(repo.path(), "HEAD")?;
    let tag_target_id = rev_parse(repo.path(), "refs/tags/v1")?;
    let tag_peeled_id = rev_parse(repo.path(), "refs/tags/v1^{}")?;
    let provenance = std::fs::read_to_string(repo.path().join("reftable-fixture.provenance"))?;
    assert!(
        provenance.starts_with("git version "),
        "each generated fixture records the Git version"
    );
    assert!(
        provenance.contains("refs migrate --ref-format=reftable"),
        "each generated fixture records the command sequence"
    );
    let files = table_files(repo.path())?;
    assert!(
        files.len() >= 2,
        "deleting a migrated reference leaves a newer table containing its tombstone"
    );
    let tables = files
        .iter()
        .map(|path| Table::read(path, Limits::default()))
        .collect::<Result<Vec<_>, _>>()?;
    for table in &tables {
        assert_eq!(
            table.header().version,
            expected_version,
            "Git selects the expected reftable version for the object format"
        );
        assert_eq!(
            table.header().object_hash,
            expected_kind,
            "Git records the repository object hash in every table header"
        );
    }

    let head = newest_ref(&tables, b"HEAD").expect("Git writes HEAD");
    assert!(
        matches!(&head.value, RefValue::Symbolic(target) if target.as_slice() == b"refs/heads/main"),
        "Git's symbolic HEAD retains its exact target: {head:?}"
    );
    let main = newest_ref(&tables, b"refs/heads/main").expect("Git writes its primary branch");
    assert_eq!(
        &main.value,
        &RefValue::Direct(commit_id),
        "Git's direct reference retains its exact object ID"
    );
    let symbolic = newest_ref(&tables, b"refs/meta/current").expect("Git writes the symbolic metadata ref");
    assert!(
        matches!(&symbolic.value, RefValue::Symbolic(target) if target.as_slice() == b"refs/heads/main"),
        "Git's nonstandard symbolic reference retains its exact target: {symbolic:?}"
    );
    let tag = newest_ref(&tables, b"refs/tags/v1").expect("Git writes the annotated tag");
    assert_eq!(
        &tag.value,
        &RefValue::Peeled {
            target: tag_target_id,
            peeled: tag_peeled_id,
        },
        "Git's annotated tag retains both the target and peeled object IDs"
    );
    let deleted = newest_ref(&tables, b"refs/heads/deleted").expect("Git writes a deletion tombstone");
    assert_eq!(
        deleted.value,
        RefValue::Deletion,
        "Git's deletion is decoded as a tombstone"
    );
    for idx in 0..bulk_refs {
        let name = format!("refs/heads/bulk/{idx:05}");
        assert!(
            newest_ref(&tables, name.as_bytes()).is_some(),
            "all bulk references used to exercise indexes are decoded"
        );
    }
    assert!(
        tables
            .iter()
            .rev()
            .any(|table| table.logs_for(b"refs/heads/main").next().is_some()),
        "Git's migrated reflog is decoded"
    );
    Ok(())
}

fn install_written_table(hash_name: &str, kind: Kind, version: Version) -> TestResult {
    let repo = create_migrated_repo(hash_name, 0)?;
    let commit_id = rev_parse(repo.path(), "HEAD")?;
    let tag_target_id = rev_parse(repo.path(), "refs/tags/v1")?;
    let tag_peeled_id = rev_parse(repo.path(), "refs/tags/v1^{}")?;
    let mut refs = vec![
        RefRecord {
            name: BString::from("HEAD"),
            update_index: 1,
            value: RefValue::Symbolic(BString::from("refs/heads/from-gix")),
        },
        RefRecord {
            name: BString::from("refs/heads/from-gix"),
            update_index: 1,
            value: RefValue::Direct(commit_id),
        },
        RefRecord {
            name: BString::from("refs/heads/deleted-by-gix"),
            update_index: 1,
            value: RefValue::Deletion,
        },
        RefRecord {
            name: BString::from("refs/tags/from-gix"),
            update_index: 1,
            value: RefValue::Peeled {
                target: tag_target_id,
                peeled: tag_peeled_id,
            },
        },
    ];
    for idx in 0..32 {
        refs.push(RefRecord {
            name: BString::from(format!("refs/heads/generated/{idx:03}")),
            update_index: 1,
            value: RefValue::Direct(commit_id),
        });
    }
    let logs = vec![LogRecord {
        ref_name: BString::from("refs/heads/from-gix"),
        update_index: 1,
        value: LogValue::Update {
            old_id: kind.null(),
            new_id: commit_id,
            name: BString::from("Reftable Test"),
            email: BString::from("reftable@example.com"),
            time: 1_700_000_000,
            tz_offset: 0,
            message: BString::from("written by gix-reftable"),
        },
    }];
    let bytes = Writer::new(WriteOptions {
        version,
        object_hash: kind,
        block_size: 128,
        restart_interval: 4,
        ..WriteOptions::default()
    })
    .write(&refs, &logs)?;

    let reftable_dir = repo.path().join(".git/reftable");
    for path in table_files(repo.path())? {
        std::fs::remove_file(path)?;
    }
    let filename = "0x000000000001-0x000000000001-gix.ref";
    std::fs::write(reftable_dir.join(filename), bytes)?;
    std::fs::write(reftable_dir.join("tables.list"), format!("{filename}\n"))?;

    let resolved = git_ok(Some(repo.path()), ["rev-parse", "refs/heads/from-gix"])?;
    assert_eq!(
        String::from_utf8(resolved.stdout)?.trim(),
        commit_id.to_string(),
        "Git resolves a direct reference written by this crate"
    );
    let symbolic = git_ok(Some(repo.path()), ["symbolic-ref", "HEAD"])?;
    assert_eq!(
        String::from_utf8(symbolic.stdout)?.trim(),
        "refs/heads/from-gix",
        "Git resolves a symbolic reference written by this crate"
    );
    assert_eq!(
        rev_parse(repo.path(), "refs/tags/from-gix")?,
        tag_target_id,
        "Git reads an annotated-tag target written by this crate"
    );
    assert_eq!(
        rev_parse(repo.path(), "refs/tags/from-gix^{}")?,
        tag_peeled_id,
        "Git peels an annotated tag written by this crate"
    );
    let deleted = gix_testtools::isolated_git_command(Some(repo.path()))
        .args(["show-ref", "--verify", "--quiet", "refs/heads/deleted-by-gix"])
        .output()?;
    assert_eq!(
        deleted.status.code(),
        Some(1),
        "Git treats a tombstone written by this crate as an absent reference"
    );
    let reflog = git_ok(
        Some(repo.path()),
        ["reflog", "show", "--format=%gs", "refs/heads/from-gix"],
    )?;
    assert_eq!(
        String::from_utf8(reflog.stdout)?.trim(),
        "written by gix-reftable",
        "Git reads a reflog message written by this crate"
    );
    let points_at = git_ok(
        Some(repo.path()),
        [
            "for-each-ref",
            "--format=%(refname)",
            &format!("--points-at={commit_id}"),
            "refs/heads/generated",
        ],
    )?;
    assert_eq!(
        String::from_utf8(points_at.stdout)?.lines().count(),
        32,
        "Git can traverse the object and ref indexes written by this crate"
    );
    Ok(())
}

#[test]
fn reads_tables_written_by_git() -> TestResult {
    if gix_testtools::should_skip_as_git_version_is_smaller_than(2, 46, 0) {
        eprintln!("skipping because the installed Git cannot migrate repositories to reftable");
        return Ok(());
    }
    parse_git_table("sha1", Kind::Sha1, Version::V1, 800)?;
    parse_git_table("sha256", Kind::Sha256, Version::V2, 0)?;
    Ok(())
}

#[test]
fn git_reads_tables_written_by_this_crate() -> TestResult {
    if gix_testtools::should_skip_as_git_version_is_smaller_than(2, 46, 0) {
        eprintln!("skipping because the installed Git cannot migrate repositories to reftable");
        return Ok(());
    }
    install_written_table("sha1", Kind::Sha1, Version::V1)
        .map_err(|err| format!("SHA-1 writer interoperability failed: {err}"))?;
    install_written_table("sha256", Kind::Sha256, Version::V2)
        .map_err(|err| format!("SHA-256 writer interoperability failed: {err}"))?;
    Ok(())
}

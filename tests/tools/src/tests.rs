use super::*;

#[test]
fn parse_version() {
    assert_eq!(git_version_from_bytes(b"git version 2.37.2").unwrap(), (2, 37, 2));
    assert_eq!(
        git_version_from_bytes(b"git version 2.32.1 (Apple Git-133)").unwrap(),
        (2, 32, 1)
    );
}

#[test]
fn parse_version_with_trailing_newline() {
    assert_eq!(git_version_from_bytes(b"git version 2.37.2\n").unwrap(), (2, 37, 2));
}

const SCOPE_ENV_VALUE: &str = "gitconfig";

fn populate_ad_hoc_config_files(dir: &Path) {
    const CONFIG_DATA: &[u8] = b"[foo]\n\tbar = baz\n";

    let paths: &[PathBuf] = if cfg!(windows) {
        let unc_literal_nul = dir.canonicalize().expect("directory exists").join("nul");
        &[dir.join(SCOPE_ENV_VALUE), dir.join("-"), unc_literal_nul]
    } else {
        &[dir.join(SCOPE_ENV_VALUE), dir.join("-"), dir.join(":")]
    };
    // Create the files.
    for path in paths {
        std::fs::write(path, CONFIG_DATA).expect("can write contents");
    }
    // Verify the files. This is mostly to show we really made a `\\?\...\nul` on Windows.
    for path in paths {
        let buf = std::fs::read(path).expect("the file really exists");
        assert_eq!(buf, CONFIG_DATA, "{path:?} should be a config file");
    }
}

#[test]
fn configure_command_clears_external_config() {
    let temp = tempfile::TempDir::new().expect("can create temp dir");
    populate_ad_hoc_config_files(temp.path());

    let mut cmd = std::process::Command::new(GIT_PROGRAM);
    cmd.env("GIT_CONFIG_SYSTEM", SCOPE_ENV_VALUE);
    cmd.env("GIT_CONFIG_GLOBAL", SCOPE_ENV_VALUE);
    configure_command(
        &mut cmd,
        gix_hash::Kind::default(),
        ["config", "-l", "--show-origin"],
        temp.path(),
    );

    let output = cmd.output().expect("can run git");
    let lines: Vec<_> = output
        .stdout
        .to_str()
        .expect("valid UTF-8")
        .lines()
        .filter(|line| !line.starts_with("command line:\t"))
        .collect();
    let status = output.status.code().expect("terminated normally");
    assert_eq!(lines, Vec::<&str>::new(), "should be no config variables from files");
    assert_eq!(status, 0, "reading the config should succeed");
}

#[test]
fn configure_command_overrides_xdg_config_home() {
    let temp = tempfile::TempDir::new().expect("can create temp dir");
    let mut cmd = std::process::Command::new(GIT_PROGRAM);
    cmd.env("XDG_CONFIG_HOME", temp.path().join("external-config"));
    configure_command(&mut cmd, gix_hash::Kind::default(), ["--version"], temp.path());

    let xdg_config_home = cmd
        .get_envs()
        .find_map(|(key, value)| (key == "XDG_CONFIG_HOME").then_some(value))
        .flatten();
    assert_eq!(
        xdg_config_home,
        Some(temp.path().join(".gix-testtools-xdg-config").as_os_str())
    );
}

#[test]
#[cfg(windows)]
fn bash_program_ok_for_platform() {
    let path = bash_program();
    assert!(path.is_absolute());

    let for_version = std::process::Command::new(path)
        .arg("--version")
        .output()
        .expect("can pass it `--version`");
    assert!(for_version.status.success(), "passing `--version` succeeds");
    for_version
        .stdout
        .lines()
        .nth(0)
        .expect("`--version` output has first line");

    let for_uname_os = std::process::Command::new(path)
        .args(["-c", "uname -o"])
        .output()
        .expect("can tell it to run `uname -o`");
    assert!(for_uname_os.status.success(), "telling it to run `uname -o` succeeds");
    assert_eq!(
        for_uname_os.stdout.trim_end(),
        b"Msys",
        "it runs commands in an MSYS environment"
    );
}

#[test]
#[cfg(not(windows))]
fn bash_program_ok_for_platform() {
    assert_eq!(bash_program(), Path::new("bash"));
}

#[test]
fn bash_program_unix_path() {
    let path = bash_program()
        .to_str()
        .expect("This test depends on the bash path being valid Unicode");
    assert!(
        !path.contains('\\'),
        "The path to bash should have no backslashes, barring very unusual environments"
    );
}

fn is_rooted_relative(path: impl AsRef<Path>) -> bool {
    let p = path.as_ref();
    p.is_relative() && p.has_root()
}

#[test]
#[cfg(windows)]
fn unix_style_absolute_is_rooted_relative() {
    assert!(is_rooted_relative("/bin/bash"), "can detect paths like /bin/bash");
}

#[test]
fn bash_program_absolute_or_unrooted() {
    let bash = bash_program();
    assert!(!is_rooted_relative(bash), "{bash:?}");
}

#[test]
fn invoke_bash_runs_in_given_working_directory() {
    let dir = tempfile::TempDir::new().expect("can create temp dir");
    invoke_bash(dir.path(), "printf '%s' hello > out");
    assert_eq!(
        std::fs::read(dir.path().join("out")).expect("script wrote output"),
        b"hello"
    );
}

#[test]
fn split_git_arguments_handles_multiline_whitespace() {
    assert_eq!(
        split_git_arguments(
            "log
             --graph
             --oneline",
        )
        .expect("valid arguments"),
        ["log", "--graph", "--oneline"]
    );
}

#[test]
fn split_git_arguments_handles_quoted_arguments() {
    assert_eq!(
        split_git_arguments(
            "commit
             -m 'subject with spaces'
             --author=\"A U Thor <author@example.com>\"",
        )
        .expect("valid arguments"),
        [
            "commit",
            "-m",
            "subject with spaces",
            "--author=A U Thor <author@example.com>"
        ]
    );
}

#[test]
fn split_git_arguments_handles_empty_quoted_arguments() {
    assert_eq!(
        split_git_arguments("diff -- pathspec:''").expect("valid arguments"),
        ["diff", "--", "pathspec:"]
    );
    assert_eq!(
        split_git_arguments("diff -- ''").expect("valid arguments"),
        ["diff", "--", ""]
    );
}

#[test]
fn split_git_arguments_handles_escaped_whitespace() {
    assert_eq!(
        split_git_arguments(r"add path\ with\ spaces").expect("valid arguments"),
        ["add", "path with spaces"]
    );
}

#[test]
fn split_git_arguments_concatenates_quoted_and_unquoted_parts() {
    assert_eq!(
        split_git_arguments(r#"commit -m prefix" quoted "suffix"#).expect("valid arguments"),
        ["commit", "-m", "prefix quoted suffix"]
    );
}

#[test]
fn split_git_arguments_rejects_unterminated_quotes() {
    assert!(split_git_arguments("commit -m 'unterminated").is_err());
    assert!(split_git_arguments("commit -m \"unterminated").is_err());
}

#[test]
fn normalize_debug_snapshot_returns_replaced_ids_by_placeholder_index() {
    let first = gix_hash::ObjectId::from_hex(b"e69de29bb2d1d6434b8b29ae775ad8c2e48c5391").expect("valid SHA1");
    let second = gix_hash::ObjectId::from_hex(b"496d6428b9cf92981dc9495211e6e1120fb6f2ba").expect("valid SHA1");
    let (snapshot, ids) = normalize_debug_snapshot(&vec![first, first, second, first]);

    assert_eq!(ids, vec![first, second]);
    assert_eq!(
        snapshot,
        r#"[
    Oid(1),
    Oid(1),
    Oid(2),
    Oid(1),
]"#
    );
}

#[test]
fn normalize_hashes_replaces_raw_object_ids() {
    let sha1 = gix_hash::ObjectId::from_hex(b"e69de29bb2d1d6434b8b29ae775ad8c2e48c5391").expect("valid SHA1");
    let sha256 = gix_hash::ObjectId::from_hex(b"473a0f4c3be8a93681a267e3b1e9a7dcda1185436fe141f7749120a303721813")
        .expect("valid SHA256");

    let (snapshot, ids) = normalize_hashes(
        "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 \
         473a0f4c3be8a93681a267e3b1e9a7dcda1185436fe141f7749120a303721813 \
         e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
    );

    assert_eq!(ids, vec![sha1, sha256]);
    assert_eq!(snapshot, "Oid(1) Oid(2) Oid(1)");
}

#[test]
#[cfg(not(feature = "worktree-exclusions"))]
fn gitignore_fallback_matches_archive_basename_patterns() {
    let lines = "\n# generated fixture archives\nrust-*.tar\n";

    assert!(is_excluded_by_lines(
        lines,
        Path::new("tests/fixtures/generated-archives/rust-basic.tar")
    ));
    assert!(!is_excluded_by_lines(
        lines,
        Path::new("tests/fixtures/generated-archives/script-basic.tar")
    ));
}

#[test]
#[cfg(not(feature = "worktree-exclusions"))]
fn gitignore_fallback_matches_paths_relative_to_fixture_base() {
    let lines = "generated-archives/rust-*.tar\n";

    assert!(is_excluded_by_lines(
        lines,
        Path::new("generated-archives/rust-basic.tar")
    ));
    assert!(!is_excluded_by_lines(
        lines,
        Path::new("other-generated-archives/rust-basic.tar")
    ));
}

#[test]
#[cfg(not(feature = "worktree-exclusions"))]
fn gitignore_fallback_treats_leading_slash_as_rooted_pattern() {
    let lines = "/generated-archives/rust-*.tar\n";

    assert!(is_excluded_by_lines(
        lines,
        Path::new("generated-archives/rust-basic.tar")
    ));
}

#[test]
#[cfg(not(feature = "worktree-exclusions"))]
fn gitignore_fallback_ignores_blank_lines_and_comments() {
    let lines = "\n  \n# generated-archives/rust-*.tar\ngenerated-archives/script-*.tar\n";

    assert!(is_excluded_by_lines(
        lines,
        Path::new("generated-archives/script-basic.tar")
    ));
    assert!(!is_excluded_by_lines(
        lines,
        Path::new("generated-archives/rust-basic.tar")
    ));
}

#[test]
#[cfg(not(feature = "worktree-exclusions"))]
fn gitignore_fallback_normalizes_windows_path_separators() {
    let lines = "generated-archives/rust-*.tar\n";

    assert!(is_excluded_by_lines(
        lines,
        Path::new(r"generated-archives\rust-basic.tar")
    ));
}

#[test]
fn archive_required_fixtures_use_a_separate_cache_directory() {
    // Archive-required fixtures must not share the normal generated fixture
    // cache. Otherwise, a previous script run can leave platform-specific
    // output behind and make a later archive-required request skip extraction.
    // Using different paths makes sure they are actually from the archive if they exist.
    let fixture_base = Path::new("tests").join("fixtures");
    let (_, generated_dir) = force_and_dir(None, &fixture_base, "scripted", Some(gix_hash::Kind::Sha1), &1234, None);
    let (_, archived_dir) = force_and_dir(
        None,
        &fixture_base,
        "scripted",
        Some(gix_hash::Kind::Sha1),
        &1234,
        Some("archive"),
    );

    assert_ne!(generated_dir, archived_dir);
    assert!(
        archived_dir
            .components()
            .any(|component| component.as_os_str() == "archive")
    );
}

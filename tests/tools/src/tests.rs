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
        gix_hash::Kind::Sha1,
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
#[cfg(windows)]
fn bash_program_ok_for_platform() {
    let path = bash_program();
    assert!(path.is_absolute());

    let for_version = std::process::Command::new(path)
        .arg("--version")
        .output()
        .expect("can pass it `--version`");
    assert!(for_version.status.success(), "passing `--version` succeeds");
    let version_line = for_version
        .stdout
        .lines()
        .nth(0)
        .expect("`--version` output has first line");
    assert!(
        version_line.ends_with(b"-pc-msys)"), // On Windows, "-pc-linux-gnu)" would be WSL.
        "it is an MSYS bash (such as Git Bash)"
    );

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

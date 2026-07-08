use std::path::Path;

#[test]
fn exe_invocation() {
    let actual = gix_path::env::exe_invocation();
    assert!(
        !actual.as_os_str().is_empty(),
        "it finds something as long as git is installed somewhere on the system (or a default location)"
    );
}

#[test]
fn shell() {
    assert!(
        Path::new(gix_path::env::shell()).exists(),
        "On CI and on Unix we expect a usable path to the shell that exists on disk"
    );
}

#[test]
fn shell_absolute() {
    assert!(
        Path::new(gix_path::env::shell()).is_absolute(),
        "On CI and on Unix we currently expect the path to the shell always to be absolute"
    );
}

#[test]
fn shell_unix_path() {
    let shell = gix_path::env::shell()
        .to_str()
        .expect("This test depends on the shell path being valid Unicode");
    assert!(
        !shell.contains('\\'),
        "The path to the shell should have no backslashes, barring strange `GIT_EXEC_PATH` values"
    );
}

#[test]
fn installation_config() {
    assert_ne!(
        gix_path::env::installation_config().map(|p| p.components().count()),
        gix_path::env::installation_config_prefix().map(|p| p.components().count()),
        "the prefix is a bit shorter than the installation config path itself"
    );
}

#[test]
fn core_dir() {
    assert!(
        gix_path::env::core_dir()
            .expect("Git is always in PATH when we run tests")
            .is_dir(),
        "The core directory is a valid directory"
    );
}

#[test]
fn core_dir_program() {
    let core_dir = gix_path::env::core_dir().expect("Git is always in PATH when we run tests");
    let program = std::fs::read_dir(core_dir)
        .expect("the core directory can be listed")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_file()))
        .find_map(|entry| {
            let name = entry.file_name();
            let stem = name.to_str()?.strip_suffix(std::env::consts::EXE_SUFFIX)?;
            stem.starts_with("git-").then(|| stem.to_owned())
        });
    // Which programs are present as separate executables depends on how Git was built -
    // Git for Windows, for instance, may not provide programs for builtin subcommands.
    if let Some(stem) = program {
        let path =
            gix_path::env::core_dir_program(&stem).expect("a program listed in the core directory is found there");
        assert!(path.is_file(), "the returned path refers to an existing file");
        assert!(
            path.is_absolute(),
            "the path is absolute as it is based on `git --exec-path`"
        );
    }
    assert_eq!(
        gix_path::env::core_dir_program("git-program-that-does-not-exist"),
        None,
        "programs that don't exist in the core directory are not found"
    );
}

#[test]
fn system_prefix() {
    assert_ne!(
        gix_path::env::system_prefix(),
        None,
        "git should be present when running tests"
    );
}

#[test]
fn home_dir() {
    assert_ne!(
        gix_path::env::home_dir(),
        None,
        "we find a home on every system these tests execute"
    );
}

mod xdg_config {
    use std::ffi::OsStr;

    #[test]
    fn prefers_xdg_config_bases() {
        let actual = gix_path::env::xdg_config("test", &mut |n| {
            (n == OsStr::new("XDG_CONFIG_HOME")).then(|| "marker".into())
        })
        .expect("set");
        #[cfg(unix)]
        assert_eq!(actual.to_str(), Some("marker/git/test"));
        #[cfg(windows)]
        assert_eq!(actual.to_str(), Some(r"marker\git\test"));
    }

    #[test]
    fn falls_back_to_home() {
        let actual = gix_path::env::xdg_config("test", &mut |n| (n == OsStr::new("HOME")).then(|| "marker".into()))
            .expect("set");
        #[cfg(unix)]
        assert_eq!(actual.to_str(), Some("marker/.config/git/test"));
        #[cfg(windows)]
        assert_eq!(actual.to_str(), Some(r"marker\.config\git\test"));
    }
}

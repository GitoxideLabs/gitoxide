use gix_hook::{Context, Options};
use gix_testtools::TestResult;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

fn fixture() -> TestResult<PathBuf> {
    Ok(gix_testtools::scripted_fixture_read_only("make_hooks.sh")?.canonicalize()?)
}

fn context<'a>(workdir: &'a Path, common_dir: &'a Path) -> Context<'a> {
    Context {
        working_dir: workdir,
        common_dir,
        command: gix_command::Context {
            git_dir: Some(common_dir.into()),
            worktree_dir: Some(workdir.into()),
            ..Default::default()
        },
    }
}

#[test]
fn discovery_validates_names_and_execute_permissions() -> TestResult {
    let fixture = fixture()?;
    let directory = fixture.join("repo/.git/hooks");
    assert_eq!(
        gix_hook::find("pre-commit", &directory)?,
        Some(directory.join("pre-commit"))
    );
    assert!(gix_hook::find("missing", &directory)?.is_none());
    assert!(gix_hook::find("directory", &directory)?.is_none());
    for name in [
        "",
        ".",
        "..",
        "../pre-commit",
        "hooks/pre-commit",
        "hooks\\pre-commit",
        "C:hook",
        "pre\0commit",
    ] {
        assert!(
            gix_hook::find(name, &directory)
                .expect_err("invalid hook name")
                .is_validation()
        );
    }
    #[cfg(unix)]
    assert!(gix_hook::find("post-commit", &directory)?.is_none());
    assert_eq!(
        gix_hook::find("applypatch-msg", &directory)?.is_some(),
        fixture.join("mode-hook-executable").exists(),
        "execute permission agrees with Git"
    );
    Ok(())
}

#[test]
fn preparation_keeps_resolved_context_and_literal_arguments() -> TestResult {
    let fixture = fixture()?;
    let workdir = fixture.join("repo");
    let common_dir = workdir.join(".git");
    let executable = std::env::current_exe()?;
    let options = Options {
        arguments: &[OsStr::new("one $literal argument")],
        environment: &[(OsStr::new("GIT_COMMON_DIR"), OsStr::new("explicit-common"))],
        ..Default::default()
    };
    let command = gix_hook::prepare(&executable, &context(&workdir, &common_dir), options)?;
    assert_eq!(
        command.get_program(),
        executable.as_os_str(),
        "native paths stay literal"
    );
    assert_eq!(
        command.get_current_dir(),
        Some(workdir.as_path()),
        "the caller supplies the repository directory"
    );
    assert_eq!(
        command.get_args().collect::<Vec<_>>(),
        options.arguments,
        "arguments are passed without shell interpretation"
    );
    assert!(
        command
            .get_envs()
            .any(|(name, value)| name == "GIT_COMMON_DIR" && value == Some(OsStr::new("explicit-common"))),
        "explicit environment wins over repository context"
    );
    Ok(())
}

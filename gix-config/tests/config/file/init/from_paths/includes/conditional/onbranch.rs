use std::fs;

use bstr::{BString, ByteSlice};
use gix_config::file::{
    includes,
    includes::conditional,
    init::{self},
};
use gix_ref::FullName;
use gix_testtools::tempfile::tempdir;

use crate::file::{bstring, init::from_paths::includes::conditional::git_init};

type Result = crate::Result;

#[test]
fn literal_branch_names_match() -> Result {
    assert_section_value(
        Options {
            condition: "literal-match",
            branch_name: "refs/heads/literal-match",
            expect: Value::OverrideByInclude,
        },
        &mut GitEnv::new()?,
    )?;
    Ok(())
}

#[test]
fn full_ref_names_do_not_match() -> Result {
    assert_section_value(
        Options {
            condition: "refs/heads/simple",
            branch_name: "refs/heads/simple",
            expect: Value::Base,
        },
        &mut GitEnv::new()?,
    )?;
    Ok(())
}

#[test]
fn non_branches_never_match() -> Result {
    assert_section_value(
        Options {
            condition: "good",
            branch_name: "refs/bisect/good",
            expect: Value::Base,
        },
        &mut GitEnv::new()?,
    )?;
    Ok(())
}

#[test]
fn patterns_ending_with_slash_match_subdirectories_recursively() -> Result {
    let mut env = GitEnv::new()?;
    assert_section_value(
        Options {
            condition: "feature/b/",
            branch_name: "refs/heads/feature/b/start",
            expect: Value::OverrideByInclude,
        },
        &mut env,
    )?;
    assert_section_value(
        Options {
            condition: "feature/",
            branch_name: "refs/heads/feature/b/start",
            expect: Value::OverrideByInclude,
        },
        &mut env,
    )?;
    assert_section_value_msg(
        Options {
            condition: "feature/b/start",
            branch_name: "refs/heads/feature/b/start",
            expect: Value::OverrideByInclude,
        },
        &mut env,
        "just for good measure, we would expect branch paths to work as well".into(),
    )?;
    Ok(())
}

#[test]
fn simple_glob_patterns() -> Result {
    let mut env = GitEnv::new()?;
    assert_section_value(
        Options {
            condition: "prefix*",
            branch_name: "refs/heads/prefixsuffix",
            expect: Value::OverrideByInclude,
        },
        &mut env,
    )?;
    assert_section_value_msg(
        Options {
            condition: "prefix*",
            branch_name: "refs/heads/prefix/suffix",
            expect: Value::Base,
        },
        &mut env,
        "single-stars do not cross component boundaries".into(),
    )?;
    assert_section_value(
        Options {
            condition: "*suffix",
            branch_name: "refs/heads/prefixsuffix",
            expect: Value::OverrideByInclude,
        },
        &mut env,
    )?;
    assert_section_value(
        Options {
            condition: "*/suffix",
            branch_name: "refs/heads/prefix/suffix",
            expect: Value::OverrideByInclude,
        },
        &mut env,
    )?;
    assert_section_value_msg(
        Options {
            condition: "*suffix",
            branch_name: "refs/heads/prefix/suffix",
            expect: Value::Base,
        },
        &mut env,
        "single-stars do not cross component boundaries".into(),
    )?;
    Ok(())
}

#[test]
fn simple_globs_do_not_cross_component_boundary() -> Result {
    let mut env = GitEnv::new()?;
    assert_section_value(
        Options {
            condition: "feature/*/start",
            branch_name: "refs/heads/feature/a/start",
            expect: Value::OverrideByInclude,
        },
        &mut env,
    )?;
    assert_section_value_msg(
        Options {
            condition: "feature/*/start",
            branch_name: "refs/heads/feature/a/b/start",
            expect: Value::Base,
        },
        &mut env,
        "path matching would never match 'a/b' as it cannot cross /".into(),
    )?;
    Ok(())
}

#[test]
fn double_star_globs_cross_component_boundaries() -> Result {
    assert_section_value(
        Options {
            condition: "feature/**/start",
            branch_name: "refs/heads/feature/a/b/start",
            expect: Value::OverrideByInclude,
        },
        &mut GitEnv::new()?,
    )?;
    Ok(())
}

enum Value {
    Base,
    OverrideByInclude,
}

#[derive(Debug)]
struct GitEnv {
    dir: gix_testtools::tempfile::TempDir,
}

impl GitEnv {
    fn new() -> crate::Result<Self> {
        let dir = tempdir()?;
        git_init(dir.path(), true)?;
        Ok(GitEnv { dir })
    }
}

struct Options<'a> {
    condition: &'a str,
    branch_name: &'a str,
    expect: Value,
}

fn assert_section_value(opts: Options, env: &mut GitEnv) -> crate::Result {
    assert_section_value_msg(opts, env, None)
}

fn assert_section_value_msg(
    Options {
        condition,
        branch_name,
        expect,
    }: Options,
    GitEnv { dir }: &mut GitEnv,
    message: Option<&str>,
) -> crate::Result<()> {
    let root_config = dir.path().join("config");
    let included_config = dir.path().join("include.config");

    fs::write(
        &root_config,
        format!(
            r#"
[section]
value = base-value

[includeIf "onbranch:{condition}"]
path = ./include.config"#,
        ),
    )?;

    fs::write(
        included_config,
        r#"
[section]
value = branch-override-by-include
"#,
    )?;

    let branch_name = FullName::try_from(BString::from(branch_name))?;
    let options = init::Options {
        includes: includes::Options::follow(
            Default::default(),
            conditional::Context {
                branch_name: Some(branch_name.as_ref()),
                ..Default::default()
            },
        ),
        ..Default::default()
    };

    let config = gix_config::File::from_paths_metadata(
        Some(gix_config::file::Metadata::try_from_path(
            &root_config,
            gix_config::Source::Local,
        )?),
        options,
    )?
    .expect("non-empty");
    assert_eq!(
        config.string_by("section", None, "value"),
        Some(bstring(match expect {
            Value::OverrideByInclude => "branch-override-by-include",
            Value::Base => "base-value",
        })),
        "{}, info: {:?}, debug at {:?}",
        match expect {
            Value::Base => "the base value should not be overridden as the branch does not match",
            Value::OverrideByInclude =>
                "the base value is overridden by an included file because the condition matches",
        },
        message,
        {
            let dir = std::mem::replace(
                dir,
                gix_testtools::tempfile::TempDir::new().expect("substitute can be created"),
            );
            dir.keep()
        }
    );

    std::fs::write(dir.path().join("HEAD"), format!("ref: {}", branch_name.as_bstr()))?;
    assure_git_agrees(expect, dir)?;
    Ok(())
}

fn assure_git_agrees(expected: Value, dir: &mut gix_testtools::tempfile::TempDir) -> crate::Result {
    let git_dir = dir.path();
    let output = gix_testtools::git_command(git_dir)
        .args(["config", "--get", "section.value"])
        .env("GIT_DIR", git_dir)
        .env("HOME", git_dir)
        .env("USERPROFILE", git_dir)
        .env("GIT_CONFIG_GLOBAL", git_dir.join(".gitconfig"))
        .output()?;

    let mut keep_dir_on_disk = || {
        let dir = std::mem::replace(
            dir,
            gix_testtools::tempfile::TempDir::new().expect("substitute can be created"),
        );
        dir.keep()
    };
    assert!(
        output.status.success(),
        "{:?}, {:?} for debugging",
        output,
        keep_dir_on_disk()
    );
    let git_output: BString = output.stdout.trim_end().into();
    assert_eq!(
        git_output,
        match expected {
            Value::Base => "base-value",
            Value::OverrideByInclude => "branch-override-by-include",
        },
        "git disagrees with gix-config, {:?} for debugging",
        keep_dir_on_disk()
    );
    Ok(())
}

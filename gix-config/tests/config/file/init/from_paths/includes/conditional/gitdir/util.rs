#![cfg_attr(windows, allow(dead_code))]

use std::{
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

use bstr::{BString, ByteSlice};
use gix_config::file::init::{self};

use crate::file::{
    bstring,
    init::from_paths::{
        escape_backslashes,
        includes::conditional::{git_init, options_with_git_dir},
    },
};

#[derive(Debug)]
pub struct GitEnv {
    tempdir: gix_testtools::tempfile::TempDir,
    root_dir: PathBuf,
    git_dir: PathBuf,
    home_dir: PathBuf,
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum ConfigLocation {
    Repo,
    User,
}

#[derive(Copy, Clone)]
enum Value {
    Original,
    Override,
}

pub struct Condition {
    condition: String,
    value: Option<Value>,
    config_location: ConfigLocation,
}

impl Condition {
    pub fn new(condition: impl Into<String>) -> Self {
        Condition {
            condition: condition.into(),
            value: Value::Override.into(),
            config_location: ConfigLocation::Repo,
        }
    }
    pub fn set_user_config_instead_of_repo_config(mut self) -> Self {
        self.config_location = ConfigLocation::User;
        self
    }
    pub fn expect_original_value(mut self) -> Self {
        self.value = Value::Original.into();
        self
    }

    pub fn expect_no_value(mut self) -> Self {
        self.value = None;
        self
    }
}

impl GitEnv {
    pub fn repo_name(repo_name: impl AsRef<Path>) -> crate::Result<Self> {
        let tempdir = gix_testtools::tempfile::tempdir()?;
        let root_dir = gix_path::realpath(tempdir.path())?;
        let worktree_dir = root_dir.join(repo_name);
        std::fs::create_dir_all(&worktree_dir)?;
        let home_dir = gix_path::realpath(tempdir.path())?;
        Ok(Self {
            tempdir,
            root_dir,
            git_dir: worktree_dir.join(".git"),
            home_dir,
        })
    }

    pub fn repo_in_home() -> crate::Result<Self> {
        Self::repo_name("")
    }
}

impl GitEnv {
    pub fn to_init_options(&self) -> init::Options<'_> {
        let mut opts = options_with_git_dir(self.git_dir());
        opts.includes.interpolate.home_dir = Some(self.home_dir());
        opts
    }

    pub fn git_dir(&self) -> &Path {
        &self.git_dir
    }
    pub fn set_git_dir(&mut self, git_dir: PathBuf) {
        self.git_dir = git_dir;
    }
    pub fn worktree_dir(&self) -> &Path {
        self.git_dir
            .parent()
            .expect("this is .git as subdirectory of the worktree")
    }
    pub fn home_dir(&self) -> &Path {
        &self.home_dir
    }
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }
}

pub fn assert_section_value(
    Condition {
        condition,
        value: expected,
        config_location,
    }: Condition,
    env: GitEnv,
) -> crate::Result {
    write_config(condition, &env, config_location)?;

    let mut paths = vec![env.git_dir().join("config")];
    if config_location == ConfigLocation::User {
        paths.push(env.home_dir().join(".gitconfig"));
    }

    let config = gix_config::File::from_paths_metadata(
        paths
            .into_iter()
            .map(|path| gix_config::file::Metadata::try_from_path(path, gix_config::Source::Local).unwrap()),
        env.to_init_options(),
    )?
    .expect("non-empty");

    assert_eq!(
        config.string_by("section", None, "value"),
        match expected {
            Some(Value::Original) => Some(bstring("base-value")),
            Some(Value::Override) => Some(bstring("override-value")),
            None => None,
        },
        "gix-config disagrees with the expected value, {} for debugging",
        env.tempdir.keep().display()
    );
    assure_git_agrees(expected, env)
}

pub fn git_env_with_symlinked_repo() -> crate::Result<GitEnv> {
    let mut env = GitEnv::repo_name("worktree")?;
    let link_destination = env.root_dir().join("symlink-worktree");
    crate::file::init::from_paths::includes::conditional::create_symlink(&link_destination, env.worktree_dir());

    let git_dir_through_symlink = link_destination.join(".git");
    env.set_git_dir(git_dir_through_symlink);
    Ok(env)
}

fn assure_git_agrees(expected: Option<Value>, env: GitEnv) -> crate::Result {
    let output = Command::new("git")
        .args(["config", "--get", "section.value"])
        .env("HOME", env.home_dir())
        .env("GIT_DIR", env.git_dir())
        .env_remove("GIT_CONFIG_COUNT")
        .env_remove("XDG_CONFIG_HOME")
        .current_dir(env.worktree_dir())
        .output()?;

    assert_eq!(
        output.status.success(),
        expected.is_some(),
        "{:?}, {} for debugging",
        output,
        env.tempdir.keep().display()
    );
    let git_output: BString = output.stdout.trim_end().into();
    assert_eq!(
        git_output,
        match expected {
            Some(Value::Original) => "base-value",
            Some(Value::Override) => "override-value",
            None => "",
        },
        "git disagrees with gix-config, {:?} for debugging",
        env.tempdir.keep()
    );
    Ok(())
}

fn write_config(condition: impl AsRef<str>, env: &GitEnv, overwrite_config_location: ConfigLocation) -> crate::Result {
    let include_config = write_included_config(env)?;
    write_main_config(condition, include_config, env, overwrite_config_location)
}

fn write_included_config(env: &GitEnv) -> crate::Result<PathBuf> {
    let include_path = env.worktree_dir().join("include.path");
    write_append_config_value(&include_path, "override-value")?;
    Ok(include_path)
}

fn write_append_config_value(path: impl AsRef<std::path::Path>, value: &str) -> crate::Result {
    let mut file = std::fs::OpenOptions::new().append(true).create(true).open(path)?;
    file.write_all(
        format!(
            "
[section]
value = {value}"
        )
        .as_bytes(),
    )?;
    Ok(())
}

fn write_main_config(
    condition: impl AsRef<str>,
    include_file_path: PathBuf,
    env: &GitEnv,
    overwrite_config_location: ConfigLocation,
) -> crate::Result {
    git_init(env.worktree_dir(), false)?;

    if overwrite_config_location == ConfigLocation::Repo {
        write_append_config_value(env.git_dir().join("config"), "base-value")?;
    }

    let config_file_path = match overwrite_config_location {
        ConfigLocation::User => env.home_dir().join(".gitconfig"),
        ConfigLocation::Repo => env.git_dir().join("config"),
    };

    let condition = condition.as_ref();
    let condition = {
        let c = condition
            .replace("$gitdir", &env.git_dir().to_string_lossy())
            .replace("$worktree", &env.worktree_dir().to_string_lossy());
        if c == condition {
            condition.to_owned()
        } else {
            escape_backslashes(c)
        }
    };
    let include_file_path = escape_backslashes(include_file_path);
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(config_file_path)?;
    file.write_all(
        format!(
            "
[includeIf \"{condition}\"]
path = {include_file_path}",
        )
        .as_bytes(),
    )?;
    Ok(())
}

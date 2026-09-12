mod util;

use gix_testtools::Env;
use serial_test::serial;
use util::{Condition, GitEnv, assert_section_value};

use crate::file::init::from_paths::escape_backslashes;

#[test]
fn relative_path_with_trailing_slash_matches_like_star_star() -> crate::Result {
    assert_section_value(Condition::new("gitdir:worktree/"), GitEnv::repo_name("worktree")?)
}

#[test]
fn relative_path_without_trailing_slash_does_not_match() -> crate::Result {
    assert_section_value(
        Condition::new("gitdir:worktree").expect_original_value(),
        GitEnv::repo_name("worktree")?,
    )
}

#[test]
fn relative_path_without_trailing_slash_and_dot_git_suffix_matches() -> crate::Result {
    assert_section_value(Condition::new("gitdir:worktree/.git"), GitEnv::repo_name("worktree")?)
}

#[test]
fn tilde_slash_expands_the_current_user_home() -> crate::Result {
    let env = GitEnv::repo_name(std::path::Path::new("subdir").join("worktree"))?;
    assert_section_value(Condition::new("gitdir:~/subdir/worktree/"), env)
}

#[test]
fn failed_user_expansion_matches_the_literal_pattern() -> crate::Result {
    let temp = gix_testtools::tempfile::tempdir()?;
    let name = format!(
        "~gix-config-{}",
        temp.path()
            .file_name()
            .expect("temporary directory has a name")
            .to_string_lossy()
    );
    let git_dir = temp.path().join(&name);
    super::git_init(&git_dir, true)?;
    std::fs::write(git_dir.join("included.config"), "[section]\nvalue = included\n")?;
    for condition in ["gitdir", "gitdir/i"] {
        std::fs::write(
            git_dir.join("config"),
            format!(
                r#"[core]
bare = true
[includeIf "{condition}:{name}"]
path = included.config
"#
            ),
        )?;
        assert_eq!(
            gix_testtools::git(&git_dir, "config --get section.value")?.trim_end(),
            "included",
            "Git matches the literal directory name when user expansion fails"
        );
        for strict in [false, true] {
            let mut options = super::options_with_git_dir(&git_dir);
            options.includes.interpolate.home_for_user = Some(|_| None);
            options.includes.err_on_interpolation_failure = strict;
            let result = gix_config::File::from_paths_metadata(
                Some(gix_config::file::Metadata::try_from_path(
                    git_dir.join("config"),
                    gix_config::Source::Local,
                )?),
                options,
            );
            if strict {
                assert!(result.is_err(), "strict mode still reports the failed lookup");
            } else {
                let config = result?.expect("the repository config exists");
                assert_eq!(
                    config.string("section.value"),
                    Some(crate::file::bstring("included")),
                    "{condition} preserves the original pattern when the user is unknown"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn tilde_alone_does_not_match_even_if_home_is_git_directory() -> crate::Result {
    let env = GitEnv::repo_in_home()?;
    assert_section_value(Condition::new("gitdir:~").expect_original_value(), env)
}

#[test]
fn explicit_star_star_prefix_and_suffix_match_zero_or_more_path_components() -> crate::Result {
    assert_section_value(Condition::new("gitdir:**/worktree/**"), GitEnv::repo_name("worktree")?)
}

#[test]
fn double_slash_does_not_match() -> crate::Result {
    assert_section_value(
        Condition::new("gitdir://worktree").expect_original_value(),
        GitEnv::repo_name("worktree")?,
    )
}

#[test]
fn absolute_git_dir_with_os_separators_match() -> crate::Result {
    assert_section_value(
        original_value_on_windows(Condition::new("gitdir:$gitdir")),
        GitEnv::repo_name("worktree")?,
    )
}

#[test]
fn absolute_worktree_dir_with_os_separators_does_not_match_if_trailing_slash_is_missing() -> crate::Result {
    assert_section_value(
        Condition::new("gitdir:$worktree").expect_original_value(),
        GitEnv::repo_name("worktree")?,
    )
}

#[test]
fn absolute_worktree_dir_with_os_separators_matches_with_trailing_glob() -> crate::Result {
    assert_section_value(
        original_value_on_windows(Condition::new(format!(
            "gitdir:$worktree{}**",
            std::path::MAIN_SEPARATOR
        ))),
        GitEnv::repo_name("worktree")?,
    )
}

#[test]
fn dot_slash_path_is_replaced_with_directory_containing_the_including_config_file() -> crate::Result {
    assert_section_value(
        Condition::new("gitdir:./").set_user_config_instead_of_repo_config(),
        GitEnv::repo_name("worktree")?,
        // the user configuration is in $HOME, which is parent to $HOME/worktree, and the pattern path ends up being $HOME/**, including worktree/.git
    )
}

#[test]
#[serial]
fn dot_slash_from_environment_causes_error() -> crate::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let env = GitEnv::repo_name("worktree")?;
    // Only slashes can be used as matches, even on Windows.
    let git_dir = env.git_dir().to_string_lossy().replace('\\', "/");

    {
        let _environment = Env::new()
            .set("GIT_CONFIG_COUNT", "1")
            .set("GIT_CONFIG_KEY_0", format!("includeIf.gitdir:{git_dir}.path"))
            .set("GIT_CONFIG_VALUE_0", "./include.path");

        let res = gix_config::File::from_env(env.to_init_options());
        assert!(
            matches!(
                res,
                Err(gix_config::file::init::from_env::Error::Includes(
                    gix_config::file::includes::Error::MissingConfigPath
                ))
            ),
            "this is a failure of resolving the include path, after trying to include it"
        );
    }

    let absolute_path = escape_backslashes(env.home_dir().join("include.config"));
    {
        let _environment = Env::new()
            .set("GIT_CONFIG_COUNT", "1")
            .set("GIT_CONFIG_KEY_0", "includeIf.gitdir:./worktree/.path")
            .set("GIT_CONFIG_VALUE_0", &absolute_path);

        let res = gix_config::File::from_env(env.to_init_options());
        assert!(
            matches!(
                res,
                Err(gix_config::file::init::from_env::Error::Includes(
                    gix_config::file::includes::Error::MissingConfigPath
                ))
            ),
            "here the pattern path tries to be resolved and fails as target config isn't set"
        );
    }

    {
        let _environment = Env::new()
            .set("GIT_CONFIG_COUNT", "1")
            .set("GIT_CONFIG_KEY_0", format!("includeIf.gitdir:{git_dir}.path"))
            .set("GIT_CONFIG_VALUE_0", absolute_path);

        let res = gix_config::File::from_env(env.to_init_options());
        assert!(res.is_ok(), "missing paths are ignored as before");
    }

    Ok(())
}

#[test]
fn dot_dot_slash_prefixes_are_not_special_and_are_not_what_you_want() -> crate::Result {
    assert_section_value(
        Condition::new("gitdir:../")
            .set_user_config_instead_of_repo_config()
            .expect_no_value(),
        GitEnv::repo_name("worktree")?,
    )
}

#[test]
fn leading_dots_are_not_special() -> crate::Result {
    assert_section_value(Condition::new("gitdir:.hidden/"), GitEnv::repo_name(".hidden")?)
}

#[test]
fn dot_slash_path_with_dot_git_suffix_matches() -> crate::Result {
    assert_section_value(
        Condition::new("gitdir:./worktree/.git").set_user_config_instead_of_repo_config(),
        GitEnv::repo_name("worktree")?,
    )
}

#[test]
fn globbing_and_wildcards() -> crate::Result {
    assert_section_value(
        Condition::new("gitdir:stan?ard/glo*ng/[xwz]ildcards/.git").set_user_config_instead_of_repo_config(),
        GitEnv::repo_name("standard/globbing/wildcards")?,
    )
}

#[test]
fn case_insensitive_matches_any_case() -> crate::Result {
    assert_section_value(Condition::new("gitdir/i:WORKTREE/"), GitEnv::repo_name("worktree")?)?;
    assert_section_value(
        Condition::new("gitdir:WORKTREE/").expect_original_value(),
        GitEnv::repo_name("worktree")?,
    )
}

#[test]
fn pattern_with_escaped_backslash() -> crate::Result {
    assert_section_value(
        original_value_on_windows(Condition::new(r"gitdir:\\work\\tree\\/")),
        GitEnv::repo_name("worktree")?,
    )
}

#[test]
fn pattern_with_backslash() -> crate::Result {
    assert_section_value(Condition::new(r"gitdir:work\tree/"), GitEnv::repo_name("worktree")?)
}

#[test]
fn star_star_in_the_middle() -> crate::Result {
    assert_section_value(
        Condition::new("gitdir:**/dir/**/worktree/**"),
        GitEnv::repo_name("dir/worktree")?,
    )
}

#[test]
#[cfg(not(windows))]
fn tilde_expansion_with_symlink() -> crate::Result {
    let env = util::git_env_with_symlinked_repo()?;
    assert_section_value(Condition::new("gitdir:~/worktree/"), env)
}

#[test]
#[cfg(not(windows))]
fn dot_path_with_symlink() -> crate::Result {
    let env = util::git_env_with_symlinked_repo()?;
    assert_section_value(
        Condition::new("gitdir:./symlink-worktree/.git").set_user_config_instead_of_repo_config(),
        env,
    )
}

#[test]
#[cfg(not(windows))]
fn relative_path_matching_symlink() -> crate::Result {
    let env = util::git_env_with_symlinked_repo()?;
    assert_section_value(
        Condition::new("gitdir:symlink-worktree/").set_user_config_instead_of_repo_config(),
        env,
    )
}

#[test]
#[cfg(not(windows))]
fn dot_path_matching_symlink_with_icase() -> crate::Result {
    let env = util::git_env_with_symlinked_repo()?;
    assert_section_value(
        Condition::new("gitdir/i:SYMLINK-WORKTREE/").set_user_config_instead_of_repo_config(),
        env,
    )
}

fn original_value_on_windows(c: Condition) -> Condition {
    if cfg!(windows) { c.expect_original_value() } else { c }
}

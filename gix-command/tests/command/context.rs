use gix_command::Context;

fn winfix(expected: impl Into<String>) -> String {
    // Unclear why it's not debug-printing the env on windows.
    if cfg!(windows) { "\"\"".into() } else { expected.into() }
}

#[test]
fn git_dir_sets_git_dir_env_and_cwd() {
    let ctx = Context {
        git_dir: Some(".".into()),
        ..Default::default()
    };
    let cmd = std::process::Command::from(gix_command::prepare("").with_context(ctx));
    assert_eq!(format!("{cmd:?}"), winfix(r#"GIT_DIR="." """#));
}

#[test]
fn worktree_dir_sets_env_only() {
    let ctx = Context {
        worktree_dir: Some(".".into()),
        ..Default::default()
    };
    let cmd = std::process::Command::from(gix_command::prepare("").with_context(ctx));
    assert_eq!(format!("{cmd:?}"), winfix(r#"GIT_WORK_TREE="." """#));
}

#[test]
fn no_replace_objects_sets_env_only() {
    for value in [false, true] {
        let expected = usize::from(value);
        let ctx = Context {
            no_replace_objects: Some(value),
            ..Default::default()
        };
        let cmd = std::process::Command::from(gix_command::prepare("").with_context(ctx));
        assert_eq!(
            format!("{cmd:?}"),
            winfix(format!(r#"GIT_NO_REPLACE_OBJECTS="{expected}" """#))
        );
    }
}

#[test]
fn ref_namespace_sets_env_only() {
    let ctx = Context {
        ref_namespace: Some("namespace".into()),
        ..Default::default()
    };
    let cmd = std::process::Command::from(gix_command::prepare("").with_context(ctx));
    assert_eq!(format!("{cmd:?}"), winfix(r#"GIT_NAMESPACE="namespace" """#));
}

#[test]
fn literal_pathspecs_sets_env_only() {
    for value in [false, true] {
        let expected = usize::from(value);
        let ctx = Context {
            literal_pathspecs: Some(value),
            ..Default::default()
        };
        let cmd = std::process::Command::from(gix_command::prepare("").with_context(ctx));
        assert_eq!(
            format!("{cmd:?}"),
            winfix(format!(r#"GIT_LITERAL_PATHSPECS="{expected}" """#))
        );
    }
}

#[test]
fn glob_pathspecs_sets_env_only() {
    for (value, expected) in [
        (false, r#"GIT_NOGLOB_PATHSPECS="1""#),
        (true, r#"GIT_GLOB_PATHSPECS="1""#),
    ] {
        let ctx = Context {
            glob_pathspecs: Some(value),
            ..Default::default()
        };
        let cmd = std::process::Command::from(gix_command::prepare("").with_context(ctx));
        assert_eq!(format!("{cmd:?}"), winfix(format!(r#"{expected} """#)));
    }
}

#[test]
fn icase_pathspecs_sets_env_only() {
    for value in [false, true] {
        let expected = usize::from(value);
        let ctx = Context {
            icase_pathspecs: Some(value),
            ..Default::default()
        };
        let cmd = std::process::Command::from(gix_command::prepare("").with_context(ctx));
        assert_eq!(
            format!("{cmd:?}"),
            winfix(format!(r#"GIT_ICASE_PATHSPECS="{expected}" """#))
        );
    }
}

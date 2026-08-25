use std::ffi::OsStr;

use gix::bstr::BString;
use gix_testtools::Env;
use serial_test::serial;

fn repository(overrides: impl IntoIterator<Item = impl Into<BString>>) -> gix_testtools::Result<gix::Repository> {
    let fixture = gix_testtools::scripted_fixture_read_only("make_config_repos.sh")?;
    let mut permissions = gix::open::Permissions::isolated();
    permissions.env.git_prefix = gix::sec::Permission::Allow;
    permissions.env.other = gix::sec::Permission::Allow;
    Ok(gix::open_opts(
        fixture.join("http-proxy-empty"),
        gix::open::Options::isolated()
            .permissions(permissions)
            .config_overrides(overrides),
    )?)
}

#[test]
#[serial]
fn follows_git_editor_precedence() -> gix_testtools::Result {
    let _env = Env::new()
        .set("TERM", "xterm")
        .set("GIT_EDITOR", ":")
        .set("VISUAL", "visual")
        .set("EDITOR", "editor");
    assert_eq!(
        repository(["core.editor=core"])?.editor().as_deref(),
        Some(OsStr::new(":")),
        "the selected editor is available without preparing a command"
    );
    let editor = repository(["core.editor=core"])?
        .editor_command()?
        .expect("GIT_EDITOR is configured");
    assert_eq!(editor.command, OsStr::new(":"));
    assert!(editor.use_shell, "the shell provides the colon builtin");

    let _env = Env::new().unset("GIT_EDITOR");
    assert_eq!(
        repository(["core.editor=core"])?
            .editor_command()?
            .map(|editor| editor.command),
        Some("core".into())
    );
    assert_eq!(
        repository(None::<BString>)?
            .editor_command()?
            .map(|editor| editor.command),
        Some("visual".into())
    );

    let _env = Env::new().unset("VISUAL");
    assert_eq!(
        repository(None::<BString>)?
            .editor_command()?
            .map(|editor| editor.command),
        Some("editor".into()),
        "EDITOR is used even for dumb terminals"
    );

    let _env = Env::new().unset("EDITOR");
    let editor = repository(None::<BString>)?
        .editor_command()?
        .expect("a capable terminal always has a default editor");
    let editor = editor.command;
    if cfg!(windows) {
        assert!(
            std::path::Path::new(&editor).is_absolute(),
            "Git for Windows' bundled vi implementation is selected without relying on PATH: {editor:?}"
        );
        assert!(
            std::path::Path::new(&editor).is_file(),
            "the selected Git for Windows editor exists: {editor:?}"
        );
    } else {
        assert_eq!(editor, OsStr::new("vi"), "vi is the fallback on non-Windows platforms");
    }
    Ok(())
}

#[test]
#[serial]
fn dumb_terminals_require_an_explicit_non_visual_editor() -> gix_testtools::Result {
    let _env = Env::new()
        .set("TERM", "dumb")
        .unset("GIT_EDITOR")
        .set("VISUAL", "visual")
        .unset("EDITOR");
    assert!(repository(None::<BString>)?.editor_command()?.is_none());

    let _env = Env::new().set("EDITOR", "editor");
    assert_eq!(
        repository(None::<BString>)?
            .editor_command()?
            .map(|editor| editor.command),
        Some("editor".into())
    );
    Ok(())
}

#[test]
#[serial]
fn generic_editor_environment_is_available_as_gitoxide_configuration() -> gix_testtools::Result {
    let _env = Env::new()
        .set("TERM", "dumb")
        .set("VISUAL", "visual-from-environment")
        .set("EDITOR", "editor-from-environment")
        .unset("GIT_EDITOR");
    let fixture = gix_testtools::scripted_fixture_read_only("make_config_repos.sh")?;
    let repo = gix::open_opts(
        fixture.join("http-proxy-empty"),
        gix::open::Options::isolated().config_overrides([
            "gitoxide.term=xterm",
            "gitoxide.visual=visual-from-config",
            "gitoxide.editor=editor-from-config",
        ]),
    )?;

    assert_eq!(
        repo.editor_command()?.map(|editor| editor.command),
        Some("visual-from-config".into()),
        "configuration is used without reading the denied environment"
    );
    {
        let _git_editor = Env::new().set("GIT_EDITOR", "git-editor");
        let mut permissions = gix::open::Permissions::isolated();
        permissions.env.other = gix::sec::Permission::Allow;
        let repo = gix::open_opts(
            fixture.join("http-proxy-empty"),
            gix::open::Options::isolated().permissions(permissions),
        )?;
        assert_eq!(
            repo.editor_command()?.map(|editor| editor.command),
            Some("editor-from-environment".into()),
            "other grants generic editor environment but not GIT_EDITOR"
        );
    }
    let mut permissions = gix::open::Permissions::isolated();
    permissions.env.git_prefix = gix::sec::Permission::Allow;
    let repo = gix::open_opts(
        fixture.join("http-proxy-empty"),
        gix::open::Options::isolated().permissions(permissions),
    )?;
    assert!(
        repo.editor_command()?.is_none(),
        "git_prefix permission doesn't grant access to generic editor environment"
    );
    Ok(())
}

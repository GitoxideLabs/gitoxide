use std::ffi::OsStr;

use gix::bstr::BString;
use gix_testtools::Env;
use serial_test::serial;

fn repository(overrides: impl IntoIterator<Item = impl Into<BString>>) -> gix_testtools::Result<gix::Repository> {
    let fixture = gix_testtools::scripted_fixture_read_only("make_config_repos.sh")?;
    let mut permissions = gix::open::Permissions::isolated();
    permissions.env.git_prefix = gix::sec::Permission::Allow;
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
        Some(OsStr::new(":"))
    );

    let _env = Env::new().unset("GIT_EDITOR");
    assert_eq!(
        repository(["core.editor=core"])?.editor().as_deref(),
        Some(OsStr::new("core"))
    );
    assert_eq!(
        repository(None::<BString>)?.editor().as_deref(),
        Some(OsStr::new("visual"))
    );

    let _env = Env::new().unset("VISUAL");
    assert_eq!(
        repository(None::<BString>)?.editor().as_deref(),
        Some(OsStr::new("editor")),
        "EDITOR is used even for dumb terminals"
    );

    let _env = Env::new().unset("EDITOR");
    assert_eq!(
        repository(None::<BString>)?.editor().as_deref(),
        Some(OsStr::new("vi")),
        "vi is the fallback when no editor is configured"
    );
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
    assert_eq!(repository(None::<BString>)?.editor(), None);

    let _env = Env::new().set("EDITOR", "editor");
    assert_eq!(
        repository(None::<BString>)?.editor().as_deref(),
        Some(OsStr::new("editor"))
    );
    Ok(())
}

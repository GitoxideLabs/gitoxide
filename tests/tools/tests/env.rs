use std::env;

use gix_testtools::Env;
use serial_test::serial;

// We rely on these not already existing, to test `Env` without using or rewriting it.
static VAR1: &str = "VAR_03FC4045_6043_4A61_9D15_852236CB632B";
static VAR2: &str = "VAR_8C135840_05DB_4F3A_BFDD_FC755EC35B89";
static VAR3: &str = "VAR_9B23A2BE_E20B_4670_93E2_3A6A8D47F274";

fn set_var(var: &str, value: &str) {
    // SAFETY: These tests are marked serial and isolate their environment
    // variables with unique names.
    unsafe { env::set_var(var, value) };
}

fn remove_var(var: &str) {
    // SAFETY: These tests are marked serial and isolate their environment
    // variables with unique names.
    unsafe { env::remove_var(var) };
}

struct TestEnv;

impl TestEnv {
    fn new() -> Self {
        assert_eq!(env::var_os(VAR1), None);
        assert_eq!(env::var_os(VAR2), None);
        assert_eq!(env::var_os(VAR3), None);
        Self
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        remove_var(VAR1);
        remove_var(VAR2);
        remove_var(VAR3);
    }
}

#[test]
#[serial]
fn nonoverlapping() {
    let _meta = TestEnv::new();
    set_var(VAR1, "old1");
    set_var(VAR2, "old2");
    {
        let _env = Env::new().set(VAR1, "new1").unset(VAR2).set(VAR3, "new3");
        assert_eq!(env::var_os(VAR1), Some("new1".into()));
        assert_eq!(env::var_os(VAR2), None);
        assert_eq!(env::var_os(VAR3), Some("new3".into()));
    }
    assert_eq!(env::var_os(VAR1), Some("old1".into()));
    assert_eq!(env::var_os(VAR2), Some("old2".into()));
    assert_eq!(env::var_os(VAR3), None);
}

#[test]
#[serial]
fn overlapping_reset() {
    let _meta = TestEnv::new();
    {
        let _env = Env::new().set(VAR1, "new1A").set(VAR1, "new1B");
        assert_eq!(env::var_os(VAR1), Some("new1B".into()));
    }
    assert_eq!(env::var_os(VAR1), None);
}

#[test]
#[serial]
fn overlapping_unset() {
    let _meta = TestEnv::new();
    set_var(VAR1, "old1");
    {
        let _env = Env::new().unset(VAR1).unset(VAR1);
        assert_eq!(env::var_os(VAR1), None);
    }
    assert_eq!(env::var_os(VAR1), Some("old1".into()));
}

#[test]
#[serial]
fn overlapping_combo() {
    let _meta = TestEnv::new();
    set_var(VAR1, "old1");
    set_var(VAR2, "old2");
    {
        let _env = Env::new()
            .set(VAR1, "new1A")
            .unset(VAR2)
            .set(VAR1, "new1B")
            .unset(VAR3)
            .set(VAR2, "new2")
            .set(VAR3, "new3")
            .unset(VAR1)
            .unset(VAR3);
        assert_eq!(env::var_os(VAR1), None);
        assert_eq!(env::var_os(VAR2), Some("new2".into()));
        assert_eq!(env::var_os(VAR3), None);
    }
    assert_eq!(env::var_os(VAR1), Some("old1".into()));
    assert_eq!(env::var_os(VAR2), Some("old2".into()));
    assert_eq!(env::var_os(VAR3), None);
}

mod isolate_git_environment {
    use std::{
        collections::BTreeMap,
        ffi::{OsStr, OsString},
        path::PathBuf,
        process::Command,
    };

    use super::*;

    type Environment = BTreeMap<OsString, OsString>;

    fn snapshot() -> Environment {
        env::vars_os().collect()
    }

    fn same_name(left: &OsStr, right: &OsStr) -> bool {
        if cfg!(windows) {
            left.as_encoded_bytes().eq_ignore_ascii_case(right.as_encoded_bytes())
        } else {
            left == right
        }
    }

    // Keep the test runner's environment intact even if the guard under test fails to restore it.
    struct RestoreEnvironment(Environment);

    impl RestoreEnvironment {
        fn new() -> Self {
            Self(snapshot())
        }
    }

    impl Drop for RestoreEnvironment {
        fn drop(&mut self) {
            let current = snapshot();
            // SAFETY: All tests using this fallback guard are serial, including during unwinding.
            unsafe {
                for name in current.keys().filter(|name| !self.0.contains_key(*name)) {
                    env::remove_var(name);
                }
                for (name, value) in &self.0 {
                    env::set_var(name, value);
                }
            }
        }
    }

    #[test]
    #[serial]
    fn policy_and_tracked_restoration() -> gix_testtools::Result {
        let _restore = RestoreEnvironment::new();
        let disposable = tempfile::tempdir()?;
        let unused = disposable.path().join("unused").to_string_lossy().into_owned();
        let _env = Env::new()
            .set(VAR1, "original")
            .set(VAR2, "")
            .unset(VAR3)
            .set("GIT_DIR", &unused)
            .set("GIT_CONFIG_COUNT", "1")
            .set("GIT_CONFIG_KEY_0", "core.hooksPath")
            .set("GIT_CONFIG_VALUE_0", &unused)
            .set("GIT_SSH", &unused)
            .set("GIT_FUTURE_TEST_ENV_VAR", "")
            .set("GIT_AUTHOR_NAME", "inherited author")
            .set("SSH_ASKPASS", &unused)
            .set("BASH_ENV", &unused)
            .set("ENV", &unused)
            .set("CDPATH", &unused)
            .set("MSYS", "winsymlinks:native")
            .set("XDG_CONFIG_HOME", &unused);
        let before = snapshot();
        let cwd = env::current_dir()?;
        let pid = std::process::id();

        // Inspect command overrides without spawning any subprocess, Git or otherwise.
        let mut command = Command::new(env::current_exe()?);
        gix_testtools::configure_git_environment(&mut command, disposable.path());
        let guard = gix_testtools::isolate_git_environment()?;
        let mut expected = before.clone();
        let mut actual = snapshot();
        for (name, value) in command.get_envs() {
            if name != "XDG_CONFIG_HOME" {
                assert_eq!(
                    env::var_os(name).as_deref(),
                    value,
                    "in-process isolation matches the command policy for {name:?}"
                );
            }
            // Windows overrides can use a different spelling, such as PATH instead of Path.
            expected.retain(|key, _| !same_name(key, name));
            actual.retain(|key, _| !same_name(key, name));
        }
        assert_eq!(
            actual, expected,
            "isolation leaves all variables outside the command policy unchanged"
        );
        let xdg =
            PathBuf::from(env::var_os("XDG_CONFIG_HOME").expect("isolation sets a private XDG configuration path"));
        assert_eq!(std::process::id(), pid, "isolation stays in the calling process");
        assert_eq!(
            env::current_dir()?,
            cwd,
            "isolation does not change the working directory"
        );
        assert!(
            xdg.is_absolute(),
            "the private configuration path is independent of the working directory"
        );
        // The XDG subdirectory may be created lazily, but its owning temporary directory must exist.
        let owned_directory = xdg
            .ancestors()
            .find(|path| path.is_dir())
            .expect("the guard keeps the temporary directory backing XDG alive")
            .to_owned();

        set_var(VAR1, "changed in scope");
        remove_var(VAR2);
        set_var(VAR3, "");
        set_var("GIT_NEW_TEST_ENV_VAR", "scope only");
        drop(guard);

        let mut expected = before;
        expected.insert(VAR1.into(), "changed in scope".into());
        expected.remove(OsStr::new(VAR2));
        expected.insert(VAR3.into(), "".into());
        expected.insert("GIT_NEW_TEST_ENV_VAR".into(), "scope only".into());
        assert_eq!(
            snapshot(),
            expected,
            "drop restores isolation's changes, including empty values, without undoing untracked changes"
        );
        assert!(
            !owned_directory.exists(),
            "dropping the guard removes its private temporary directory"
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn git_commands_do_not_inherit_later_environment_overrides() -> gix_testtools::Result {
        let _environment = gix_testtools::isolate_git_environment()?;
        let repo = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        gix_testtools::git(repo.path(), "init -q")?;
        gix_testtools::git(outside.path(), "init -q")?;
        gix_testtools::git(repo.path(), "config foo.bar intended")?;
        gix_testtools::git(outside.path(), "config foo.bar outside")?;
        let mut command = gix_testtools::git_command(repo.path());
        command.args(["config", "--local", "--get", "foo.bar"]);

        let _environment = _environment.set("GIT_DIR", outside.path().join(".git").to_string_lossy());
        let output = command.output()?;
        assert!(
            output.status.success(),
            "the already-configured command succeeds: {output:?}"
        );
        assert_eq!(
            output.stdout, b"intended\n",
            "Git cannot inherit repository selectors introduced after command setup"
        );

        command.env("GIT_DIR", outside.path().join(".git"));
        let output = command.output()?;
        assert!(
            output.status.success(),
            "an explicit command override succeeds: {output:?}"
        );
        assert_eq!(
            output.stdout, b"outside\n",
            "deliberate command-specific overrides remain supported"
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn nested_guards_restore_the_enclosing_scope() -> gix_testtools::Result {
        let _restore = RestoreEnvironment::new();
        let _env = Env::new().set(VAR1, "original").set(VAR2, "").unset(VAR3);
        let before = snapshot();
        let name = VAR1.to_owned();
        let outer = gix_testtools::isolate_git_environment()?
            .set(&name, "outer")
            .unset(VAR2)
            .set("GIT_FUTURE_TEST_ENV_VAR", "outer override");
        let outer_state = snapshot();
        let outer_xdg = env::var_os("XDG_CONFIG_HOME");
        {
            let _inner = gix_testtools::isolate_git_environment()?
                .unset(VAR1)
                .set(VAR2, "inner")
                .set(VAR3, "inner addition");
            assert_ne!(
                env::var_os("XDG_CONFIG_HOME"),
                outer_xdg,
                "nested guards own distinct configuration directories"
            );
            assert_eq!(
                env::var_os("GIT_FUTURE_TEST_ENV_VAR"),
                None,
                "inner isolation removes even outer Git overrides"
            );
        }
        assert_eq!(
            snapshot(),
            outer_state,
            "inner drop restores the outer scope, not the original environment"
        );
        drop(outer);
        assert_eq!(
            snapshot(),
            before,
            "outer drop restores the environment from before either guard"
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn early_and_error_returns_restore_the_environment() -> gix_testtools::Result {
        fn leave_scope(fail: bool) -> gix_testtools::Result {
            let _guard = gix_testtools::isolate_git_environment()?
                .set(VAR1, "changed before returning")
                .unset(VAR2)
                .set(VAR3, "added before returning");
            if !fail {
                return Ok(());
            }
            Err::<(), _>(std::io::Error::other("deliberate scoped error"))?;
            Ok(())
        }

        let _restore = RestoreEnvironment::new();
        let _env = Env::new().set(VAR1, "original").set(VAR2, "").unset(VAR3);
        let before = snapshot();
        leave_scope(false)?;
        assert_eq!(
            snapshot(),
            before,
            "an early successful return restores the entire environment"
        );
        let error = leave_scope(true).expect_err("the deliberate error leaves the guarded scope");
        assert_eq!(
            error.to_string(),
            "deliberate scoped error",
            "the error comes from inside the scope, not guard creation"
        );
        assert_eq!(snapshot(), before, "error propagation restores the entire environment");
        Ok(())
    }

    #[test]
    #[serial]
    fn panic_unwinding_restores_the_environment() -> gix_testtools::Result {
        let _restore = RestoreEnvironment::new();
        let _env = Env::new().set(VAR1, "original").set(VAR2, "").unset(VAR3);
        let before = snapshot();
        let mut entered = false;
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> gix_testtools::Result {
            let _guard = gix_testtools::isolate_git_environment()?
                .set(VAR1, "changed before panicking")
                .unset(VAR2)
                .set(VAR3, "added before panicking");
            entered = true;
            panic!("deliberate panic inside the guarded scope");
        }));
        assert!(entered, "guard creation succeeded before the deliberate panic");
        assert!(outcome.is_err(), "the deliberate panic unwound the guarded scope");
        assert_eq!(snapshot(), before, "panic unwinding restores the entire environment");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    #[serial]
    fn non_utf8_names_and_values_are_preserved() -> gix_testtools::Result {
        use std::os::unix::ffi::OsStringExt;

        let _restore = RestoreEnvironment::new();
        let name = OsString::from_vec(b"GIX_TEST_ENV_NAME_\xff".to_vec());
        let git_name = OsString::from_vec(b"git_TEST_ENV_NAME_\xfe".to_vec());
        let added_name = OsString::from_vec(b"GIX_TEST_ENV_ADDED_\xfd".to_vec());
        let value = OsString::from_vec(b"test value \xff".to_vec());
        // SAFETY: This serial test owns a fallback guard that restores even non-UTF8 entries.
        unsafe {
            env::set_var(&name, &value);
            env::set_var(&git_name, &value);
            env::set_var(VAR1, &value);
            env::remove_var(&added_name);
        }
        let before = snapshot();
        {
            let _guard = gix_testtools::isolate_git_environment()?;
            assert_eq!(
                env::var_os(&name),
                Some(value.clone()),
                "non-UTF8 non-Git entries survive isolation byte-for-byte"
            );
            assert_eq!(
                env::var_os(VAR1),
                Some(value.clone()),
                "UTF8 names may also hold non-UTF8 values"
            );
            assert_eq!(
                env::var_os(&git_name),
                None,
                "Git variable removal is case-insensitive even for non-UTF8 names"
            );
            // SAFETY: Access remains serialized and both guards are alive until these changes are restored.
            unsafe {
                env::remove_var(&name);
                env::set_var(&git_name, "scope override");
                env::set_var(&added_name, &value);
            }
            let _guard = _guard.set(VAR1, "scope override");
        }
        let mut expected = before;
        expected.remove(&name);
        expected.insert(added_name, value);
        assert_eq!(
            snapshot(),
            expected,
            "drop restores tracked OsString keys and values exactly, leaving unrelated non-UTF8 changes alone"
        );
        Ok(())
    }
}

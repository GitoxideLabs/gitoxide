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

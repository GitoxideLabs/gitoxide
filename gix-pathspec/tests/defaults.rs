use gix_pathspec::{Defaults, MagicSignature, SearchMode};
use serial_test::serial;

#[test]
#[serial]
fn literal_only_combines_with_icase() -> gix_testtools::Result {
    {
        let _env = gix_testtools::Env::new()
            .set("GIT_LITERAL_PATHSPECS", "true")
            .set("GIT_ICASE_PATHSPECS", "1")
            .set("GIT_NOGLOB_PATHSPECS", "yes");
        assert_eq!(
            Defaults::from_environment(&mut |n| std::env::var_os(n))?,
            Defaults {
                signature: MagicSignature::ICASE,
                search_mode: SearchMode::Literal,
                literal: true,
            }
        );
    }
    {
        let _env = gix_testtools::Env::new()
            .set("GIT_LITERAL_PATHSPECS", "true")
            .set("GIT_ICASE_PATHSPECS", "false")
            .set("GIT_GLOB_PATHSPECS", "yes");
        assert_eq!(
            Defaults::from_environment(&mut |n| std::env::var_os(n))?,
            Defaults {
                signature: MagicSignature::default(),
                search_mode: SearchMode::Literal,
                literal: true,
            }
        );
    }
    Ok(())
}
#[test]
#[serial]
fn nothing_is_set_then_it_is_like_the_default_impl() -> gix_testtools::Result {
    assert_eq!(
        Defaults::from_environment(&mut |n| std::env::var_os(n))?,
        Defaults::default()
    );
    Ok(())
}

#[test]
#[serial]
fn glob_and_noglob_cause_error() -> gix_testtools::Result {
    let _env = gix_testtools::Env::new()
        .set("GIT_GLOB_PATHSPECS", "1")
        .set("GIT_NOGLOB_PATHSPECS", "yes");
    assert_eq!(
        Defaults::from_environment(&mut |n| std::env::var_os(n))
            .unwrap_err()
            .to_string(),
        "Glob and no-glob settings are mutually exclusive"
    );

    Ok(())
}

#[test]
#[serial]
fn noglob_works() -> gix_testtools::Result {
    let _env = gix_testtools::Env::new()
        .set("GIT_GLOB_PATHSPECS", "0")
        .set("GIT_NOGLOB_PATHSPECS", "true");
    assert_eq!(
        Defaults::from_environment(&mut |n| std::env::var_os(n))?,
        Defaults {
            signature: MagicSignature::default(),
            search_mode: SearchMode::Literal,
            literal: false,
        },
        "it's OK to set only one of them, and only no-glob has an interesting, non-default effect"
    );
    Ok(())
}

#[test]
#[serial]
fn glob_works() -> gix_testtools::Result {
    let _env = gix_testtools::Env::new().set("GIT_GLOB_PATHSPECS", "yes");
    assert_eq!(
        Defaults::from_environment(&mut |n| std::env::var_os(n))?,
        Defaults {
            signature: MagicSignature::default(),
            search_mode: SearchMode::PathAwareGlob,
            literal: false,
        },
        "docs here are strange but they mean to use pathaware globbing when this variable is set"
    );
    Ok(())
}

use std::path::{Path, PathBuf};

use gix_config::file::{includes, init};

#[test]
fn simple() -> crate::Result {
    let (config, root) = config_with_includes("basic")?;
    compare_baseline(&config, "user.this", root.join("expected"));
    assert_eq!(config.string("user.that"), None);
    Ok(())
}

#[test]
fn inclusion_order() -> crate::Result {
    let (config, root) = config_with_includes("inclusion-order")?;
    for key in ["one", "two", "three"] {
        compare_baseline(&config, format!("user.{key}"), root.join(format!("expected.{key}")));
    }
    Ok(())
}

#[test]
fn globs() -> crate::Result {
    let (config, root) = config_with_includes("globs")?;
    for key in ["dss", "dse", "dsm", "ssm"] {
        compare_baseline(&config, format!("user.{key}"), root.join(format!("expected.{key}")));
    }
    assert_eq!(config.string("user.no"), None);
    Ok(())
}

#[test]
fn cycle_breaker() -> crate::Result {
    for name in ["cycle-breaker-direct", "cycle-breaker-indirect"] {
        let (_config, _root) = config_with_includes(name)?;
    }

    Ok(())
}

#[test]
fn no_cycle() -> crate::Result {
    let (config, root) = config_with_includes("no-cycle")?;
    compare_baseline(&config, "user.name", root.join("expected"));
    Ok(())
}

fn compare_baseline(config: &gix_config::File<'static>, key: impl AsRef<str>, expected: impl AsRef<Path>) {
    let expected = expected.as_ref();
    let key = key.as_ref();
    assert_eq!(
        config
            .string(key)
            .unwrap_or_else(|| panic!("key '{key} should be included"))
            .as_ref(),
        std::fs::read_to_string(expected)
            .unwrap_or_else(|err| panic!("Couldn't find '{expected:?}' for reading: {err}"))
            .trim(),
        "baseline with git should match: '{key}' != {expected:?}"
    );
}

fn config_with_includes(name: &str) -> crate::Result<(gix_config::File<'static>, PathBuf)> {
    let root = crate::scripted_fixture_read_only("hasconfig.sh")?.join(name);
    let options = init::Options {
        includes: includes::Options::follow(Default::default(), Default::default()),
        ..Default::default()
    };

    let config = gix_config::File::from_paths_metadata(
        Some(gix_config::file::Metadata::try_from_path(
            root.join("config"),
            gix_config::Source::Local,
        )?),
        options,
    )?
    .expect("non-empty");
    Ok((config, root))
}

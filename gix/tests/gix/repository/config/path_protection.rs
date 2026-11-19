use gix_testtools::{TestResult, tempfile};

#[test]
fn repository_sources_respect_the_section_filter() -> TestResult {
    use gix_sec::Trust;

    #[derive(Debug, Eq, PartialEq)]
    struct Protections {
        windows: bool,
        hfs: bool,
        ntfs: bool,
    }

    let fixture = gix_testtools::scripted_fixture_read_only("make_path_protection_repos.sh")?;
    let probe_dir = tempfile::tempdir_in(fixture.parent().expect("the fixture has a cache directory"))?;
    let symlinks_supported = gix_fs::Capabilities::probe_symlink(probe_dir.path())?;
    for source in ["local", "worktree"] {
        for value in ["false", "true", "invalid"] {
            for (options, use_config) in [
                (crate::restricted().with_trust(Trust::Reduced), false),
                (crate::restricted().with_trust(Trust::Full), true),
                (
                    crate::restricted()
                        .with_trust(Trust::Full)
                        .filter_config_section(|_| false),
                    false,
                ),
                (
                    crate::restricted()
                        .with_trust(Trust::Reduced)
                        .filter_config_section(|_| true),
                    true,
                ),
            ] {
                let repo = gix::open_opts(fixture.join(format!("{source}-{value}")), options)?;
                let checkout = repo.checkout_options(gix::worktree::stack::state::attributes::Source::IdMapping)?;
                let validate = checkout.validate;
                let configured = (use_config && value != "invalid").then_some(value == "true");
                let fallback = if repo.git_dir_trust() == Trust::Full && use_config {
                    gix_fs::Capabilities::default().symlink
                } else {
                    symlinks_supported
                };
                assert_eq!(
                    checkout.fs.symlink,
                    configured.unwrap_or(fallback),
                    "{source}-{value}: checkout uses the fallback when no eligible value supplies symlink support"
                );
                let expected = Protections {
                    windows: configured.unwrap_or(cfg!(windows)),
                    hfs: configured.unwrap_or(cfg!(target_os = "macos")),
                    ntfs: configured.unwrap_or(true),
                };
                assert_eq!(
                    Protections {
                        windows: validate.protect_windows,
                        hfs: validate.protect_hfs,
                        ntfs: validate.protect_ntfs,
                    },
                    expected,
                    "{source}-{value}: only eligible settings can replace platform defaults"
                );
                assert_eq!(
                    repo.refs.prohibit_windows_device_names, expected.windows,
                    "{source}-{value}: ref-store initialization must use the same section filter"
                );
                for (name, mode, rejected) in [
                    ("GIT~1", None, expected.ntfs),
                    (".git...:stream", None, expected.ntfs),
                    (".gi\u{200c}t", None, expected.hfs),
                    (
                        ".gi\u{200c}tmodules",
                        Some(gix_validate::path::component::Mode::Symlink),
                        expected.hfs,
                    ),
                    ("dir\\file", None, expected.windows),
                ] {
                    assert_eq!(
                        gix_validate::path::component(name.into(), mode, validate).is_err(),
                        rejected,
                        "{source}-{value}: path validation retains the selected protections for {name:?}"
                    );
                }
            }
        }
        for value in ["false", "true"] {
            let overrides = [
                format!("core.symlinks={value}"),
                format!("core.protectHFS={value}"),
                format!("core.protectNTFS={value}"),
                format!("gitoxide.core.protectWindows={value}"),
            ];
            let repo = gix::open_opts(
                fixture.join(format!("{source}-invalid")),
                crate::restricted()
                    .with_trust(Trust::Reduced)
                    .strict_config(true)
                    .config_overrides(overrides),
            )?;
            let checkout = repo.checkout_options(gix::worktree::stack::state::attributes::Source::IdMapping)?;
            let validate = checkout.validate;
            assert_eq!(
                checkout.fs.symlink,
                value == "true",
                "{source}: trusted overrides control symlink checkout"
            );
            assert_eq!(
                (validate.protect_windows, validate.protect_hfs, validate.protect_ntfs),
                (value == "true", value == "true", value == "true"),
                "{source}: trusted API overrides remain usable under reduced trust"
            );
            assert_eq!(
                repo.refs.prohibit_windows_device_names,
                value == "true",
                "{source}: trusted API overrides also configure the reference store"
            );
        }
    }
    Ok(())
}

#[test]
fn symlink_fallback_uses_the_effective_checkout() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_path_protection_repos.sh")?;
    let worktree = fixture.join("unavailable-checkout");
    let mut options = crate::restricted()
        .with_trust(gix::sec::Trust::Reduced)
        .system_config_path(worktree.join(".git/checkout.config"));
    options.permissions.config.system = true;
    let repo = gix::open_opts(worktree, options)?;
    assert_eq!(
        repo.workdir(),
        Some(repo.git_dir().join("checkout-file").as_path()),
        "protected configuration supplies the effective checkout"
    );
    assert!(
        !repo.filesystem_options()?.symlink,
        "filesystem options fall back to plain files where links cannot be created"
    );
    assert!(
        !repo
            .checkout_options(gix::worktree::stack::state::attributes::Source::IdMapping)?
            .fs
            .symlink,
        "checkout uses the same symlink fallback"
    );
    Ok(())
}

#[test]
fn safe_directory_promotion_updates_reference_protection_and_sharing() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_path_protection_repos.sh")?;
    let mut permissions = gix::open::Permissions::isolated();
    permissions.config.system = true;
    for source in ["local", "worktree"] {
        for value in ["false", "true"] {
            let repo = gix::open_opts(
                fixture.join(format!("{source}-{value}")),
                crate::restricted()
                    .with_trust(gix_sec::Trust::Reduced)
                    .permissions(permissions)
                    .system_config_path(fixture.join("trust-all-repositories.config")),
            )?;
            assert_eq!(
                repo.git_dir_trust(),
                gix_sec::Trust::Full,
                "the system safe.directory promotes trust"
            );
            assert_eq!(
                repo.refs.prohibit_windows_device_names,
                value == "true",
                "{source}: reference protection uses the final section trust"
            );
            assert_eq!(
                repo.refs.shared_repository_permissions,
                gix::fs::SharedPermissions::Group,
                "{source}: reference sharing uses the final section trust"
            );
        }
    }
    Ok(())
}

#[test]
fn invalid_values_are_checked_only_in_eligible_sections() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_path_protection_repos.sh")?;
    for source in ["local", "worktree"] {
        let repo = gix::open_opts(
            fixture.join(format!("{source}-invalid")),
            crate::restricted()
                .with_trust(gix_sec::Trust::Reduced)
                .strict_config(true),
        )?;
        repo.checkout_options(gix::worktree::stack::state::attributes::Source::IdMapping)?;
        for key in [
            "core.symlinks",
            "core.protectHFS",
            "core.protectNTFS",
            "gitoxide.core.protectWindows",
        ] {
            let err = gix::open_opts(
                fixture.join(format!("{source}-true")),
                crate::restricted()
                    .with_trust(gix_sec::Trust::Reduced)
                    .strict_config(true)
                    .config_overrides([format!("{key}=invalid")]),
            )
            .and_then(|repo| {
                repo.checkout_options(gix::worktree::stack::state::attributes::Source::IdMapping)
                    .map(|_| ())
            })
            .expect_err("an eligible malformed protection setting must fail in strict mode");
            assert!(
                err.is_validation(),
                "{source}: invalid {key} remains a validation error"
            );
        }
    }
    Ok(())
}

#[test]
fn reason_diagnostics() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/fixtures/unstable/invalid_reasons.rs");
}

#[test]
fn downstream_visibility() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = root
        .join("../target/gix-macros-unstable")
        .join(std::process::id().to_string());
    std::fs::create_dir_all(&fixture).expect("the disposable fixture directory can be created");
    std::fs::write(
        fixture.join("Cargo.toml"),
        format!(
            "\
[workspace]
[package]
name = \"unstable-macro-fixture\"
version = \"0.0.0\"
edition = \"2024\"
[features]
unstable = []
[dependencies]
gix-macros = {{ path = {root:?} }}
[lib]
path = {api:?}
[[bin]]
name = \"consumer\"
path = {consumer:?}
",
            api = root.join("tests/fixtures/unstable/api.rs"),
            consumer = root.join("tests/fixtures/unstable/consumer.rs"),
        ),
    )
    .expect("the fixture manifest can be written");

    let run = |command: &str, args: &[&str]| {
        std::process::Command::new(env!("CARGO"))
            .args([command, "--offline", "--manifest-path"])
            .arg(fixture.join("Cargo.toml"))
            .arg("--target-dir")
            .arg(root.join("../target/gix-macros-unstable/build"))
            .args(args)
            .output()
            .expect("Cargo can check the isolated downstream fixture")
    };
    for args in [&["--lib"][..], &["--bin", "consumer", "--features", "unstable"][..]] {
        let output = run("check", args);
        assert!(
            output.status.success(),
            "internal calls remain available without the feature, and downstream access works with it: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let output = run("check", &["--bin", "consumer"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "downstream unstable access must require the feature"
    );
    for diagnostic in [
        "module `plumbing` is private",
        "module `module` is private",
        "field `raw` of struct `Handle` is private",
        "method `raw` is private",
        "method `individual` is private",
        "`Handle` doesn't implement `std::fmt::Display`",
    ] {
        assert!(
            stderr.contains(diagnostic),
            "missing expected visibility diagnostic {diagnostic:?}: {stderr}"
        );
    }

    let docs = root.join("../target/gix-macros-unstable/build/doc/unstable_macro_fixture");
    for features in [&["--features", "unstable"][..], &[][..]] {
        let output = run("doc", &[&["--no-deps", "--lib"], features].concat());
        assert!(
            output.status.success(),
            "fixture rustdoc succeeds: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let handle = std::fs::read_to_string(docs.join("struct.Handle.html")).expect("Handle documentation exists");
        for reason in [
            "The field exposes plumbing state.",
            "These methods expose plumbing state.",
        ] {
            assert_eq!(
                handle.contains(reason),
                !features.is_empty(),
                "the reason is documented only with unstable enabled"
            );
        }
        if !features.is_empty() {
            let module = std::fs::read_to_string(docs.join("module/index.html")).expect("module documentation exists");
            assert!(
                module.contains("File-backed module documentation is preserved."),
                "original module documentation is retained"
            );
            assert!(
                module.contains("This file-backed module still exposes plumbing."),
                "the module's reason is documented"
            );
        }
    }
}

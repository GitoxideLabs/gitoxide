use crate::TestResult;
use gix_config::File;

#[test]
fn can_reconstruct_empty_config() {
    let config = r#"

    "#;
    assert_eq!(File::try_from(config).unwrap().to_string(), config);
}

#[test]
fn can_reconstruct_non_empty_config() {
    let config = r#"
        [user]
            email = code@eddie.sh
        [core]
            autocrlf = input
        [push]
            default = simple
        [commit]
            gpgsign = true
        [gpg]
            program = gpg
        [url "ssh://git@github.com/"]
            insteadOf = "github://"
        [url "ssh://git@git.eddie.sh/edward/"]
            insteadOf = "gitea://"
        [pull]
            ff = only
        [init]
            defaultBranch = master
    "#;

    assert_eq!(File::try_from(config).unwrap().to_string(), config);
}

#[test]
fn can_reconstruct_configs_with_implicits() {
    let config = r#"
        [user]
            email
            name
        [core]
            autocrlf
        [push]
            default
        [commit]
            gpgsign
    "#;

    assert_eq!(File::try_from(config).unwrap().to_string(), config);
}

#[test]
fn can_reconstruct_configs_without_whitespace_in_middle() {
    let config = r#"
        [core]
            autocrlf=input
        [push]
            default=simple
        [commit]
            gpgsign=true
        [pull]
            ff = only
        [init]
            defaultBranch = master
    "#;

    assert_eq!(File::try_from(config).unwrap().to_string(), config);
}

#[test]
fn equality_ignores_section_and_value_name_case_but_not_subsection_case() -> TestResult {
    let mixed_case = File::try_from("[Core]\nMixedCase = value\n[Remote \"Origin\"]\nURL = location\n")?;
    let equivalent = File::try_from("[core]\nmixedcase = value\n[remote \"Origin\"]\nurl = location\n")?;
    assert_eq!(mixed_case, equivalent, "section and value names are case-insensitive");

    let different_subsection = File::try_from("[core]\nmixedcase = value\n[remote \"origin\"]\nurl = location\n")?;
    assert_ne!(
        mixed_case, different_subsection,
        "quoted subsection names are case-sensitive"
    );
    Ok(())
}

#[test]
fn equality_uses_canonical_legacy_subsection_names() -> TestResult {
    let equivalent = File::try_from(
        r#"
        [remote "origin"]
            url = location
        "#,
    )?;
    let different = File::try_from(
        r#"
        [remote "Origin"]
            url = location
        "#,
    )?;
    for name in ["Origin", "ORIGIN", "origin"] {
        let legacy = File::try_from(
            format!(
                "
                [remote.{name}]
                    url = location
                "
            )
            .as_str(),
        )?;
        assert_eq!(legacy, equivalent, "equal lookup names identify the same subsection");
        assert_eq!(equivalent, legacy, "equality is symmetric across header styles");
        assert_ne!(
            legacy, different,
            "different effective names identify distinct subsections"
        );
    }
    Ok(())
}

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

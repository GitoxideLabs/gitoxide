use bstr::ByteVec;
use gix_config::file::{Metadata, init};

#[test]
fn empty_sections_roundtrip() {
    let input = r#"
        [a]
    [b]
        [c] 
        
            [d]
"#;

    let config = gix_config::File::try_from(input).unwrap();
    assert_eq!(config.to_bstring(), input);
}

#[test]
fn empty_sections_with_comments_roundtrip() {
    let input = r#"; pre-a
        [a] # side a
        ; post a  
    [b] ; side b
        [c] ; side c
        ; post c
            [d] # side d
"#;

    let mut config = gix_config::File::try_from(input).unwrap();
    let mut single_string = config.to_bstring();
    assert_eq!(single_string, input);
    assert_eq!(
        config.append(config.clone()).to_string(),
        {
            let clone = single_string.clone();
            single_string.push_str(&clone);
            single_string
        },
        "string-duplication is the same as data structure duplication"
    );
}

#[test]
fn complex_lossless_roundtrip() {
    let input = r#"
        [core]
            repositoryformatversion = 0
            filemode = true
            bare = false
            logallrefupdates = true
        
        [remote "origin"]
            url = git@github.com:GitoxideLabs/gitoxide.git
            fetch = +refs/heads/*:refs/remotes/origin/*

        [test]  # other comment
            other-quoted = "hello" ; comment
            implicit
            implicit-equal =
            implicit-equal-trailing-ws=     
            
        ; more comments
        # another one
            
        [test "sub-section \"special\" C:\\root"] ; section comment
            bool-explicit = false
            bool-implicit
            integer-no-prefix = 10 ; a value comment
            integer-prefix = 10g
            color = brightgreen red \
            bold
            other = hello world
            other-quoted = "hello world"
            location = ~/tmp
            location-quoted = "~/quoted"
            escaped = \n\thi\b
            escaped-quoted = "\n\thi\b"
            
        [alias]
            save = "!f() { \
               git status; \
               git add "-A"; \
               git commit -m \"$1\"; \
               git push -f; \
               git log -1;  \
            }; \
            f;  \
            unset f" ; here we go
    "#;
    let config = gix_config::File::try_from(input).unwrap();
    assert_eq!(config.to_bstring(), input);

    let lossy_config = gix_config::File::from_bytes_owned(
        &mut input.as_bytes().into(),
        Metadata::api(),
        init::Options {
            lossy: true,
            ..Default::default()
        },
    )
    .unwrap();

    let lossy_config: gix_config::File = lossy_config.to_string().parse().unwrap();
    assert_eq!(
        lossy_config, config,
        "Even lossy configuration serializes properly to be able to restore all values"
    );
}

mod to_filter {
    use bstr::ByteSlice;
    use gix_config::file::Metadata;

    use crate::file::cow_str;

    #[test]
    fn allows_only_selected_sections() -> crate::Result {
        let mut config = gix_config::File::new(Metadata::api());
        config.set_raw_value_by("a", None, "b", "c")?;

        let meta: Metadata = gix_config::Source::Local.into();
        config.set_meta(meta);

        config
            .new_section("a", cow_str("local"))?
            .push("b".try_into()?, Some("c".into()))
            .push("c".try_into()?, Some("d".into()));

        let meta: Metadata = gix_config::Source::User.into();
        config.set_meta(meta);

        config
            .new_section("a", cow_str("user"))?
            .push("b".try_into()?, Some("c".into()))
            .push("c".try_into()?, Some("d".into()));

        let mut buf = Vec::<u8>::new();
        config.write_to_filter(&mut buf, |s| s.meta().source == gix_config::Source::Local)?;
        let nl = config.detect_newline_style();
        assert_eq!(buf.to_str_lossy(), format!("[a \"local\"]{nl}\tb = c{nl}\tc = d{nl}"));

        Ok(())
    }
}

use crate::Result;
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
        config
            .append(config.clone())
            .expect("the small fixture fits into the backing buffer")
            .to_string(),
        {
            let clone = single_string.clone();
            single_string.push_str(&clone);
            single_string
        },
        "string-duplication is the same as data structure duplication"
    );
}

#[test]
fn decoded_subsection_names_keep_their_raw_spelling() {
    let input = r#"[remote "single \t \0"]
"#;
    let config = gix_config::File::try_from(input).expect("valid config");
    let section = config
        .section("remote", Some("single t 0".into()))
        .expect("the decoded subsection name is used for lookup");

    assert_eq!(
        section.header().subsection_name(),
        Some("single t 0".into()),
        "semantic access returns the decoded subsection name"
    );
    assert_eq!(
        config.to_bstring(),
        input,
        "serialization uses the original raw subsection spelling"
    );
}

#[test]
fn inserted_newlines_use_each_sections_newline_style() {
    let input = "[a]\nx=1\n[b]y=2\r\n";
    let config = gix_config::File::try_from(input).expect("valid config");

    assert_eq!(
        config.to_bstring(),
        "[a]\nx=1\n[b]\r\ny=2\r\n",
        "the newline inserted after a header matches the newline style of its section body"
    );
}

#[test]
fn crlf_after_a_comment_is_detected_and_used_for_insertions() -> Result {
    let input = "; root\r\n[core]\nkey=value\n";
    let mut config = gix_config::File::try_from(input)?;
    assert_eq!(
        config.detect_newline_style(),
        "\r\n",
        "the first complete newline determines the file's newline style even when it follows a comment"
    );

    config.new_section("new", None)?.push("key", Some("value".into()))?;
    assert_eq!(
        config.to_bstring(),
        "; root\r\n[core]\nkey=value\n\r\n[new]\r\n\tkey = value\r\n",
        "new section boundaries and contents must use the CRLF style detected after the root comment"
    );
    Ok(())
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
    use crate::Result;
    use bstr::ByteSlice;
    use gix_config::file::Metadata;

    #[test]
    fn allows_only_selected_sections() -> Result {
        let mut config = gix_config::File::new(Metadata::api());
        config.set_raw_value_by("a", None, "b", "c")?;

        let meta: Metadata = gix_config::Source::Local.into();
        config.set_meta(meta);

        config
            .new_section("a", "local")?
            .push("b", Some("c".into()))?
            .push("c", Some("d".into()))?;

        let meta: Metadata = gix_config::Source::User.into();
        config.set_meta(meta);

        config
            .new_section("a", "user")?
            .push("b", Some("c".into()))?
            .push("c", Some("d".into()))?;

        let mut buf = Vec::<u8>::new();
        config.write_to_filter(&mut buf, |s| s.meta().source == gix_config::Source::Local)?;
        let nl = config.detect_newline_style();
        assert_eq!(buf.to_str_lossy(), format!("[a \"local\"]{nl}\tb = c{nl}\tc = d{nl}"));

        Ok(())
    }
}

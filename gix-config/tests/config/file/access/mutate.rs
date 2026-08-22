mod new_section {
    #[test]
    fn accepts_a_borrowed_subsection_name() -> crate::Result {
        let mut file = gix_config::File::default();
        file.new_section("remote", "origin")?;
        file.new_section("branch", "main")?;

        let nl = if cfg!(windows) { "\r\n" } else { "\n" };
        assert_eq!(
            file.to_string(),
            format!("[remote \"origin\"]{nl}[branch \"main\"]{nl}"),
            "borrowed byte and string subsection names are owned by their new sections"
        );
        Ok(())
    }

    #[test]
    fn owned_sections_accept_a_borrowed_subsection_name() -> crate::Result {
        let section = gix_config::file::Section::new("remote", "origin", gix_config::file::Metadata::default())?;
        assert_eq!(section.to_ref().header().subsection_name(), Some("origin".into()));
        Ok(())
    }
}

mod remove_section {
    #[test]
    fn removal_of_all_sections_programmatically_with_sections_and_ids_by_name() {
        let mut file = gix_config::File::try_from("[core] \na = b\nb=c\n\n[core \"name\"]\nd = 1\ne = 2").unwrap();
        for id in file
            .sections_and_ids_by_name("core")
            .expect("2 sections present")
            .map(|(_, id)| id)
            .collect::<Vec<_>>()
        {
            _ = file.remove_section_by_id(id);
        }
        assert!(file.is_void());
        assert_eq!(file.sections().count(), 0);
    }

    #[test]
    fn removal_of_all_sections_programmatically_with_sections_and_ids() {
        let mut file = gix_config::File::try_from("[core] \na = b\nb=c\n\n[core \"name\"]\nd = 1\ne = 2").unwrap();
        for id in file.sections_and_ids().map(|(_, id)| id).collect::<Vec<_>>() {
            _ = file.remove_section_by_id(id);
        }
        assert!(file.is_void());
        assert_eq!(file.sections().count(), 0);
    }

    #[test]
    fn removal_is_complete_and_sections_can_be_read() {
        let mut file = gix_config::File::try_from("[core] \na = b\nb=c\n\n[core \"name\"]\nd = 1\ne = 2").unwrap();
        assert_eq!(file.sections().count(), 2);

        let removed = file.remove_section("core", None).expect("removed correct section");
        assert_eq!(removed.to_ref().header().name(), "core");
        assert_eq!(removed.to_ref().header().subsection_name(), None);
        assert_eq!(file.sections().count(), 1);
        assert!(file.remove_section("core", None).is_none(), "it's OK to try again");

        let removed = file.remove_section("core", "name").expect("found");
        assert_eq!(removed.to_ref().header().name(), "core");
        assert_eq!(removed.to_ref().header().subsection_name(), Some("name".into()));
        assert_eq!(file.sections().count(), 0);
        assert!(file.remove_section("core", "name").is_none());

        file.section_mut_or_create_new("core", None).expect("creation succeeds");
        file.section_mut_or_create_new("core", "name")
            .expect("creation succeeds");
    }

    #[test]
    fn removing_lookup_buckets_preserves_siblings_and_drops_the_final_name() -> crate::Result {
        let mut file = gix_config::File::try_from(
            "[core] key=plain\n\
             [core \"a\"] key=a\n\
             [core \"b\"] key=b\n",
        )?;

        file.remove_section("core", None).expect("plain section exists");
        let err = file.section("core", None).unwrap_err();
        assert!(err.downcast_any_ref::<gix_error::NotFoundError>().is_some());
        assert_eq!(
            err.to_string(),
            "The requested subsection does not exist",
            "the `core` section name still exists through its siblings, but its no-subsection bucket was removed"
        );
        assert_eq!(file.section("core", "a")?.value("key"), Some("a".into()));

        file.remove_section("core", "a").expect("first subsection exists");
        assert_eq!(file.section("core", "b")?.value("key"), Some("b".into()));

        file.remove_section("core", "b").expect("final subsection exists");
        let err = file.section("core", "b").unwrap_err();
        assert!(err.downcast_any_ref::<gix_error::NotFoundError>().is_some());
        assert_eq!(err.to_string(), "The requested section does not exist");
        Ok(())
    }

    #[test]
    fn removed_sections_can_be_mutated_and_reinserted() -> crate::Result {
        let mut file = gix_config::File::try_from("[core]\na = b\n")?;
        let mut section = file.remove_section("core", None).expect("section is present");
        let removed_id = section.to_ref().id();

        section.to_mut().set("detached", "changed")?;
        assert_eq!(section.to_ref().value("detached"), Some("changed".into()));

        let inserted_id = file.push_section(section)?.id();
        assert_ne!(inserted_id, removed_id, "reinsertion assigns a fresh section id");
        assert_eq!(file.section("core", None)?.value("detached"), Some("changed".into()));
        assert_eq!(file.string("core.detached"), Some("changed".into()));
        Ok(())
    }
}
mod remove_section_filter {
    #[test]
    fn removal_of_section_is_complete() {
        let mut file = gix_config::File::try_from("[core] \na = b\nb=c\n\n[core \"name\"]\nd = 1\ne = 2").unwrap();
        assert_eq!(file.sections().count(), 2);

        let removed = file
            .remove_section_filter("core", None, |_| true)
            .expect("removed correct section");
        assert_eq!(removed.to_ref().header().name(), "core");
        assert_eq!(removed.to_ref().header().subsection_name(), None);
        assert_eq!(file.sections().count(), 1);
        let removed = file.remove_section_filter("core", "name", |_| true).expect("found");
        assert_eq!(removed.to_ref().header().name(), "core");
        assert_eq!(removed.to_ref().header().subsection_name(), Some("name".into()));
        assert_eq!(file.sections().count(), 0);

        assert!(
            file.remove_section_filter("core", None, |_| true).is_none(),
            "it's OK to try again"
        );
        assert!(file.remove_section_filter("core", "name", |_| true).is_none());

        file.section_mut_or_create_new("core", None).expect("creation succeeds");
        file.section_mut_or_create_new("core", "name")
            .expect("creation succeeds");
    }
}

mod rename_section {

    #[test]
    fn section_renaming_validates_new_name() {
        let mut file = gix_config::File::try_from("[core] a = b").unwrap();
        let err = file.rename_section("core", None, "new_core", None).unwrap_err();
        assert!(err.downcast_any_ref::<gix_error::ValidationError>().is_some());
        assert_eq!(err.to_string(), "section names can only be ascii, '-': \"new_core\"");

        let err = file.rename_section("core", None, "new-core", "a\nb").unwrap_err();
        assert!(err.downcast_any_ref::<gix_error::ValidationError>().is_some());
        assert_eq!(
            err.to_string(),
            "sub-section names must not contain newlines or null bytes: \"a\\nb\""
        );
    }

    #[test]
    fn accepts_borrowed_new_subsection_names() -> crate::Result {
        let mut file = gix_config::File::try_from("[core] a = b")?;
        file.rename_section("core", None, "remote", "origin")?;
        assert_eq!(
            file.sections().next().expect("one section").header().subsection_name(),
            Some("origin".into())
        );

        let mut file = gix_config::File::try_from("[core] a = b")?;
        file.rename_section_filter("core", None, "branch", "main", |_| true)?;
        assert_eq!(
            file.sections().next().expect("one section").header().subsection_name(),
            Some("main".into())
        );
        Ok(())
    }

    #[test]
    fn all_matching_sections_are_renamed_and_target_collisions_are_preserved() -> crate::Result {
        let mut file = gix_config::File::try_from(
            "[branch \"source\"] key = one\n\
             [some \"gar\"] key = unrelated\n\
             [branch \"dest\"] key = existing\n\
             [branch \"source\"] key = two\n",
        )?;

        file.rename_section("branch", "source", "branch", "dest")?;

        insta::assert_snapshot!(file.to_string(), "all sections are renamed, just like what Git does", @r#"
        [branch "dest"]
         key = one
        [some "gar"]
         key = unrelated
        [branch "dest"]
         key = existing
        [branch "dest"]
         key = two
        "#);
        Ok(())
    }

    #[test]
    fn filter_renames_every_accepted_section() -> crate::Result {
        let mut file = gix_config::File::try_from(
            "[branch \"source\"] key = one\n\
             [branch \"source\"] key = two\n\
             [branch \"source\"] key = three\n",
        )?;
        let ids: Vec<_> = file
            .sections_and_ids_by_name("branch")
            .expect("branch sections exist")
            .map(|(_, id)| id)
            .collect();
        file.section_mut_by_id(ids[0])
            .expect("first section exists")
            .set_trust(gix_sec::Trust::Reduced);
        file.section_mut_by_id(ids[2])
            .expect("third section exists")
            .set_trust(gix_sec::Trust::Reduced);

        file.rename_section_filter("branch", "source", "branch", "dest", |meta| {
            meta.trust == gix_sec::Trust::Reduced
        })?;

        insta::assert_snapshot!(file.to_string(), "only the first and the last section were selected", @r#"
        [branch "dest"]
         key = one
        [branch "source"]
         key = two
        [branch "dest"]
         key = three
        "#);

        let prev = file.to_string();
        let err = file
            .rename_section_filter("branch", "source", "branch", "other", |_| false)
            .unwrap_err();
        assert!(err.downcast_any_ref::<gix_error::NotFoundError>().is_some());
        assert_eq!(
            err.to_string(),
            "The key does not exist in the requested section",
            "matching nothing causes an error"
        );
        assert_eq!(
            file.to_string(),
            prev,
            "rejecting every candidate leaves the source unchanged"
        );
        Ok(())
    }

    #[test]
    fn renaming_to_the_same_identity_updates_all_headers() -> crate::Result {
        let mut file = gix_config::File::try_from(
            "[branch.source] one = 1\n\
             [branch.source] two = 2\n",
        )?;
        file.rename_section("branch", "source", "branch", "source")?;
        insta::assert_snapshot!(file.to_string(), "we only ever write non-legacy headers", @r#"
        [branch "source"]
         one = 1
        [branch "source"]
         two = 2
        "#);
        Ok(())
    }

    #[test]
    fn an_empty_lookup_bucket_is_reported_as_missing() -> crate::Result {
        let mut file = gix_config::File::try_from("[core] key = value\n")?;
        file.remove_section("core", None).expect("section exists");
        let err = file.rename_section("core", None, "other", None).unwrap_err();
        assert!(err.downcast_any_ref::<gix_error::NotFoundError>().is_some());
        assert_eq!(err.to_string(), "The requested section does not exist");
        Ok(())
    }
}
mod set_meta {
    use gix_config::file;

    #[test]
    fn affects_newly_added_sections() -> crate::Result {
        let mut file = gix_config::File::default();
        let expected = &file::Metadata::api();
        assert_eq!(file.meta(), expected);

        {
            let section = file.new_section("new", None)?;
            assert_eq!(
                section.meta(),
                expected,
                "sections inherit the underlying files metadata"
            );
        }
        let meta = file::Metadata {
            path: None,
            source: gix_config::Source::Local,
            level: 0,
            trust: gix_sec::Trust::Reduced,
        };
        file.set_meta(meta.clone());
        let section = file.new_section("new", None)?;
        assert_eq!(section.meta(), &meta, "it picks up changes as well");
        Ok(())
    }
}

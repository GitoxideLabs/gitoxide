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
            file.remove_section_by_id(id);
        }
        assert!(file.is_void());
        assert_eq!(file.sections().count(), 0);
    }

    #[test]
    fn removal_of_all_sections_programmatically_with_sections_and_ids() {
        let mut file = gix_config::File::try_from("[core] \na = b\nb=c\n\n[core \"name\"]\nd = 1\ne = 2").unwrap();
        for id in file.sections_and_ids().map(|(_, id)| id).collect::<Vec<_>>() {
            file.remove_section_by_id(id);
        }
        assert!(file.is_void());
        assert_eq!(file.sections().count(), 0);
    }

    #[test]
    fn removal_is_complete_and_sections_can_be_read() {
        let mut file = gix_config::File::try_from("[core] \na = b\nb=c\n\n[core \"name\"]\nd = 1\ne = 2").unwrap();
        assert_eq!(file.sections().count(), 2);

        assert!(file.remove_section("core", None), "removed correct section");
        assert_eq!(file.sections().count(), 1);
        assert!(!file.remove_section("core", None), "it's OK to try again");

        assert!(file.remove_section("core", Some("name".into())), "found");
        assert_eq!(file.sections().count(), 0);
        assert!(!file.remove_section("core", Some("name".into())));

        file.section_mut_or_create_new("core", None).expect("creation succeeds");
        file.section_mut_or_create_new("core", Some("name".into()))
            .expect("creation succeeds");
    }
}
mod remove_section_filter {
    #[test]
    fn removal_of_section_is_complete() {
        let mut file = gix_config::File::try_from("[core] \na = b\nb=c\n\n[core \"name\"]\nd = 1\ne = 2").unwrap();
        assert_eq!(file.sections().count(), 2);

        assert!(
            file.remove_section_filter("core", None, |_| true),
            "removed correct section"
        );
        assert_eq!(file.sections().count(), 1);
        assert!(
            file.remove_section_filter("core", Some("name".into()), |_| true),
            "found"
        );
        assert_eq!(file.sections().count(), 0);

        assert!(
            !file.remove_section_filter("core", None, |_| true),
            "it's OK to try again"
        );
        assert!(!file.remove_section_filter("core", Some("name".into()), |_| true));

        file.section_mut_or_create_new("core", None).expect("creation succeeds");
        file.section_mut_or_create_new("core", Some("name".into()))
            .expect("creation succeeds");
    }
}

mod rename_section {
    use gix_config::{file::rename_section, parse::section};

    #[test]
    fn section_renaming_validates_new_name() {
        let mut file = gix_config::File::try_from("[core] a = b").unwrap();
        assert!(matches!(
            file.rename_section("core", None, "new_core", None),
            Err(rename_section::Error::Section(section::header::Error::InvalidName))
        ));

        assert!(matches!(
            file.rename_section("core", None, "new-core", Some("a\nb".into())),
            Err(rename_section::Error::Section(
                section::header::Error::InvalidSubSection
            ))
        ));
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

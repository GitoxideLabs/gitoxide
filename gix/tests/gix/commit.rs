#[cfg(feature = "revision")]
mod describe {
    use gix::commit::describe::SelectRef::{AllRefs, AllTags, AnnotatedTags};

    use crate::named_repo;

    #[cfg(feature = "status")]
    mod with_dirty_suffix {
        use gix::commit::describe::SelectRef;

        use crate::util::named_subrepo_opts;

        #[test]
        fn dirty_suffix_applies_automatically_if_dirty() -> crate::Result {
            let repo = named_subrepo_opts(
                "make_submodules.sh",
                "submodule-head-changed",
                gix::open::Options::isolated(),
            )?;

            let actual = repo
                .head_commit()?
                .describe()
                .names(SelectRef::AllRefs)
                .try_resolve()?
                .expect("resolution")
                .format_with_dirty_suffix("dirty".to_owned())?
                .to_string();
            assert_eq!(actual, "main-dirty");
            Ok(())
        }

        #[test]
        fn dirty_suffix_does_not_apply_if_not_dirty() -> crate::Result {
            let repo = named_subrepo_opts("make_submodules.sh", "module1", gix::open::Options::isolated())?;

            let actual = repo
                .head_commit()?
                .describe()
                .names(SelectRef::AllRefs)
                .try_resolve()?
                .expect("resolution")
                .format_with_dirty_suffix("dirty".to_owned())?
                .to_string();
            assert_eq!(actual, "main");
            Ok(())
        }
    }

    #[test]
    fn tags_are_sorted_by_date_and_lexicographically() -> crate::Result {
        let repo = named_repo("make_commit_describe_multiple_tags.sh")?;
        let mut describe = repo.head_commit()?.describe();
        for filter in &[AnnotatedTags, AllTags, AllRefs] {
            describe = describe.names(*filter);
            assert_eq!(describe.format()?.to_string(), "v4", "{filter:?}");
        }
        Ok(())
    }

    #[test]
    fn tags_are_sorted_by_priority() -> crate::Result {
        let repo = named_repo("make_commit_describe_multiple_tags.sh")?;
        let commit = repo.find_reference("refs/tags/v0")?.id().object()?.into_commit();
        let mut describe = commit.describe();
        for filter in &[AnnotatedTags, AllTags, AllRefs] {
            describe = describe.names(*filter);
            assert_eq!(describe.format()?.to_string(), "v1", "{filter:?}");
        }
        Ok(())
    }

    #[test]
    fn lightweight_tags_are_sorted_lexicographically() -> crate::Result {
        let repo = named_repo("make_commit_describe_multiple_tags.sh")?;
        let commit = repo.find_reference("refs/tags/l0")?.id().object()?.into_commit();
        let mut describe = commit.describe();
        for filter in &[AnnotatedTags, AllTags, AllRefs] {
            describe = describe.names(*filter);
            let expected = match filter {
                AnnotatedTags => None,
                _ => Some("l0"),
            };
            let actual = describe.try_format()?.map(|f| f.to_string());
            assert_eq!(actual.as_deref(), expected, "{filter:?}");
        }
        Ok(())
    }

    #[test]
    fn tags_preserve_the_full_git_timestamp_range() -> crate::Result {
        let repo = named_repo("make_commit_name_rev_edge_cases.sh")?;
        let mut describe = repo.head_commit()?.describe();
        for filter in &[AnnotatedTags, AllTags, AllRefs] {
            describe = describe.names(*filter);
            assert_eq!(describe.format()?.to_string(), "z", "{filter:?}");
        }
        Ok(())
    }
}

#[cfg(feature = "revision")]
mod name_rev {
    use gix::commit::name_rev::SelectRef::{AllRefs, AllTags};

    use crate::named_repo;

    #[test]
    fn default_uses_all_refs() -> crate::Result {
        let repo = named_repo("make_commit_describe_multiple_tags.sh")?;
        let actual = repo.head_commit()?.name_rev().format()?.to_string();
        assert!(
            actual.starts_with("tags/"),
            "default selection should consider tag refs before the branch name: {actual}"
        );
        Ok(())
    }

    #[test]
    fn tags_only_uses_tag_refs() -> crate::Result {
        let repo = named_repo("make_commit_describe_multiple_tags.sh")?;
        assert_eq!(
            repo.head_commit()?.name_rev().names(AllTags).format()?.to_string(),
            "tags/v2^0"
        );
        Ok(())
    }

    #[test]
    fn excluding_tags_allows_branch_names_to_win() -> crate::Result {
        let repo = named_repo("make_commit_describe_multiple_tags.sh")?;
        assert_eq!(
            repo.head_commit()?
                .name_rev()
                .exclude_ref("tags/*")
                .format()?
                .to_string(),
            "main"
        );
        Ok(())
    }

    #[test]
    fn include_patterns_limit_usable_refs() -> crate::Result {
        let repo = named_repo("make_commit_describe_multiple_tags.sh")?;
        assert_eq!(
            repo.head_commit()?
                .name_rev()
                .names(AllRefs)
                .include_ref("refs/tags/v4")
                .format()?
                .to_string(),
            "tags/v4^0"
        );
        assert_eq!(
            repo.head_commit()?
                .name_rev()
                .names(AllRefs)
                .include_ref("tags/v4")
                .format()?
                .to_string(),
            "v4^0"
        );
        Ok(())
    }

    #[test]
    fn fallback_to_id_if_no_refs_match() -> crate::Result {
        let repo = named_repo("make_commit_describe_multiple_tags.sh")?;
        let mut name_rev = repo.head_commit()?.name_rev().exclude_ref("refs/*");
        assert!(
            name_rev.try_format()?.is_none(),
            "no symbolic name is available after excluding all refs"
        );
        assert_eq!(name_rev.format()?.to_string().len(), 7);
        Ok(())
    }

    #[test]
    fn non_commit_refs_are_ignored() -> crate::Result {
        let repo = named_repo("make_commit_name_rev_edge_cases.sh")?;
        assert_eq!(
            repo.head_commit()?.name_rev().names(AllTags).format()?.to_string(),
            "tags/a^0"
        );
        Ok(())
    }

    #[test]
    fn older_tips_do_not_name_newer_targets() -> crate::Result {
        let repo = named_repo("make_commit_name_rev_cutoff.sh")?;
        let commit = repo
            .find_reference("refs/heads/skew-target")?
            .id()
            .object()?
            .into_commit();

        assert!(
            commit.name_rev().names(AllTags).try_format()?.is_none(),
            "the skewed child tag is older than the target cutoff and should be ignored"
        );
        assert_eq!(commit.name_rev().format()?.to_string(), "skew-target");
        Ok(())
    }
}

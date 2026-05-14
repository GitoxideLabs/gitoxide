mod reflog {
    mod packed {
        use gix_ref::file::ReferenceExt;

        use crate::file;

        #[test]
        fn iter() -> crate::Result {
            let store = file::store_with_packed_refs()?;
            let r = store.find("main")?;
            assert_eq!(r.log_iter(&store).all()?.expect("log exists").count(), 1);
            assert!(r.log_exists(&store), "it exists if its readable");
            Ok(())
        }

        #[test]
        fn iter_rev() -> crate::Result {
            let store = file::store_with_packed_refs()?;
            let r = store.find("main")?;
            assert_eq!(r.log_iter(&store).rev()?.expect("log exists").count(), 1);
            Ok(())
        }
    }

    mod loose {
        use crate::file;

        #[test]
        fn iter() -> crate::Result {
            let store = file::store()?;
            let r = store.find_loose("HEAD")?;
            let mut buf = Vec::new();
            assert_eq!(r.log_iter(&store, &mut buf)?.expect("log exists").count(), 1);
            assert!(r.log_exists(&store), "it exists if its readable");
            Ok(())
        }

        #[test]
        fn iter_rev() -> crate::Result {
            let store = file::store()?;
            let r = store.find_loose("HEAD")?;
            let mut buf = [0u8; 256];
            assert_eq!(r.log_iter_rev(&store, &mut buf)?.expect("log exists").count(), 1);
            Ok(())
        }
    }
}

mod peel {
    use gix_object::FindExt;
    use gix_ref::{Reference, file::ReferenceExt};

    use crate::{
        file,
        file::{EmptyCommit, store_with_packed_refs},
        hex_to_id,
    };

    #[test]
    fn one_level() -> crate::Result {
        let store = file::store()?;
        let r = store.find_loose("HEAD")?;
        assert_eq!(r.kind(), gix_ref::Kind::Symbolic, "there is something to peel");

        let nr = Reference::from(r).follow(&store).expect("exists").expect("no failure");
        assert!(
            matches!(nr.target.to_ref(), gix_ref::TargetRef::Object(_)),
            "iteration peels a single level"
        );
        assert!(nr.follow(&store).is_none(), "end of iteration");
        assert_eq!(
            nr.target.to_ref(),
            gix_ref::TargetRef::Object(&hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03")),
            "we still have the peeled target"
        );
        Ok(())
    }

    #[test]
    fn peel_with_packed_involvement() -> crate::Result {
        let store = store_with_packed_refs()?;
        let mut head: Reference = store.find_loose("HEAD")?.into();
        let expected = hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03");
        assert_eq!(head.peel_to_id(&store, &EmptyCommit)?, expected);
        assert_eq!(head.target.try_id().map(ToOwned::to_owned), Some(expected));

        let mut head = store.find("dt1")?;
        assert_eq!(head.peel_to_id(&store, &gix_object::find::Never)?, expected);
        assert_eq!(head.target.into_id(), expected);
        Ok(())
    }

    #[test]
    fn peel_one_level_with_pack() -> crate::Result {
        let store = store_with_packed_refs()?;

        let mut head = store.find("dt1")?;
        assert_eq!(
            head.target.try_id().map(ToOwned::to_owned),
            Some(hex_to_id("4c3f4cce493d7beb45012e478021b5f65295e5a3"))
        );
        assert_eq!(
            head.kind(),
            gix_ref::Kind::Object,
            "its peeled, but does have another step to peel to…"
        );
        let final_stop = hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03");
        assert_eq!(head.peeled, Some(final_stop), "…it knows its peeled object");

        assert_eq!(
            head.follow(&store).transpose()?,
            None,
            "but following doesn't do that, only real peeling does"
        );

        head.peel_to_id(&store, &EmptyCommit)?;
        assert_eq!(
            head.target.try_id().map(ToOwned::to_owned),
            Some(final_stop),
            "packed refs are always peeled (at least the ones we choose to read)"
        );
        assert_eq!(head.kind(), gix_ref::Kind::Object, "it's terminally peeled now");
        assert_eq!(
            head.follow(&store).transpose()?,
            None,
            "following doesn't change anything"
        );
        Ok(())
    }

    #[test]
    fn to_id_multi_hop() -> crate::Result {
        let store = file::store()?;
        let mut r: Reference = store.find_loose("multi-link")?.into();
        assert_eq!(r.kind(), gix_ref::Kind::Symbolic, "there is something to peel");

        let commit = hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03");
        assert_eq!(r.peel_to_id(&store, &EmptyCommit)?, commit);
        assert_eq!(r.name.as_bstr(), "refs/remotes/origin/multi-link-target3");

        let mut r: Reference = store.find_loose("dt1")?.into();
        assert_eq!(
            r.peel_to_id(&store, &EmptyCommit)?,
            hex_to_id("4c3f4cce493d7beb45012e478021b5f65295e5a3"),
            "points to a tag object without actual object lookup"
        );

        let odb = crate::file::odb_at(store.git_dir().join("objects"))?;
        let mut r: Reference = store.find_loose("dt1")?.into();
        assert_eq!(r.peel_to_id(&store, &odb)?, commit, "points to the commit with lookup");

        Ok(())
    }

    #[test]
    fn to_id_long_jump() -> crate::Result {
        for packed in [None, Some("packed")] {
            let store = file::store_at_with_args("make_multi_hop_ref.sh", packed)?;
            let odb = crate::file::odb_at(store.git_dir().join("objects"))?;
            let mut r: Reference = store.find("multi-hop")?;
            r.peel_to_id(&store, &odb)?;

            let commit_id = hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03");
            assert_eq!(r.peeled, Some(commit_id));

            let mut buf = Vec::new();
            let obj = odb.find(&commit_id, &mut buf)?;
            assert_eq!(obj.kind, gix_object::Kind::Commit, "always peeled to the first non-tag");

            let mut r: Reference = store.find("multi-hop")?;
            let tag_id = r.follow_to_object_packed(&store, store.cached_packed_buffer()?.as_ref().map(|p| &***p))?;
            let obj = odb.find(&tag_id, &mut buf)?;
            assert_eq!(obj.kind, gix_object::Kind::Tag, "the first direct object target");
            assert_eq!(
                obj.decode()?.into_tag().expect("tag").name,
                "dt2",
                "this is the first annotated tag, which points at dt1"
            );
            let mut r: Reference = store.find("multi-hop2")?;
            let other_tag_id =
                r.follow_to_object_packed(&store, store.cached_packed_buffer()?.as_ref().map(|p| &***p))?;
            assert_eq!(other_tag_id, tag_id, "it can follow with multiple hops as well");
        }
        Ok(())
    }

    #[test]
    fn to_id_cycle() -> crate::Result {
        let store = file::store()?;
        let mut r: Reference = store.find_loose("loop-a")?.into();
        assert_eq!(r.kind(), gix_ref::Kind::Symbolic, "there is something to peel");
        assert_eq!(r.name.as_bstr(), "refs/loop-a");

        assert!(matches!(
            r.peel_to_id(&store, &gix_object::find::Never).unwrap_err(),
            gix_ref::peel::to_id::Error::FollowToObject(gix_ref::peel::to_object::Error::Cycle { .. })
        ));
        assert_eq!(r.name.as_bstr(), "refs/loop-a", "the ref is not changed on error");

        let mut r: Reference = store.find_loose("loop-a")?.into();
        let err = r
            .follow_to_object_packed(&store, store.cached_packed_buffer()?.as_ref().map(|p| &***p))
            .unwrap_err();
        assert!(matches!(err, gix_ref::peel::to_object::Error::Cycle { .. }));
        Ok(())
    }
}

mod parse {
    mod invalid {
        use gix_ref::file::loose::Reference;

        macro_rules! mktest {
            ($name:ident, $input:literal, $err:literal) => {
                #[test]
                fn $name() {
                    use std::convert::TryInto;
                    let err = Reference::try_from_path(
                        "HEAD".try_into().expect("this is a valid name"),
                        $input,
                        gix_hash::Kind::Sha1,
                    )
                    .unwrap_err();
                    assert_eq!(err.to_string(), $err);
                }
            };
        }

        mktest!(hex_id, b"foobar", "\"foobar\" could not be parsed");
        mktest!(ref_tag, b"reff: hello", "\"reff: hello\" could not be parsed");
        mktest!(
            sha256_sized_id_for_sha1,
            b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n",
            "\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\\n\" could not be parsed"
        );
        mktest!(
            trailing_garbage_after_id,
            b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaextra",
            "\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaextra\" could not be parsed"
        );
    }
    mod valid {
        use gix_object::bstr::ByteSlice;
        use gix_ref::file::loose::Reference;

        use crate::sha1_hex_to_id;

        macro_rules! mktest {
            ($name:ident, $input:literal, $kind:path, $id:expr, $ref:expr) => {
                #[test]
                fn $name() {
                    use std::convert::TryInto;
                    let reference = Reference::try_from_path(
                        "HEAD".try_into().expect("valid static name"),
                        $input,
                        gix_hash::Kind::Sha1,
                    )
                    .unwrap();
                    assert_eq!(reference.kind(), $kind);
                    assert_eq!(reference.target.to_ref().try_id(), $id);
                    assert_eq!(
                        reference.target.to_ref().try_name().map(|n| n.as_bstr()),
                        $ref
                    );
                }
            };
        }

        mktest!(
            peeled,
            b"c5241b835b93af497cda80ce0dceb8f49800df1c\n",
            gix_ref::Kind::Object,
            Some(sha1_hex_to_id("c5241b835b93af497cda80ce0dceb8f49800df1c").as_ref()),
            None
        );

        mktest!(
            peeled_uppercase,
            b"C5241B835B93AF497CDA80CE0DCEB8F49800DF1C\n",
            gix_ref::Kind::Object,
            Some(sha1_hex_to_id("c5241b835b93af497cda80ce0dceb8f49800df1c").as_ref()),
            None
        );

        mktest!(
            symbolic,
            b"ref: refs/heads/main\n",
            gix_ref::Kind::Symbolic,
            None,
            Some(b"refs/heads/main".as_bstr())
        );

        mktest!(
            symbolic_more_than_one_space,
            b"ref:        refs/foobar\n",
            gix_ref::Kind::Symbolic,
            None,
            Some(b"refs/foobar".as_bstr())
        );

        #[test]
        fn peeled_sha256() {
            use std::convert::TryInto;

            let input = &b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"[..];
            let reference = Reference::try_from_path(
                "HEAD".try_into().expect("valid static name"),
                input,
                gix_hash::Kind::Sha256,
            )
            .unwrap();
            assert_eq!(reference.kind(), gix_ref::Kind::Object);
            let target_id = reference.target.to_ref().try_id().expect("non-symbolic").to_owned();
            assert_eq!(target_id.kind(), gix_hash::Kind::Sha256);
            assert_eq!(target_id, gix_hash::ObjectId::from_hex(input).unwrap());
        }
    }
}

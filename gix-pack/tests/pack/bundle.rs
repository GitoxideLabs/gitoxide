mod locate {
    use bstr::ByteSlice;
    use gix_object::Kind;
    use gix_odb::pack;

    use crate::{SMALL_PACK_INDEX, fixture_path, hex_to_id};

    fn locate<'a>(hex_id: &str, out: &'a mut Vec<u8>) -> gix_object::Data<'a> {
        let bundle = pack::Bundle::at(fixture_path(SMALL_PACK_INDEX), gix_hash::Kind::Sha1).expect("pack and idx");
        bundle
            .find(
                &hex_to_id(hex_id),
                out,
                &mut gix_zlib::Inflate::default(),
                &mut pack::cache::Never,
            )
            .expect("read success")
            .expect("id present")
            .0
    }

    mod locate_and_verify {
        use gix_odb::pack;

        use crate::{PACKS_AND_INDICES, fixture_path};

        #[test]
        fn all() -> gix_testtools::TestResult {
            for (index_path, data_path) in PACKS_AND_INDICES {
                // both paths are equivalent
                pack::Bundle::at(fixture_path(index_path), gix_hash::Kind::Sha1)?;
                let bundle = pack::Bundle::at(fixture_path(data_path), gix_hash::Kind::Sha1)?;

                let mut buf = Vec::new();
                for entry in bundle.index.iter() {
                    let entry = entry?;
                    let (obj, _location) = bundle
                        .find(
                            &entry.oid,
                            &mut buf,
                            &mut gix_zlib::Inflate::default(),
                            &mut pack::cache::Never,
                        )?
                        .expect("id present");
                    obj.verify_checksum(&entry.oid)?;
                }
            }
            Ok(())
        }
    }

    #[test]
    fn blob() -> gix_testtools::TestResult {
        let mut out = Vec::new();
        let obj = locate("bd46bb3f5bb4ca5431770c4fde0735fb89d382f3", &mut out);

        assert_eq!(
            obj.data.as_bstr(),
            b"GitPython is a python library used to interact with Git repositories.\n\nHi there\n".as_bstr()
        );
        assert_eq!(obj.kind, Kind::Blob);
        let object = obj.decode()?;
        assert_eq!(object.kind(), Kind::Blob);
        assert_eq!(object.as_blob().expect("blob").data, obj.data);
        Ok(())
    }

    #[test]
    fn tree() -> gix_testtools::TestResult {
        let mut out = Vec::new();
        let obj = locate("e90926b07092bccb7bf7da445fae6ffdfacf3eae", &mut out);

        assert_eq!(obj.kind, Kind::Tree);
        assert_eq!(obj.decode()?.kind(), Kind::Tree);
        Ok(())
    }

    #[test]
    fn commit() -> gix_testtools::TestResult {
        let mut out = Vec::new();
        let obj = locate("779c5451ba9fe210ffd1f55db202e55f51acecac", &mut out);

        assert_eq!(obj.kind, Kind::Commit);
        assert_eq!(obj.decode()?.kind(), Kind::Commit);
        Ok(())
    }
}

#[cfg(all(not(feature = "wasm"), feature = "streaming-input"))]
mod write_to_directory {
    use crate::Result;
    use std::{
        fs,
        io::{Cursor, Write},
        path::Path,
        sync::atomic::AtomicBool,
    };

    use gix_odb::pack;
    use gix_testtools::tempfile::TempDir;
    use gix_utils::progress;

    use crate::{SMALL_PACK, SMALL_PACK_INDEX, fixture_path};

    fn expected_outcome() -> Result<pack::bundle::write::Outcome> {
        Ok(pack::bundle::write::Outcome {
            index: pack::index::write::Outcome {
                index_version: pack::index::Version::V2,
                index_hash: gix_hash::ObjectId::from_hex(b"544a7204a55f6e9cacccf8f6e191ea8f83575de3")?,
                data_hash: gix_hash::ObjectId::from_hex(b"0f3ea84cd1bba10c2a03d736a460635082833e59")?,
                pack_size: fs::metadata(fixture_path(SMALL_PACK))?.len(),
                num_objects: 42,
            },
            pack_version: pack::data::Version::V2,
            index_path: None,
            data_path: None,
            keep_path: None,
            object_hash: gix_hash::Kind::Sha1,
        })
    }

    #[test]
    fn without_providing_one() -> gix_testtools::TestResult {
        let res = write_pack(None::<&Path>, SMALL_PACK)?;
        assert_eq!(res, expected_outcome()?);
        assert_eq!(
            res.index.index_hash,
            pack::index::File::at(fixture_path(SMALL_PACK_INDEX), gix_hash::Kind::Sha1)?.index_checksum()
        );
        assert!(res.to_bundle().is_none());
        Ok(())
    }

    #[test]
    fn given_a_directory() -> gix_testtools::TestResult {
        let dir = TempDir::new()?;
        let mut res = write_pack(Some(&dir), SMALL_PACK)?;
        let (index_path, data_path, keep_path) = (res.index_path.take(), res.data_path.take(), res.keep_path.take());
        assert_eq!(res, expected_outcome()?);
        let mut sorted_entries = fs::read_dir(&dir)?
            .filter_map(std::result::Result::ok)
            .collect::<Vec<_>>();
        sorted_entries.sort_by_key(fs::DirEntry::file_name);
        assert_eq!(
            sorted_entries.len(),
            3,
            "we want a pack and the corresponding index and the keep file"
        );

        let pack_hash = res.index.data_hash.to_hex();
        assert_eq!(file_name(&sorted_entries[0]), format!("pack-{pack_hash}.idx"));
        assert_eq!(Some(sorted_entries[0].path()), index_path);
        assert_eq!(file_name(&sorted_entries[1]), format!("pack-{pack_hash}.keep"));
        assert_eq!(Some(sorted_entries[1].path()), keep_path);
        assert_eq!(file_name(&sorted_entries[2]), format!("pack-{pack_hash}.pack"));
        assert_eq!(Some(sorted_entries[2].path()), data_path);

        res.index_path = index_path;
        assert!(res.to_bundle().transpose()?.is_some());
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn immutable_bundle_permissions_match_git_after_umask() -> gix_testtools::TestResult {
        use std::os::unix::fs::PermissionsExt;

        static SHOULD_INTERRUPT: AtomicBool = AtomicBool::new(false);
        for mask in [0o022, 0o077] {
            if !gix_testtools::run_with_umask(mask)? {
                continue;
            }
            for (setting, permissions) in [
                ("false", gix_fs::SharedPermissions::Umask),
                ("group", gix_fs::SharedPermissions::Group),
                ("all", gix_fs::SharedPermissions::Everybody),
                ("0640", gix_fs::SharedPermissions::Explicit(0o640)),
            ] {
                let git_pack_dir = gix_testtools::scripted_fixture_read_only_with_args(
                    "make_shared_repository_pack.sh",
                    [format!("{mask:04o}"), setting.into(), SMALL_PACK.into()],
                )?
                .join("git/objects/pack");
                let dir = TempDir::new()?;
                for eager in [false, true] {
                    let actual_dir = dir.path().join(format!("actual-{eager}"));
                    let options = pack::bundle::write::Options {
                        thread_limit: Some(1),
                        shared_repository_permissions: permissions,
                        ..Default::default()
                    };
                    let input = fs::read(fixture_path(SMALL_PACK))?;
                    let outcome = if eager {
                        pack::Bundle::write_to_directory_eagerly(
                            Box::new(Cursor::new(input)),
                            None,
                            Some(actual_dir.clone()),
                            &mut progress::Discard,
                            &SHOULD_INTERRUPT,
                            None::<gix_object::find::Never>,
                            gix_hash::Kind::Sha1,
                            options,
                        )?
                    } else {
                        pack::Bundle::write_to_directory(
                            &mut Cursor::new(input),
                            Some(&actual_dir),
                            &mut progress::Discard,
                            &SHOULD_INTERRUPT,
                            None::<gix_object::find::Never>,
                            gix_hash::Kind::Sha1,
                            options,
                        )?
                    };
                    let actual = outcome.data_path.as_ref().expect("the pack contains objects");
                    let expected = git_pack_dir.join(actual.file_name().expect("pack filename"));
                    assert_eq!(
                        fs::metadata(&actual_dir)?.permissions().mode() & 0o7777,
                        fs::metadata(&git_pack_dir)?.permissions().mode() & 0o7777,
                        "shared={setting}, umask={mask:o}, eager={eager}: newly created pack directories follow Git"
                    );
                    for extension in ["pack", "idx", "keep"] {
                        assert_eq!(
                            fs::metadata(actual.with_extension(extension))?.permissions().mode() & 0o7777,
                            fs::metadata(expected.with_extension(extension))?.permissions().mode() & 0o7777,
                            "shared={setting}, umask={mask:o}, eager={eager}: {extension} permissions follow Git"
                        );
                    }
                    assert!(
                        outcome.to_bundle().transpose()?.is_some(),
                        "the published pack and index can be opened"
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn an_existing_keep_marker_is_preserved_and_not_owned_by_the_receiver() -> Result {
        let dir = TempDir::new()?;
        let original = write_pack(Some(&dir), SMALL_PACK)?;
        let keep_path = original.keep_path.expect("new packs are protected");
        fs::remove_file(original.data_path.expect("nonempty pack"))?;
        fs::remove_file(original.index_path.expect("index was written"))?;
        fs::write(&keep_path, b"another operation")?;
        let received = write_pack(Some(&dir), SMALL_PACK)?;
        assert_eq!(
            received.keep_path, None,
            "the caller must not remove another operation's marker"
        );
        assert_eq!(
            fs::read(&keep_path)?,
            b"another operation",
            "existing keep markers are not truncated"
        );
        assert!(
            received.to_bundle().transpose()?.is_some(),
            "receiving the pack still succeeds"
        );
        Ok(())
    }

    #[test]
    fn a_prepared_pack_stays_protected_until_the_publication_guard_is_dropped() -> gix_testtools::TestResult {
        let dir = TempDir::new()?;
        let mut prepared = prepare_pack(dir.path())?;
        let expected = expected_outcome()?.index;
        assert_eq!(
            prepared.index(),
            &expected,
            "preparation indexes the same pack as direct writing"
        );
        prepared.with_bundle(|bundle| {
            assert_eq!(bundle.index.num_objects(), 42, "the staged index is readable");
            assert_eq!(
                bundle.pack.checksum(),
                expected.data_hash,
                "the staged pack is readable"
            );
            let entry = bundle.index.iter().next().expect("the staged index contains objects")?;
            let mut buffer = Vec::new();
            let (object, _) = bundle
                .find(
                    &entry.oid,
                    &mut buffer,
                    &mut gix_zlib::Inflate::default(),
                    &mut pack::cache::Never,
                )?
                .expect("the bundle can look up the object before publication");
            object.verify_checksum(&entry.oid)?;
            Ok(())
        })?;

        let pack_path = dir.path().join(format!("pack-{}.pack", expected.data_hash.to_hex()));
        let index_path = pack_path.with_extension("idx");
        let keep_path = pack_path.with_extension("keep");
        assert!(
            !pack_path.exists() && !index_path.exists() && !keep_path.exists(),
            "preparation stays outside the published pack namespace"
        );

        let published = prepared.publish()?;
        assert_eq!(
            published.data_path(),
            pack_path,
            "publication installs the pack under its content hash"
        );
        let bundle = pack::Bundle::at(&index_path, gix_hash::Kind::Sha1)?;
        assert_eq!(
            bundle.index.index_checksum(),
            expected.index_hash,
            "publication installs the prepared index"
        );
        drop(bundle);
        assert!(keep_path.is_file(), "the guard keeps the pack protected");
        drop(published);
        assert!(!keep_path.exists(), "dropping the guard releases the marker");
        assert!(
            pack_path.is_file() && index_path.is_file(),
            "releasing the marker leaves the published bundle in place"
        );
        Ok(())
    }

    #[test]
    fn a_prepared_pack_aborts_on_keep_contention() -> gix_testtools::TestResult {
        let dir = TempDir::new()?;
        let prepared = prepare_pack(dir.path())?;
        let data_path = dir
            .path()
            .join(format!("pack-{}.pack", prepared.index().data_hash.to_hex()));
        let keep_path = data_path.with_extension("keep");
        fs::write(&keep_path, b"another operation")?;

        let err = prepared
            .publish()
            .expect_err("publication requires its own keep marker");
        assert_eq!(
            err.downcast_any_ref::<std::io::Error>().map(std::io::Error::kind),
            Some(std::io::ErrorKind::AlreadyExists),
            "callers can detect contention"
        );
        assert_eq!(
            fs::read(&keep_path)?,
            b"another operation",
            "the other operation keeps its marker"
        );
        assert_eq!(
            fs::read_dir(dir.path())?.count(),
            1,
            "contention leaves only the original marker"
        );
        Ok(())
    }

    #[test]
    fn dropping_a_prepared_pack_discards_its_files() -> gix_testtools::TestResult {
        let dir = TempDir::new()?;
        let prepared = prepare_pack(dir.path())?;
        drop(prepared);
        assert_eq!(
            fs::read_dir(dir.path())?.count(),
            0,
            "cancellation discards the staged pack and index"
        );
        Ok(())
    }

    #[test]
    fn a_prepared_pack_can_be_retried_after_index_installation_fails() -> gix_testtools::TestResult {
        let dir = TempDir::new()?;
        let prepared = prepare_pack(dir.path())?;
        let data_path = dir
            .path()
            .join(format!("pack-{}.pack", prepared.index().data_hash.to_hex()));
        let index_path = data_path.with_extension("idx");
        let keep_path = data_path.with_extension("keep");
        fs::create_dir(&index_path)?;

        prepared
            .publish()
            .expect_err("a directory prevents installing the index");
        assert!(
            data_path.is_file(),
            "a pack installed before the failure remains available for retry"
        );
        assert!(
            !keep_path.exists(),
            "failure releases the marker acquired for publication"
        );
        assert_eq!(
            fs::read_dir(dir.path())?.count(),
            2,
            "failure discards the temporary files"
        );

        fs::remove_dir(&index_path)?;
        let published = prepare_pack(dir.path())?.publish()?;
        assert!(
            index_path.is_file() && keep_path.is_file(),
            "retry installs the missing index under a new marker"
        );
        drop(published);
        Ok(())
    }

    #[test]
    fn preparation_keeps_only_the_verified_pack_bytes() -> gix_testtools::TestResult {
        let bytes = fs::read(fixture_path(SMALL_PACK))?;
        let mut input = bytes.clone();
        input.extend_from_slice(b"read-ahead bytes");
        let dir = TempDir::new()?;
        let prepared = prepare_input(dir.path(), &mut Cursor::new(input))?.expect("the fixture pack contains objects");
        let published = prepared.publish()?;
        assert_eq!(
            fs::read(published.data_path())?,
            bytes,
            "publication ends at the verified trailer"
        );
        Ok(())
    }

    #[test]
    fn preparation_verifies_empty_packs() -> gix_testtools::TestResult {
        let dir = TempDir::new()?;
        let header = pack::data::header::encode(pack::data::Version::V2, 0);
        let mut hasher = gix_hash::hasher(gix_hash::Kind::Sha1);
        hasher.update(&header);
        let mut input = header.to_vec();
        input.extend_from_slice(hasher.try_finalize()?.as_slice());
        assert!(
            prepare_input(dir.path(), &mut Cursor::new(&input))?.is_none(),
            "a valid empty pack has nothing to publish"
        );
        assert!(
            prepare_input(dir.path(), &mut Cursor::new(header)).is_err(),
            "even an empty pack requires a checksum trailer"
        );
        assert_eq!(
            fs::read_dir(dir.path())?.count(),
            0,
            "empty and invalid packs leave no files"
        );
        Ok(())
    }

    fn prepare_pack(directory: &Path) -> gix_testtools::TestResult<pack::bundle::write::Prepared> {
        let mut input = std::io::BufReader::new(fs::File::open(fixture_path(SMALL_PACK))?);
        Ok(prepare_input(directory, &mut input)?.expect("the fixture pack contains objects"))
    }

    fn prepare_input(
        directory: &Path,
        input: &mut dyn std::io::BufRead,
    ) -> gix_error::Result<Option<pack::bundle::write::Prepared>> {
        pack::Bundle::prepare_to_directory(
            input,
            directory,
            &mut progress::Discard,
            &AtomicBool::new(false),
            None::<gix_object::find::Never>,
            gix_hash::Kind::Sha1,
            Default::default(),
        )
    }

    /// A forward reference is a `REF_DELTA` stored before the object named as its base.
    /// Unlike `OFS_DELTA`, its object ID can name an object at any position in the pack.
    ///
    /// Git normally writes bases first, but sends thin packs which omit bases the receiver
    /// already has. `index-pack --fix-thin` makes such packs self-contained by appending
    /// those bases, leaving the original deltas as forward references.
    #[test]
    fn in_pack_ref_deltas_with_forward_references() -> gix_testtools::TestResult {
        for object_hash in [gix_hash::Kind::Sha1, gix_hash::Kind::Sha256] {
            for objects in [
                &[b"A".as_slice(), b"B".as_slice()][..],
                &[b"A".as_slice(), b"B".as_slice(), b"C".as_slice()][..],
            ] {
                let pack_data = ref_delta_pack(object_hash, objects, pack::data::Version::V2)?;
                for lookup in [None, Some(gix_object::find::Never)] {
                    let dir = TempDir::new()?;
                    let mut input = Cursor::new(pack_data.clone());
                    let outcome = pack::Bundle::write_to_directory(
                        &mut input,
                        Some(dir.as_ref()),
                        &mut progress::Discard,
                        &AtomicBool::new(false),
                        lookup,
                        object_hash,
                        pack::bundle::write::Options {
                            thread_limit: None,
                            iteration_mode: pack::data::input::Mode::Verify,
                            index_version: pack::index::Version::V2,
                            compression: gix_zlib::Compression::BEST_SPEED,
                            shared_repository_permissions: gix_fs::SharedPermissions::Umask,
                        },
                    )?;
                    assert_eq!(
                        outcome.index.num_objects as usize,
                        objects.len(),
                        "all in-pack objects are indexed"
                    );

                    let bundle = outcome
                        .to_bundle()
                        .transpose()?
                        .expect("writing to a directory creates a bundle");
                    let mut buf = Vec::new();
                    for expected in objects {
                        let id = gix_object::compute_hash(object_hash, gix_object::Kind::Blob, expected)?;
                        let object = bundle
                            .find(
                                &id,
                                &mut buf,
                                &mut gix_zlib::Inflate::default(),
                                &mut pack::cache::Never,
                            )?
                            .expect("object is indexed")
                            .0;
                        assert_eq!(object.kind, gix_object::Kind::Blob);
                        assert_eq!(object.data, *expected, "the ref-delta is fully resolved");
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    fn version_3_with_thin_pack_lookup() -> gix_testtools::TestResult {
        let object_hash = gix_hash::Kind::Sha1;
        let pack_data = ref_delta_pack(
            object_hash,
            &[b"A".as_slice(), b"B".as_slice()],
            pack::data::Version::V3,
        )?;
        let outcome = pack::Bundle::write_to_directory(
            &mut Cursor::new(pack_data),
            Some(TempDir::new()?.as_ref()),
            &mut progress::Discard,
            &AtomicBool::new(false),
            Some(gix_object::find::Never),
            object_hash,
            pack::bundle::write::Options {
                iteration_mode: pack::data::input::Mode::Verify,
                ..Default::default()
            },
        )?;
        assert_eq!(
            outcome.pack_version,
            pack::data::Version::V3,
            "the rewritten pack preserves its supported input version"
        );
        Ok(())
    }

    #[test]
    fn unresolved_ref_delta_base_is_reported() -> gix_testtools::TestResult {
        let object_hash = gix_hash::Kind::Sha1;
        let base_id = object_hash.null();
        let delta = [0, 0];
        let mut pack_data = pack::data::header::encode(pack::data::Version::V2, 1).to_vec();
        pack::data::entry::Header::RefDelta { base_id }.write_to(delta.len() as u64, &mut pack_data)?;
        pack_data.extend(deflate(&delta)?);
        let mut hasher = gix_hash::hasher(object_hash);
        hasher.update(&pack_data);
        pack_data.extend_from_slice(hasher.try_finalize()?.as_slice());

        let err = pack::Bundle::write_to_directory(
            &mut Cursor::new(pack_data),
            None,
            &mut progress::Discard,
            &AtomicBool::new(false),
            None::<gix_object::find::Never>,
            object_hash,
            pack::bundle::write::Options {
                thread_limit: None,
                iteration_mode: pack::data::input::Mode::Verify,
                index_version: pack::index::Version::V2,
                compression: gix_zlib::Compression::BEST_SPEED,
                shared_repository_permissions: gix_fs::SharedPermissions::Umask,
            },
        )
        .expect_err("a ref-delta without an in-pack or external base cannot be indexed");
        assert!(err.is_not_found(), "an unresolved base is classified as not found");
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&(err), &[]), "the missing base id is retained in the error chain", @"The ref-delta base object Oid(1) could not be found");
        Ok(())
    }

    #[test]
    fn many_small_objects_can_be_indexed_and_traversed() -> gix_testtools::TestResult {
        let object_hash = gix_hash::Kind::Sha1;
        let num_objects = 5102u32;
        let mut pack_data = pack::data::header::encode(pack::data::Version::V2, num_objects).to_vec();
        for index in 0..num_objects {
            let blob = index.to_le_bytes();
            pack::data::entry::Header::Blob.write_to(blob.len() as u64, &mut pack_data)?;
            pack_data.extend(deflate(&blob)?);
        }
        let mut hasher = gix_hash::hasher(object_hash);
        hasher.update(&pack_data);
        pack_data.extend_from_slice(hasher.try_finalize()?.as_slice());

        let directory = TempDir::new()?;
        let outcome = pack::Bundle::write_to_directory(
            &mut Cursor::new(pack_data),
            Some(directory.path()),
            &mut progress::Discard,
            &AtomicBool::new(false),
            None::<gix_object::find::Never>,
            object_hash,
            pack::bundle::write::Options { ..Default::default() },
        )?;
        assert_eq!(outcome.index.num_objects, num_objects, "all small blobs are indexed");
        let bundle = outcome.to_bundle().expect("the index was written to the directory")?;
        for traversal in [
            pack::index::traverse::Algorithm::Lookup,
            pack::index::traverse::Algorithm::DeltaTreeLookup,
        ] {
            let outcome = bundle.index.traverse(
                &bundle.pack,
                &mut progress::Discard,
                &AtomicBool::new(false),
                |_, bytes, _, _| {
                    assert_eq!(bytes.len(), 4, "every decoded blob retains its contents");
                    Ok(())
                },
                pack::index::traverse::Options {
                    traversal,
                    ..Default::default()
                },
            )?;
            assert_eq!(
                outcome.statistics.num_blobs, num_objects,
                "both algorithms visit all blobs"
            );
        }
        Ok(())
    }

    fn file_name(entry: &fs::DirEntry) -> String {
        entry.path().file_name().unwrap().to_str().unwrap().to_owned()
    }

    fn write_pack(directory: Option<impl AsRef<Path>>, pack_file: &str) -> Result<pack::bundle::write::Outcome> {
        let pack_file = fs::File::open(fixture_path(pack_file))?;
        static SHOULD_INTERRUPT: AtomicBool = AtomicBool::new(false);
        Ok(pack::Bundle::write_to_directory_eagerly(
            Box::new(pack_file),
            None,
            directory,
            &mut progress::Discard,
            &SHOULD_INTERRUPT,
            None::<gix_object::find::Never>,
            gix_hash::Kind::Sha1,
            pack::bundle::write::Options {
                thread_limit: None,
                iteration_mode: pack::data::input::Mode::Verify,
                index_version: pack::index::Version::V2,
                compression: gix_zlib::Compression::BEST_SPEED,
                shared_repository_permissions: gix_fs::SharedPermissions::Umask,
            },
        )?)
    }

    /// Build a complete pack whose one-byte blobs form a forward `REF_DELTA` chain.
    /// `objects` lists the base first, but entries are written in reverse dependency order:
    ///
    /// ```text
    /// objects = [A, B, C]
    ///
    /// increasing pack offset ────────────────────────────────────────────────►
    /// [REF_DELTA base=oid(B), yields C] → [REF_DELTA base=oid(A), yields B] → [BLOB A]
    /// ```
    ///
    /// Each arrow points to the entry needed as the base, so every delta refers forward.
    /// With `[A, B]`, the first entry is omitted, leaving `B → A`.
    fn ref_delta_pack(
        object_hash: gix_hash::Kind,
        objects: &[&'static [u8]],
        version: pack::data::Version,
    ) -> Result<Vec<u8>> {
        let mut pack_data = pack::data::header::encode(version, objects.len() as u32).to_vec();
        for pair in objects.windows(2).rev() {
            let (base, resolved) = (pair[0], pair[1]);
            let base_id = gix_object::compute_hash(object_hash, gix_object::Kind::Blob, base)?;
            let delta = [1, 1, 1, resolved[0]];
            pack::data::entry::Header::RefDelta { base_id }.write_to(delta.len() as u64, &mut pack_data)?;
            pack_data.extend(deflate(&delta)?);
        }
        let base = objects[0];
        pack::data::entry::Header::Blob.write_to(base.len() as u64, &mut pack_data)?;
        pack_data.extend(deflate(base)?);

        let mut hasher = gix_hash::hasher(object_hash);
        hasher.update(&pack_data);
        pack_data.extend_from_slice(hasher.try_finalize()?.as_slice());
        Ok(pack_data)
    }

    fn deflate(input: &[u8]) -> Result<Vec<u8>> {
        let mut out = gix_zlib::stream::deflate::Write::new(Vec::new(), gix_zlib::Compression::BEST_SPEED);
        out.write_all(input)?;
        out.flush()?;
        Ok(out.into_inner())
    }
}

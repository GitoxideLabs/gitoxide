use std::sync::atomic::AtomicBool;

use gix_features::{parallel::InOrderIter, progress};
use gix_odb::{pack, pack::FindExt};
use gix_pack::data::{
    output,
    output::{count, entry},
};

use crate::pack::{
    data::output::{db, DbKind},
    hex_to_id,
};

#[test]
fn traversals() -> crate::Result {
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
    struct Count {
        trees: usize,
        commits: usize,
        blobs: usize,
        tags: usize,
        delta_ref: usize,
        delta_oid: usize,
    }
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
    struct ObjectCount {
        trees: usize,
        commits: usize,
        blobs: usize,
        tags: usize,
    }
    impl ObjectCount {
        fn total(&self) -> usize {
            self.tags + self.trees + self.commits + self.blobs
        }
        fn add(&mut self, kind: gix_object::Kind) {
            use gix_object::Kind::*;
            match kind {
                Tree => self.trees += 1,
                Commit => self.commits += 1,
                Blob => self.blobs += 1,
                Tag => self.tags += 1,
            }
        }
    }
    impl Count {
        fn total(&self) -> usize {
            self.tags + self.trees + self.commits + self.blobs + self.delta_ref + self.delta_oid
        }
        fn add(&mut self, kind: output::entry::Kind) {
            use gix_object::Kind::*;
            use output::entry::Kind::*;
            match kind {
                Base(Tree) => self.trees += 1,
                Base(Commit) => self.commits += 1,
                Base(Blob) => self.blobs += 1,
                Base(Tag) => self.tags += 1,
                DeltaRef { .. } => self.delta_ref += 1,
                DeltaOid { .. } => self.delta_oid += 1,
            }
        }
    }
    let whole_pack = Count {
        trees: 21,
        commits: 16,
        blobs: 288,
        tags: 1,
        delta_ref: 542,
        delta_oid: 0, // these are basically none-existing in non-legacy packs, but are used only in thin packs on the wire
    };
    let whole_pack_obj_count = ObjectCount {
        trees: 40,
        commits: 16,
        blobs: 811,
        tags: 1,
    };
    for db_kind in [
        DbKind::DeterministicGeneratedContent,
        DbKind::DeterministicGeneratedContentMultiIndex,
    ] {
        let db = db(db_kind)?;
        for (
            expansion_mode,
            expected_count,
            expected_obj_count,
            expected_counts_outcome,
            expected_entries_outcome,
            expected_pack_hash,
            expected_thin_pack_hash,
            take,
            allow_thin_pack,
        ) in [
            (
                count::objects::ObjectExpansion::AsIs,
                Count {
                    trees: 0,
                    commits: 15,
                    blobs: 0,
                    tags: 1,
                    delta_ref: 0,
                    delta_oid: 0,
                },
                ObjectCount {
                    trees: 0,
                    commits: 15,
                    blobs: 0,
                    tags: 1,
                },
                output::count::objects::Outcome {
                    input_objects: 16,
                    expanded_objects: 0,
                    decoded_objects: 16,
                    total_objects: 16,
                },
                output::entry::iter_from_counts::Outcome {
                    decoded_and_recompressed_objects: 0,
                    missing_objects: 0,
                    objects_copied_from_pack: 16,
                    ref_delta_objects: 0,
                },
                hex_to_id("b920bbb055e1efb9080592a409d3975738b6efb3"),
                None,
                None,
                false,
            ),
            (
                count::objects::ObjectExpansion::TreeAdditionsComparedToAncestor,
                Count {
                    trees: 3,
                    commits: 2, // todo: why more?
                    blobs: 19,
                    tags: 0,
                    delta_ref: 5,
                    delta_oid: 74,
                },
                ObjectCount {
                    trees: 5,
                    commits: 2, // todo: figure out why its more than expected
                    blobs: 96,
                    tags: 0,
                },
                output::count::objects::Outcome {
                    input_objects: 1,
                    expanded_objects: 102,
                    decoded_objects: 10,
                    total_objects: 103,
                },
                output::entry::iter_from_counts::Outcome {
                    decoded_and_recompressed_objects: 0,
                    missing_objects: 0,
                    objects_copied_from_pack: 103,
                    ref_delta_objects: 74,
                },
                hex_to_id("25114bd8820b393c402cd53ad8ec7f6a84bb0633"),
                Some(hex_to_id("29ab9797aff1ca826afb699680356695d19c5acb")),
                Some(1),
                true,
            ),
            (
                count::objects::ObjectExpansion::TreeAdditionsComparedToAncestor,
                Count {
                    trees: 5,
                    commits: 2, // todo: why more?
                    blobs: 91,
                    tags: 0,
                    delta_ref: 5,
                    delta_oid: 0,
                },
                ObjectCount {
                    trees: 5,
                    commits: 2, // todo: figure out why its more than expected
                    blobs: 96,
                    tags: 0,
                },
                output::count::objects::Outcome {
                    input_objects: 1,
                    expanded_objects: 102,
                    decoded_objects: 10,
                    total_objects: 103,
                },
                output::entry::iter_from_counts::Outcome {
                    decoded_and_recompressed_objects: 74,
                    missing_objects: 0,
                    objects_copied_from_pack: 29,
                    ref_delta_objects: 0,
                },
                hex_to_id("d83d42128e40957c5174920189a0390b5a70f446"),
                None,
                Some(1),
                false,
            ),
            (
                count::objects::ObjectExpansion::TreeContents,
                whole_pack,
                whole_pack_obj_count,
                output::count::objects::Outcome {
                    input_objects: 16,
                    expanded_objects: 852,
                    decoded_objects: 57,
                    total_objects: 868,
                },
                output::entry::iter_from_counts::Outcome {
                    decoded_and_recompressed_objects: 0,
                    missing_objects: 0,
                    objects_copied_from_pack: 868,
                    ref_delta_objects: 0,
                },
                hex_to_id("542ad1d1c7c762ea4e36907570ff9e4b5b7dde1b"),
                None,
                None,
                false,
            ),
            (
                count::objects::ObjectExpansion::TreeAdditionsComparedToAncestor,
                whole_pack,
                whole_pack_obj_count,
                output::count::objects::Outcome {
                    input_objects: 16,
                    expanded_objects: 866,
                    decoded_objects: 74,
                    total_objects: 868,
                },
                output::entry::iter_from_counts::Outcome {
                    decoded_and_recompressed_objects: 0,
                    missing_objects: 0,
                    objects_copied_from_pack: 868,
                    ref_delta_objects: 0,
                },
                hex_to_id("542ad1d1c7c762ea4e36907570ff9e4b5b7dde1b"),
                None,
                None,
                false,
            ),
        ]
        .iter()
        .copied()
        {
            let head = hex_to_id("dfcb5e39ac6eb30179808bbab721e8a28ce1b52e");
            let mut commits = gix_traverse::commit::Simple::new(Some(head), db.clone())
                .map(Result::unwrap)
                .map(|c| c.id)
                .collect::<Vec<_>>();
            if let Some(take) = take {
                commits.resize(take, gix_hash::Kind::Sha1.null());
            }

            let deterministic_count_needs_single_thread = Some(1);
            let (counts, stats) = output::count::objects(
                db.clone(),
                Box::new(
                    commits
                        .into_iter()
                        .chain(std::iter::once(hex_to_id(if take.is_some() {
                            "0000000000000000000000000000000000000000"
                        } else {
                            "e3fb53cbb4c346d48732a24f09cf445e49bc63d6"
                        })))
                        .filter(|o| !o.is_null())
                        .map(Ok),
                ),
                &progress::Discard,
                &AtomicBool::new(false),
                count::objects::Options {
                    input_object_expansion: expansion_mode,
                    thread_limit: deterministic_count_needs_single_thread,
                    ..Default::default()
                },
            )?;
            let actual_count = counts.iter().fold(ObjectCount::default(), |mut c, e| {
                let mut buf = Vec::new();
                if let Ok((obj, _location)) = db.find(&e.id, &mut buf) {
                    c.add(obj.kind);
                }
                c
            });
            assert_eq!(actual_count, expected_obj_count);
            let counts_len = counts.len();
            assert_eq!(counts_len, expected_obj_count.total());

            assert_eq!(stats, expected_counts_outcome);
            assert_eq!(stats.total_objects, expected_obj_count.total());

            let mut entries_iter = output::entry::iter_from_counts(
                counts,
                db.clone(),
                Box::new(progress::Discard),
                output::entry::iter_from_counts::Options {
                    allow_thin_pack,
                    ..Default::default()
                },
            );
            let entries: Vec<_> = InOrderIter::from(entries_iter.by_ref())
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .collect();
            let actual_count = entries.iter().fold(Count::default(), |mut c, e| {
                c.add(e.kind);
                c
            });
            assert_eq!(actual_count, expected_count);
            assert_eq!(counts_len, expected_count.total());
            let stats = entries_iter.finalize_boxed()?;
            assert_eq!(stats, expected_entries_outcome);

            assert_eq!(
                expected_obj_count.total(),
                expected_count.total(),
                "two different ways of counting, still the same in the end"
            );

            write_and_verify(db.clone(), entries, expected_pack_hash, expected_thin_pack_hash)?;
        }
    }

    Ok(())
}

#[test]
fn empty_pack_is_allowed() {
    assert_eq!(
        write_and_verify(
            db(DbKind::DeterministicGeneratedContent).unwrap(),
            vec![],
            hex_to_id("029d08823bd8a8eab510ad6ac75c823cfd3ed31e"),
            None,
        )
        .unwrap_err()
        .to_string(),
        "pack data directory should be set",
        "empty packs are not actually written as they would be useless"
    );
}

fn write_and_verify(
    db: gix_odb::HandleArc,
    entries: Vec<output::Entry>,
    _expected_pack_hash: gix_hash::ObjectId,
    _expected_thin_pack_hash: Option<gix_hash::ObjectId>,
) -> crate::Result {
    let tmp_dir = gix_testtools::tempfile::TempDir::new()?;
    let pack_file_path = tmp_dir.path().join("new.pack");
    let mut pack_file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&pack_file_path)?;
    let (num_written_bytes, pack_hash) = {
        let num_entries = entries.len();
        let mut pack_writer = output::bytes::FromEntriesIter::new(
            std::iter::once(Ok::<_, entry::iter_from_counts::Error>(entries)),
            &mut pack_file,
            num_entries as u32,
            pack::data::Version::V2,
            gix_hash::Kind::Sha1,
        );
        let mut n = pack_writer.next().expect("one entries bundle was written")?;
        n += pack_writer.next().expect("the trailer was written")?;
        assert!(
            pack_writer.next().is_none(),
            "there is nothing more to iterate this time"
        );
        // verify we can still get the original parts back
        let hash = pack_writer.digest().expect("digest is available when iterator is done");
        let _ = pack_writer.input;
        let _ = pack_writer.into_write();
        (n, hash)
    };
    assert_eq!(
        num_written_bytes,
        pack_file.metadata()?.len(),
        "it reports the correct amount of written bytes"
    );
    let pack = pack::data::File::at(&pack_file_path, gix_hash::Kind::Sha1)?;
    let should_interrupt = AtomicBool::new(false);
    let hash = pack.verify_checksum(&mut progress::Discard, &should_interrupt)?;
    assert_eq!(
        hash, pack_hash,
        "the trailer of the pack matches the actually written trailer"
    );

    // TODO: figure out why these hashes change, also depending on the machine, even though they are indeed stable.
    // assert_eq!(hash, expected_pack_hash, "pack hashes are stable if the input is");

    // Re-generate the index from the pack for validation.
    let object_hash = gix_hash::Kind::Sha1; // TODO: parameterize this
    let bundle = pack::Bundle::at(
        pack::Bundle::write_to_directory(
            &mut std::io::BufReader::new(std::fs::File::open(pack_file_path)?),
            Some(tmp_dir.path()),
            &mut progress::Discard,
            &should_interrupt,
            Some(&db),
            pack::bundle::write::Options::default(),
        )?
        .data_path
        .ok_or("pack data directory should be set")?,
        object_hash,
    )?;
    // TODO: figure out why these hashes change, also depending on the machine, even though they are indeed stable.
    // if let Some(thin_pack_checksum) = expected_thin_pack_hash {
    //     let actual_checksum = bundle.pack.verify_checksum(progress::Discard, &should_interrupt)?;
    //     assert_eq!(
    //         actual_checksum, thin_pack_checksum,
    //         "the thin pack is written reproducibly and checksums pan out"
    //     );
    // }

    bundle.verify_integrity(
        &mut progress::Discard,
        &should_interrupt,
        gix_pack::index::verify::integrity::Options {
            verify_mode: pack::index::verify::Mode::HashCrc32DecodeEncode,
            traversal: pack::index::traverse::Algorithm::Lookup,
            make_pack_lookup_cache: || pack::cache::Never,
            thread_limit: None,
        },
    )?;

    Ok(())
}

#[cfg(feature = "all-features")]
#[test]
fn customized_delta_topo() -> crate::Result {
    use gix_pack::data::output::entry::iter_from_counts::{Mode, Options};

    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
    struct Count {
        trees: usize,
        commits: usize,
        blobs: usize,
        tags: usize,
        delta_ref: usize,
        delta_oid: usize,
    }
    impl Count {
        fn add(&mut self, kind: output::entry::Kind) {
            use gix_object::Kind::*;
            use output::entry::Kind::*;
            match kind {
                Base(Tree) => self.trees += 1,
                Base(Commit) => self.commits += 1,
                Base(Blob) => self.blobs += 1,
                Base(Tag) => self.tags += 1,
                DeltaRef { .. } => self.delta_ref += 1,
                DeltaOid { .. } => self.delta_oid += 1,
            }
        }
    }

    for db_kind in [
        DbKind::DeterministicGeneratedContent,
        DbKind::DeterministicGeneratedContentMultiIndex,
    ] {
        let db = db(db_kind)?;

        // Get objects for testing
        // TODO: delta chain may not stable
        let objects: Vec<_> = vec![
            hex_to_id("a63e479f22985d08b5debd6567e15999123d25a4"), // base
            hex_to_id("d1ff3f36411c6eead64400062a7c8e30886b94ff"), // delta @ 1
            hex_to_id("37fbc9660088c6afad4b48169e80fe59670190d1"), // delta @ 2
            hex_to_id("dc2da8bbf4d82a654b35a2a43c0d714d4d7afbf9"), // delta @ 3
        ];

        // Count objects
        let (counts, _) = output::count::objects(
            db.clone(),
            Box::new(objects.clone().into_iter().map(Ok)),
            &progress::Discard,
            &AtomicBool::new(false),
            count::objects::Options {
                input_object_expansion: count::objects::ObjectExpansion::AsIs,
                thread_limit: Some(1),
                ..Default::default()
            },
        )?;

        // Empty topo: every object is base
        {
            let topo = std::collections::HashMap::new();

            let mut entries_iter = output::entry::iter_from_counts::customized::iter_from_counts_with_topo(
                counts,
                db.clone(),
                Box::new(progress::Discard),
                topo,
                1024 * 1024, // 1MB cache
                Options {
                    mode: Mode::Customized,
                    ..Default::default()
                },
            );

            let entries: Vec<_> = InOrderIter::from(entries_iter.by_ref())
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .collect();

            let actual_count = entries.iter().fold(Count::default(), |mut c, e| {
                c.add(e.kind);
                c
            });

            // All should be base objects since topo is empty
            assert!(actual_count.delta_ref == 0 && actual_count.delta_oid == 0);
        }

        // Test with non-empty topo
        {
            let objects = objects.to_owned();
            let topo_with_deltas = {
                let mut m = std::collections::HashMap::new();
                m.insert(objects[3], objects[2]);
                m.insert(objects[1], objects[2]);
                m
            };

            let (counts2, _) = output::count::objects(
                db.clone(),
                Box::new(objects.to_owned().into_iter().map(Ok)),
                &progress::Discard,
                &AtomicBool::new(false),
                count::objects::Options {
                    input_object_expansion: count::objects::ObjectExpansion::AsIs,
                    thread_limit: Some(1),
                    ..Default::default()
                },
            )?;

            // Test reuse delta
            {
                let entries_iter2 = output::entry::iter_from_counts::customized::iter_from_counts_with_topo(
                    counts2.clone(),
                    db.clone(),
                    Box::new(progress::Discard),
                    topo_with_deltas.to_owned(),
                    1024 * 1024,
                    Options {
                        mode: Mode::Customized,
                        ..Default::default()
                    },
                );
                let stat = entries_iter2.finalize_boxed().unwrap();
                assert_eq!(stat.objects_copied_from_pack, 1);
            }

            let mut entries_iter2 = output::entry::iter_from_counts::customized::iter_from_counts_with_topo(
                counts2.clone(),
                db.clone(),
                Box::new(progress::Discard),
                topo_with_deltas,
                1024 * 1024,
                Options {
                    mode: Mode::Customized,
                    ..Default::default()
                },
            );

            let entries2: Vec<_> = InOrderIter::from(entries_iter2.by_ref())
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .collect();

            assert_eq!(entries2.len(), counts2.len(), "length of input and output should equal");
            let delta_oids = std::collections::HashSet::from([objects[1], objects[3]]);
            for entry in entries2.iter() {
                if delta_oids.contains(&entry.id) {
                    assert!(matches!(entry.kind, entry::Kind::DeltaRef { .. }));
                } else {
                    assert!(matches!(entry.kind, entry::Kind::Base(..)));
                }
            }

            // Directly write to a pack file
            let tmp_dir = gix_testtools::tempfile::TempDir::new()?;
            let pack_file_path = tmp_dir.path().join("new.pack");
            let mut pack_file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&pack_file_path)?;
            let (_num_written_bytes, _pack_hash) = {
                let num_entries = entries2.len();
                let mut pack_writer = output::bytes::FromEntriesIter::new(
                    std::iter::once(Ok::<_, entry::iter_from_counts::Error>(entries2)),
                    &mut pack_file,
                    num_entries as u32,
                    pack::data::Version::V2,
                    gix_hash::Kind::Sha1,
                );
                let mut n = pack_writer.next().expect("one entries bundle was written")?;
                n += pack_writer.next().expect("the trailer was written")?;
                assert!(
                    pack_writer.next().is_none(),
                    "there is nothing more to iterate this time"
                );
                // verify we can still get the original parts back
                let hash = pack_writer.digest().expect("digest is available when iterator is done");
                let _ = pack_writer.input;
                let _ = pack_writer.into_write();
                (n, hash)
            };

            // TODO: parse pack file
        }
    }

    Ok(())
}

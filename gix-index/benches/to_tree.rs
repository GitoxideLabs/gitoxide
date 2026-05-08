use std::{hint::black_box, io::Read};

use bstr::ByteSlice;
use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use gix_index::{
    entry::{Flags, Mode},
    State,
};

fn to_tree(c: &mut Criterion) {
    let objects = MemoryDb {
        object_hash: gix_hash::Kind::Sha1,
    };
    let mut group = c.benchmark_group("to_tree");

    let mut flat = State::new(gix_hash::Kind::Sha1);
    for idx in 0..10_000 {
        flat.dangerously_push_entry(
            Default::default(),
            repeated_id(b'a'),
            Flags::empty(),
            Mode::FILE,
            format!("file-{idx:05}").as_bytes().as_bstr(),
        );
    }
    group.throughput(Throughput::Elements(flat.entries().len() as u64));
    group.bench_function("flat 10k files", |b| {
        b.iter(|| {
            let id = flat.to_tree(&objects, Default::default()).expect("tree can be written");
            black_box(id);
        });
    });

    let mut wide_deep = State::new(gix_hash::Kind::Sha1);
    for dir_idx in 0..100 {
        for file_idx in 0..100 {
            wide_deep.dangerously_push_entry(
                Default::default(),
                repeated_id(b'a'),
                Flags::empty(),
                Mode::FILE,
                format!("dir-{dir_idx:03}/file-{file_idx:03}").as_bytes().as_bstr(),
            );
        }
    }
    group.throughput(Throughput::Elements(wide_deep.entries().len() as u64));
    group.bench_function("wide 100 x 100 files", |b| {
        b.iter(|| {
            let id = wide_deep
                .to_tree(&objects, Default::default())
                .expect("tree can be written");
            black_box(id);
        });
    });

    let mut sparse = State::new(gix_hash::Kind::Sha1);
    for idx in 0..10_000 {
        sparse.dangerously_push_entry(
            Default::default(),
            repeated_id(b't'),
            Flags::empty(),
            Mode::DIR,
            format!("sparse-{idx:05}/").as_bytes().as_bstr(),
        );
    }
    group.throughput(Throughput::Elements(sparse.entries().len() as u64));
    group.bench_function("sparse 10k directories", |b| {
        b.iter(|| {
            let id = sparse
                .to_tree(&objects, Default::default())
                .expect("tree can be written");
            black_box(id);
        });
    });
}

criterion_group!(benches, to_tree);
criterion_main!(benches);

struct MemoryDb {
    object_hash: gix_hash::Kind,
}

impl gix_object::Exists for MemoryDb {
    fn exists(&self, _id: &gix_hash::oid) -> bool {
        true
    }
}

impl gix_object::Write for MemoryDb {
    fn write_buf(&self, kind: gix_object::Kind, from: &[u8]) -> Result<gix_hash::ObjectId, gix_object::write::Error> {
        Ok(gix_object::compute_hash(self.object_hash, kind, from)?)
    }

    fn write_stream(
        &self,
        kind: gix_object::Kind,
        _size: u64,
        from: &mut dyn Read,
    ) -> Result<gix_hash::ObjectId, gix_object::write::Error> {
        let mut buf = Vec::new();
        from.read_to_end(&mut buf)?;
        self.write_buf(kind, &buf)
    }
}

fn repeated_id(byte: u8) -> gix_hash::ObjectId {
    gix_hash::ObjectId::from_hex(&vec![byte; gix_hash::Kind::Sha1.len_in_hex()]).expect("valid hex")
}

//! Benchmarks for the word index.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use helix_core::Rope;
use helix_view::handlers::word_index::bench;

/// See `texts/README`. These texts are used in unicode-segmentation's benchmarks.
const CORPORA: &[(&str, &str)] = &[
    ("arabic", include_str!("texts/arabic.txt")),
    ("english", include_str!("texts/english.txt")),
    ("hindi", include_str!("texts/hindi.txt")),
    ("japanese", include_str!("texts/japanese.txt")),
    ("korean", include_str!("texts/korean.txt")),
    ("mandarin", include_str!("texts/mandarin.txt")),
    ("russian", include_str!("texts/russian.txt")),
    ("source_code", include_str!("texts/source_code.txt")),
];

/// Just extracting words from a rope (i.e. the hot path).
fn words(c: &mut Criterion) {
    let mut group = c.benchmark_group("words");
    for (name, text) in CORPORA {
        let rope = Rope::from_str(text);
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), &rope, |b, rope| {
            b.iter(|| {
                let mut count = 0usize;
                for word in bench::words(black_box(rope.slice(..))) {
                    black_box(word);
                    count += 1;
                }
                count
            });
        });
    }
    group.finish();
}

/// End-to-end extraction plus hashing / locking / inserting.
fn add_document(c: &mut Criterion) {
    let mut group = c.benchmark_group("add_document");
    for (name, text) in CORPORA {
        let rope = Rope::from_str(text);
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), &rope, |b, rope| {
            b.iter(|| {
                let index = bench::WordIndex::default();
                bench::add_document(&index, black_box(rope));
                index
            });
        });
    }
    group.finish();
}

criterion_group!(benches, words, add_document);
criterion_main!(benches);

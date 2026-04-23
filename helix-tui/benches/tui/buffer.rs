use criterion::{BenchmarkId, Criterion};
use helix_tui::buffer::{Buffer, Cell};
use helix_view::{graphics::Rect, theme::Style};
use std::hint::black_box;

criterion::criterion_group!(
    benches,
    empty,
    filled,
    diff_no_change,
    diff_partial_change,
    diff_full_change,
    diff_multi_width,
    diff_emoji,
);

const fn rect(size: u16) -> Rect {
    Rect::new(0, 0, size, size)
}

fn empty(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer/empty");
    for size in [16, 64, 255] {
        let area = rect(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &area, |b, &area| {
            b.iter(|| {
                let _buffer = Buffer::empty(black_box(area));
            });
        });
    }
    group.finish();
}

/// This likely should have the same performance as `empty`, but it's here for completeness
/// and to catch any potential performance regressions.
fn filled(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer/filled");
    for size in [16, 64, 255] {
        let area = rect(size);
        let cell = Cell::new("AAAA"); // simulate a multi-byte character
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(area, cell),
            |b, (area, cell)| {
                b.iter(|| {
                    let _buffer = Buffer::filled(black_box(*area), cell);
                });
            },
        );
    }
    group.finish();
}

fn diff_no_change(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer/diff_no_change");
    for size in [16, 64, 128] {
        let area = rect(size);
        let buffer = Buffer::filled(area, &Cell::new("x"));
        group.bench_with_input(BenchmarkId::from_parameter(size), &buffer, |b, buffer| {
            b.iter(|| {
                let diff = black_box(buffer).diff(black_box(buffer));
                black_box(diff);
            });
        });
    }
    group.finish();
}

/// This tests maximum update cost with every cell needing an update
fn diff_full_change(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer/diff_full_change");
    for size in [16, 64, 128] {
        let area = rect(size);
        let prev = Buffer::filled(area, &Cell::new("a"));
        let next = Buffer::filled(area, &Cell::new("b"));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(prev, next),
            |b, (prev, next)| {
                b.iter(|| {
                    let diff = black_box(prev).diff(black_box(next));
                    black_box(diff);
                });
            },
        );
    }
    group.finish();
}

/// This simulates typical incremental updates in a TUI
fn diff_partial_change(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer/diff_partial_change");
    for size in [16, 64, 128] {
        let area = rect(size);
        let prev = Buffer::filled(area, &Cell::new("a"));
        let mut next = Buffer::filled(area, &Cell::new("a"));

        let total_cells = (size as usize) * (size as usize);
        for i in (0..total_cells).step_by(10) {
            if i < next.content.len() {
                next.content[i].set_symbol("b");
            }
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(prev, next),
            |b, (prev, next)| {
                b.iter(|| {
                    let diff = black_box(prev).diff(black_box(next));
                    black_box(diff);
                });
            },
        );
    }
    group.finish();
}
fn diff_multi_width(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer/diff_multi_width");
    for size in [16, 64] {
        let area = rect(size);
        let prev = Buffer::with_lines(vec!["a".repeat(size as usize); size as usize]);
        let mut next = Buffer::filled(area, &Cell::new(" "));

        for y in 0..size {
            next.set_string(
                0,
                y,
                "日本語中文한국어".repeat(size as usize / 6),
                Style::default(),
            );
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(prev, next),
            |b, (prev, next)| {
                b.iter(|| {
                    let diff = black_box(prev).diff(black_box(next));
                    black_box(diff);
                });
            },
        );
    }
    group.finish();
}

/// Tests the emoji-specific VS16 clearing path for complex emoji sequences
fn diff_emoji(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer/diff_emoji");
    for size in [16, 64] {
        let area = rect(size);
        let prev = Buffer::filled(area, &Cell::new("a"));
        let mut next = Buffer::filled(area, &Cell::new(" "));

        for y in 0..size {
            next.set_string(0, y, "⌨️👁️‍🗨️🐻‍❄️".repeat(size as usize / 6), Style::default());
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(prev, next),
            |b, (prev, next)| {
                b.iter(|| {
                    let diff = black_box(prev).diff(black_box(next));
                    black_box(diff);
                });
            },
        );
    }
    group.finish();
}

//! Benchmarks for scanner operations: diff, sort, filter, export.
//!
//! Run with: `cargo bench -p prt-core`
//! Reports: `target/criterion/report/index.html`

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use prt_core::model::*;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Instant;

fn make_entries(n: usize) -> Vec<PortEntry> {
    (0..n)
        .map(|i| PortEntry {
            protocol: if i % 3 == 0 {
                Protocol::Udp
            } else {
                Protocol::Tcp
            },
            local_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 1024 + i as u16),
            remote_addr: if i % 2 == 0 {
                Some(SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::new(10, 0, 0, (i % 256) as u8)),
                    50000 + i as u16,
                ))
            } else {
                None
            },
            state: match i % 4 {
                0 => ConnectionState::Listen,
                1 => ConnectionState::Established,
                2 => ConnectionState::TimeWait,
                _ => ConnectionState::CloseWait,
            },
            process: ProcessInfo {
                pid: 1000 + i as u32,
                name: format!("proc-{i}"),
                path: None,
                cmdline: Some(format!("/usr/bin/proc-{i} --flag")),
                user: Some(format!("user{}", i % 5)),
                parent_pid: Some(1),
                parent_name: Some("init".into()),
            },
        })
        .collect()
}

fn make_tracked(entries: &[PortEntry]) -> Vec<TrackedEntry> {
    let now = Instant::now();
    entries
        .iter()
        .cloned()
        .map(|entry| TrackedEntry {
            entry,
            status: EntryStatus::Unchanged,
            seen_at: now,
        })
        .collect()
}

fn bench_diff(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_entries");
    for size in [10, 100, 500, 1000] {
        let prev_raw = make_entries(size);
        let prev = make_tracked(&prev_raw);

        // ~20% new, ~20% gone, ~60% unchanged
        let mut current = make_entries(size);
        // Remove last 20% (they'll become Gone)
        current.truncate(size * 80 / 100);
        // Add 20% new
        for i in 0..(size / 5) {
            current.push(PortEntry {
                protocol: Protocol::Tcp,
                local_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 60000 + i as u16),
                remote_addr: None,
                state: ConnectionState::Listen,
                process: ProcessInfo {
                    pid: 90000 + i as u32,
                    name: format!("new-{i}"),
                    path: None,
                    cmdline: None,
                    user: None,
                    parent_pid: None,
                    parent_name: None,
                },
            });
        }

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                prt_core::core::scanner::diff_entries(
                    black_box(&prev),
                    black_box(current.clone()),
                    Instant::now(),
                )
            });
        });
    }
    group.finish();
}

fn bench_sort(c: &mut Criterion) {
    let mut group = c.benchmark_group("sort_entries");
    let columns = [
        ("port", SortColumn::Port),
        ("pid", SortColumn::Pid),
        ("name", SortColumn::ProcessName),
        ("user", SortColumn::User),
    ];

    for (name, col) in columns {
        let entries_raw = make_entries(500);
        let tracked = make_tracked(&entries_raw);
        let state = SortState {
            column: col,
            ascending: true,
        };

        group.bench_with_input(BenchmarkId::from_parameter(name), &name, |b, _| {
            b.iter_batched(
                || tracked.clone(),
                |mut data| prt_core::core::scanner::sort_entries(black_box(&mut data), &state),
                criterion::BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter_indices");
    let entries_raw = make_entries(500);
    let tracked = make_tracked(&entries_raw);

    let queries = [
        ("empty", ""),
        ("port", "1080"),
        ("name", "proc-42"),
        ("proto", "udp"),
        ("no_match", "zzzzz"),
    ];

    for (name, query) in queries {
        group.bench_with_input(BenchmarkId::from_parameter(name), &name, |b, _| {
            b.iter(|| {
                prt_core::core::scanner::filter_indices(black_box(&tracked), black_box(query))
            });
        });
    }
    group.finish();
}

fn bench_export(c: &mut Criterion) {
    let mut group = c.benchmark_group("export");
    let entries = make_entries(200);

    group.bench_function("json_200", |b| {
        b.iter(|| {
            prt_core::core::scanner::export(black_box(&entries), ExportFormat::Json).unwrap()
        });
    });
    group.bench_function("csv_200", |b| {
        b.iter(|| prt_core::core::scanner::export(black_box(&entries), ExportFormat::Csv).unwrap());
    });
    group.finish();
}

criterion_group!(benches, bench_diff, bench_sort, bench_filter, bench_export);
criterion_main!(benches);

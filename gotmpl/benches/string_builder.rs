use gotmpl::enum_parser::{parse, parse_cap};
use gotmpl::simple_parser::parse as simple_parse;
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::collections::HashMap;

pub fn string_builder_benchmark(c: &mut Criterion) {
    let tmpl = std::fs::read_to_string("templates/large.tmpl").unwrap();
    
    let data = HashMap::from([
        ("name1".to_string(), "A1".to_string()),
        ("name2".to_string(), "A2".to_string()),
        ("name3".to_string(), "A3".to_string()),
        ("surname1".to_string(), "M1".to_string()),
        ("surname2".to_string(), "M2".to_string()),
        ("surname3".to_string(), "M3".to_string()),
    ]);

    let mut group = c.benchmark_group("string_builder");

    group.bench_with_input(
        BenchmarkId::new("enum_parser/simple", "large_tmpl"),
        &(tmpl.clone(), data.clone()), 
        |b, (tmpl, data)| {
            b.iter(|| parse(black_box(tmpl.clone()), black_box(data.clone())));
        });

    group.bench_with_input(
        BenchmarkId::new("enum_parser/capacity", "large_tmpl"),
        &(tmpl.clone(), data.clone()), 
        |b, (tmpl, data)| {
            b.iter(|| parse_cap(black_box(tmpl.clone()), black_box(data.clone())));
        });

    group.bench_with_input(
            BenchmarkId::new("simple_parser", "large_tmpl"),
            &(tmpl.clone(), data.clone()), 
            |b, (tmpl, data)| {
                b.iter(|| simple_parse(black_box(tmpl.clone()), black_box(data.clone())));
            });

    group.finish();
}

criterion_group!(benches, string_builder_benchmark);
criterion_main!(benches);

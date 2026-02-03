//! Benchmarks for the PHP deserialize parser.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use php_deserialize_core::from_bytes;

fn simple_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_types");

    // Null
    let null_data = b"N;";
    group.throughput(Throughput::Bytes(null_data.len() as u64));
    group.bench_function("null", |b| {
        b.iter(|| from_bytes(black_box(null_data)))
    });

    // Boolean
    let bool_data = b"b:1;";
    group.throughput(Throughput::Bytes(bool_data.len() as u64));
    group.bench_function("bool", |b| {
        b.iter(|| from_bytes(black_box(bool_data)))
    });

    // Integer
    let int_data = b"i:1234567890;";
    group.throughput(Throughput::Bytes(int_data.len() as u64));
    group.bench_function("int", |b| {
        b.iter(|| from_bytes(black_box(int_data)))
    });

    // Float
    let float_data = b"d:3.141592653589793;";
    group.throughput(Throughput::Bytes(float_data.len() as u64));
    group.bench_function("float", |b| {
        b.iter(|| from_bytes(black_box(float_data)))
    });

    group.finish();
}

fn strings(c: &mut Criterion) {
    let mut group = c.benchmark_group("strings");

    // Short string
    let short = b"s:5:\"hello\";";
    group.throughput(Throughput::Bytes(short.len() as u64));
    group.bench_function("short_5b", |b| {
        b.iter(|| from_bytes(black_box(short)))
    });

    // Medium string (100 bytes)
    let medium_content = "x".repeat(100);
    let medium = format!("s:100:\"{}\";", medium_content);
    let medium = medium.as_bytes();
    group.throughput(Throughput::Bytes(medium.len() as u64));
    group.bench_function("medium_100b", |b| {
        b.iter(|| from_bytes(black_box(medium)))
    });

    // Large string (10KB)
    let large_content = "x".repeat(10_000);
    let large = format!("s:10000:\"{}\";", large_content);
    let large = large.as_bytes();
    group.throughput(Throughput::Bytes(large.len() as u64));
    group.bench_function("large_10kb", |b| {
        b.iter(|| from_bytes(black_box(large)))
    });

    // Very large string (1MB)
    let huge_content = "x".repeat(1_000_000);
    let huge = format!("s:1000000:\"{}\";", huge_content);
    let huge = huge.as_bytes();
    group.throughput(Throughput::Bytes(huge.len() as u64));
    group.bench_function("huge_1mb", |b| {
        b.iter(|| from_bytes(black_box(huge)))
    });

    group.finish();
}

fn arrays(c: &mut Criterion) {
    let mut group = c.benchmark_group("arrays");

    // Empty array
    let empty = b"a:0:{}";
    group.throughput(Throughput::Bytes(empty.len() as u64));
    group.bench_function("empty", |b| {
        b.iter(|| from_bytes(black_box(empty)))
    });

    // Small array (10 elements)
    let small: String = {
        let items: String = (0..10)
            .map(|i| format!("i:{};i:{};", i, i * 2))
            .collect();
        format!("a:10:{{{}}}", items)
    };
    let small = small.as_bytes();
    group.throughput(Throughput::Bytes(small.len() as u64));
    group.bench_function("small_10", |b| {
        b.iter(|| from_bytes(black_box(small)))
    });

    // Medium array (100 elements)
    let medium: String = {
        let items: String = (0..100)
            .map(|i| format!("i:{};i:{};", i, i * 2))
            .collect();
        format!("a:100:{{{}}}", items)
    };
    let medium = medium.as_bytes();
    group.throughput(Throughput::Bytes(medium.len() as u64));
    group.bench_function("medium_100", |b| {
        b.iter(|| from_bytes(black_box(medium)))
    });

    // Large array (1000 elements)
    let large: String = {
        let items: String = (0..1000)
            .map(|i| format!("i:{};i:{};", i, i * 2))
            .collect();
        format!("a:1000:{{{}}}", items)
    };
    let large = large.as_bytes();
    group.throughput(Throughput::Bytes(large.len() as u64));
    group.bench_function("large_1000", |b| {
        b.iter(|| from_bytes(black_box(large)))
    });

    // Associative array with string keys
    let assoc: String = {
        let items: String = (0..100)
            .map(|i| {
                let key = format!("key_{}", i);
                format!("s:{}:\"{}\";i:{};", key.len(), key, i)
            })
            .collect();
        format!("a:100:{{{}}}", items)
    };
    let assoc = assoc.as_bytes();
    group.throughput(Throughput::Bytes(assoc.len() as u64));
    group.bench_function("assoc_100", |b| {
        b.iter(|| from_bytes(black_box(assoc)))
    });

    group.finish();
}

fn nested_structures(c: &mut Criterion) {
    let mut group = c.benchmark_group("nested");

    // Nested array (depth 10)
    let nested_10: String = {
        let mut s = String::from("s:4:\"leaf\";");
        for i in 0..10 {
            let key = format!("key{}", i);
            s = format!("a:1:{{s:{}:\"{}\";{}}}", key.len(), key, s);
        }
        s
    };
    let nested_10 = nested_10.as_bytes();
    group.throughput(Throughput::Bytes(nested_10.len() as u64));
    group.bench_function("depth_10", |b| {
        b.iter(|| from_bytes(black_box(nested_10)))
    });

    // Nested array (depth 50)
    let nested_50: String = {
        let mut s = String::from("s:4:\"leaf\";");
        for i in 0..50 {
            let key = format!("k{}", i % 10);
            s = format!("a:1:{{s:{}:\"{}\";{}}}", key.len(), key, s);
        }
        s
    };
    let nested_50 = nested_50.as_bytes();
    group.throughput(Throughput::Bytes(nested_50.len() as u64));
    group.bench_function("depth_50", |b| {
        b.iter(|| from_bytes(black_box(nested_50)))
    });

    group.finish();
}

fn real_world(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world");

    // Simulated form data
    let form_data = br#"a:3:{s:6:"fields";a:3:{i:0;a:3:{s:4:"type";s:4:"text";s:5:"label";s:4:"Name";s:8:"required";b:1;}i:1;a:3:{s:4:"type";s:5:"email";s:5:"label";s:5:"Email";s:8:"required";b:1;}i:2;a:3:{s:4:"type";s:8:"textarea";s:5:"label";s:7:"Message";s:8:"required";b:0;}}s:8:"settings";a:2:{s:11:"submit_text";s:6:"Submit";s:15:"success_message";s:10:"Thank you!";}s:11:"permissions";a:3:{i:0;s:4:"read";i:1;s:5:"write";i:2;s:6:"delete";}}"#;
    group.throughput(Throughput::Bytes(form_data.len() as u64));
    group.bench_function("form_data", |b| {
        b.iter(|| from_bytes(black_box(form_data)))
    });

    // DB-escaped data
    let escaped = br#""a:2:{s:4:""name"";s:5:""Alice"";s:3:""age"";i:30;}""#;
    group.throughput(Throughput::Bytes(escaped.len() as u64));
    group.bench_function("db_escaped", |b| {
        b.iter(|| from_bytes(black_box(escaped)))
    });

    group.finish();
}

#[cfg(feature = "serde")]
fn json_conversion(c: &mut Criterion) {
    use php_deserialize_core::json::to_json_string;

    let mut group = c.benchmark_group("json");

    let data = br#"a:3:{s:4:"name";s:5:"Alice";s:3:"age";i:30;s:4:"tags";a:2:{i:0;s:5:"admin";i:1;s:6:"active";}}"#;

    group.throughput(Throughput::Bytes(data.len() as u64));
    group.bench_function("parse_and_convert", |b| {
        b.iter(|| {
            let value = from_bytes(black_box(data)).unwrap();
            to_json_string(&value).unwrap()
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    simple_types,
    strings,
    arrays,
    nested_structures,
    real_world,
);

#[cfg(feature = "serde")]
criterion_group!(serde_benches, json_conversion);

#[cfg(feature = "serde")]
criterion_main!(benches, serde_benches);

#[cfg(not(feature = "serde"))]
criterion_main!(benches);

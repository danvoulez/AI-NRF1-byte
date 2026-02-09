use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use nrf_core::Value;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Test fixtures: small, medium, large values
// ---------------------------------------------------------------------------

fn make_small() -> Value {
    let mut m = BTreeMap::new();
    m.insert("name".into(), Value::String("test".into()));
    m.insert("value".into(), Value::Int(42));
    Value::Map(m)
}

fn make_medium() -> Value {
    let mut m = BTreeMap::new();
    m.insert("agent_id".into(), Value::String("did:ubl:lab512#key-1".into()));
    m.insert("action".into(), Value::String("evaluate".into()));
    m.insert("timestamp".into(), Value::Int(1738000000000000000));
    m.insert("nonce".into(), Value::Bytes(vec![0xAB; 16]));
    m.insert("verdict".into(), Value::String("ACK".into()));
    m.insert("reason".into(), Value::String("All checks passed".into()));

    let mut metrics = BTreeMap::new();
    metrics.insert("latency_ms".into(), Value::Int(42));
    metrics.insert("score".into(), Value::String("0.95".into()));
    metrics.insert("checks_passed".into(), Value::Int(7));
    m.insert("metrics".into(), Value::Map(metrics));

    let evidence = vec![
        Value::Bytes(vec![0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04,
                          0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C,
                          0x0D, 0x0E, 0x0F, 0x10, 0x11, 0x12, 0x13, 0x14,
                          0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C]),
    ];
    m.insert("evidence_cids".into(), Value::Array(evidence));
    Value::Map(m)
}

fn make_large() -> Value {
    let mut items = Vec::with_capacity(100);
    for i in 0..100 {
        let mut entry = BTreeMap::new();
        entry.insert("id".into(), Value::Int(i));
        entry.insert("name".into(), Value::String(format!("item_{i}")));
        entry.insert("active".into(), Value::Bool(i % 2 == 0));
        entry.insert("score".into(), Value::String(format!("{}.{}", i / 10, i % 10)));
        items.push(Value::Map(entry));
    }
    let mut root = BTreeMap::new();
    root.insert("version".into(), Value::String("receipt-v1".into()));
    root.insert("items".into(), Value::Array(items));
    root.insert("total".into(), Value::Int(100));
    Value::Map(root)
}

// ---------------------------------------------------------------------------
// Equivalent serde structs for bincode/json comparison
// ---------------------------------------------------------------------------

#[derive(serde::Serialize, serde::Deserialize)]
struct SmallDoc {
    name: String,
    value: i64,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct MediumMetrics {
    latency_ms: i64,
    score: String,
    checks_passed: i64,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct MediumDoc {
    agent_id: String,
    action: String,
    timestamp: i64,
    nonce: Vec<u8>,
    verdict: String,
    reason: String,
    metrics: MediumMetrics,
    evidence_cids: Vec<Vec<u8>>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct LargeItem {
    id: i64,
    name: String,
    active: bool,
    score: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct LargeDoc {
    version: String,
    items: Vec<LargeItem>,
    total: i64,
}

fn make_small_serde() -> SmallDoc {
    SmallDoc { name: "test".into(), value: 42 }
}

fn make_medium_serde() -> MediumDoc {
    MediumDoc {
        agent_id: "did:ubl:lab512#key-1".into(),
        action: "evaluate".into(),
        timestamp: 1738000000000000000,
        nonce: vec![0xAB; 16],
        verdict: "ACK".into(),
        reason: "All checks passed".into(),
        metrics: MediumMetrics {
            latency_ms: 42,
            score: "0.95".into(),
            checks_passed: 7,
        },
        evidence_cids: vec![vec![0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04,
                                  0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C,
                                  0x0D, 0x0E, 0x0F, 0x10, 0x11, 0x12, 0x13, 0x14,
                                  0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C]],
    }
}

fn make_large_serde() -> LargeDoc {
    let items: Vec<LargeItem> = (0..100).map(|i| LargeItem {
        id: i,
        name: format!("item_{i}"),
        active: i % 2 == 0,
        score: format!("{}.{}", i / 10, i % 10),
    }).collect();
    LargeDoc { version: "receipt-v1".into(), items, total: 100 }
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

fn bench_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode");

    let small = make_small();
    let medium = make_medium();
    let large = make_large();

    let small_s = make_small_serde();
    let medium_s = make_medium_serde();
    let large_s = make_large_serde();

    // ai-nrf1
    group.bench_with_input(BenchmarkId::new("nrf1", "small"), &small, |b, v| {
        b.iter(|| nrf_core::encode(black_box(v)))
    });
    group.bench_with_input(BenchmarkId::new("nrf1", "medium"), &medium, |b, v| {
        b.iter(|| nrf_core::encode(black_box(v)))
    });
    group.bench_with_input(BenchmarkId::new("nrf1", "large"), &large, |b, v| {
        b.iter(|| nrf_core::encode(black_box(v)))
    });

    // serde_json
    group.bench_with_input(BenchmarkId::new("json", "small"), &small_s, |b, v| {
        b.iter(|| serde_json::to_vec(black_box(v)).unwrap())
    });
    group.bench_with_input(BenchmarkId::new("json", "medium"), &medium_s, |b, v| {
        b.iter(|| serde_json::to_vec(black_box(v)).unwrap())
    });
    group.bench_with_input(BenchmarkId::new("json", "large"), &large_s, |b, v| {
        b.iter(|| serde_json::to_vec(black_box(v)).unwrap())
    });

    // bincode
    group.bench_with_input(BenchmarkId::new("bincode", "small"), &small_s, |b, v| {
        b.iter(|| bincode::serialize(black_box(v)).unwrap())
    });
    group.bench_with_input(BenchmarkId::new("bincode", "medium"), &medium_s, |b, v| {
        b.iter(|| bincode::serialize(black_box(v)).unwrap())
    });
    group.bench_with_input(BenchmarkId::new("bincode", "large"), &large_s, |b, v| {
        b.iter(|| bincode::serialize(black_box(v)).unwrap())
    });

    group.finish();
}

fn bench_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode");

    let small_nrf = nrf_core::encode(&make_small());
    let medium_nrf = nrf_core::encode(&make_medium());
    let large_nrf = nrf_core::encode(&make_large());

    let small_json = serde_json::to_vec(&make_small_serde()).unwrap();
    let medium_json = serde_json::to_vec(&make_medium_serde()).unwrap();
    let large_json = serde_json::to_vec(&make_large_serde()).unwrap();

    let small_bin = bincode::serialize(&make_small_serde()).unwrap();
    let medium_bin = bincode::serialize(&make_medium_serde()).unwrap();
    let large_bin = bincode::serialize(&make_large_serde()).unwrap();

    // ai-nrf1
    group.bench_with_input(BenchmarkId::new("nrf1", "small"), &small_nrf, |b, v| {
        b.iter(|| nrf_core::decode(black_box(v)).unwrap())
    });
    group.bench_with_input(BenchmarkId::new("nrf1", "medium"), &medium_nrf, |b, v| {
        b.iter(|| nrf_core::decode(black_box(v)).unwrap())
    });
    group.bench_with_input(BenchmarkId::new("nrf1", "large"), &large_nrf, |b, v| {
        b.iter(|| nrf_core::decode(black_box(v)).unwrap())
    });

    // serde_json
    group.bench_with_input(BenchmarkId::new("json", "small"), &small_json, |b, v| {
        b.iter(|| serde_json::from_slice::<SmallDoc>(black_box(v)).unwrap())
    });
    group.bench_with_input(BenchmarkId::new("json", "medium"), &medium_json, |b, v| {
        b.iter(|| serde_json::from_slice::<MediumDoc>(black_box(v)).unwrap())
    });
    group.bench_with_input(BenchmarkId::new("json", "large"), &large_json, |b, v| {
        b.iter(|| serde_json::from_slice::<LargeDoc>(black_box(v)).unwrap())
    });

    // bincode
    group.bench_with_input(BenchmarkId::new("bincode", "small"), &small_bin, |b, v| {
        b.iter(|| bincode::deserialize::<SmallDoc>(black_box(v)).unwrap())
    });
    group.bench_with_input(BenchmarkId::new("bincode", "medium"), &medium_bin, |b, v| {
        b.iter(|| bincode::deserialize::<MediumDoc>(black_box(v)).unwrap())
    });
    group.bench_with_input(BenchmarkId::new("bincode", "large"), &large_bin, |b, v| {
        b.iter(|| bincode::deserialize::<LargeDoc>(black_box(v)).unwrap())
    });

    group.finish();
}

fn bench_hash(c: &mut Criterion) {
    let mut group = c.benchmark_group("hash");

    let small = make_small();
    let medium = make_medium();
    let large = make_large();

    group.bench_with_input(BenchmarkId::new("blake3", "small"), &small, |b, v| {
        b.iter(|| nrf_core::hash_value(black_box(v)))
    });
    group.bench_with_input(BenchmarkId::new("blake3", "medium"), &medium, |b, v| {
        b.iter(|| nrf_core::hash_value(black_box(v)))
    });
    group.bench_with_input(BenchmarkId::new("blake3", "large"), &large, |b, v| {
        b.iter(|| nrf_core::hash_value(black_box(v)))
    });

    group.finish();
}

fn bench_size(c: &mut Criterion) {
    // Not a timing benchmark — just prints encoded sizes for comparison
    let small_nrf = nrf_core::encode(&make_small());
    let medium_nrf = nrf_core::encode(&make_medium());
    let large_nrf = nrf_core::encode(&make_large());

    let small_json = serde_json::to_vec(&make_small_serde()).unwrap();
    let medium_json = serde_json::to_vec(&make_medium_serde()).unwrap();
    let large_json = serde_json::to_vec(&make_large_serde()).unwrap();

    let small_bin = bincode::serialize(&make_small_serde()).unwrap();
    let medium_bin = bincode::serialize(&make_medium_serde()).unwrap();
    let large_bin = bincode::serialize(&make_large_serde()).unwrap();

    println!("\n=== Encoded sizes (bytes) ===");
    println!("         {:>8} {:>8} {:>8}", "nrf1", "json", "bincode");
    println!("small    {:>8} {:>8} {:>8}", small_nrf.len(), small_json.len(), small_bin.len());
    println!("medium   {:>8} {:>8} {:>8}", medium_nrf.len(), medium_json.len(), medium_bin.len());
    println!("large    {:>8} {:>8} {:>8}", large_nrf.len(), large_json.len(), large_bin.len());

    // Dummy benchmark so criterion doesn't complain
    c.bench_function("size_report", |b| b.iter(|| 1 + 1));
}

criterion_group!(benches, bench_encode, bench_decode, bench_hash, bench_size);
criterion_main!(benches);

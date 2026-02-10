use nrf_core::Value;
use proptest::prelude::*;
use std::collections::BTreeMap;
use ubl_json_view::{from_json, to_json};

// ---------------------------------------------------------------------------
// Strategy: generate arbitrary NRF Values (no floats, NFC strings, no BOM)
// ---------------------------------------------------------------------------

fn arb_value() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        any::<i64>().prop_map(Value::Int),
        // ASCII-only strings to avoid NFC issues in generated data
        "[a-zA-Z0-9_]{0,20}".prop_map(Value::String),
        // Bytes of specific lengths that roundtrip cleanly
        prop_oneof![
            Just(vec![]).prop_map(Value::Bytes), // empty
            proptest::collection::vec(any::<u8>(), 16).prop_map(Value::Bytes), // 16
            proptest::collection::vec(any::<u8>(), 32).prop_map(Value::Bytes), // 32
            proptest::collection::vec(any::<u8>(), 64).prop_map(Value::Bytes), // 64
            proptest::collection::vec(any::<u8>(), 1..=15).prop_map(Value::Bytes), // other
        ],
    ];

    leaf.prop_recursive(
        3,  // depth
        32, // max nodes
        8,  // items per collection
        |inner| {
            prop_oneof![
                proptest::collection::vec(inner.clone(), 0..4).prop_map(Value::Array),
                proptest::collection::vec(("[a-z]{1,8}", inner), 0..4).prop_map(|pairs| {
                    let mut m = BTreeMap::new();
                    for (k, v) in pairs {
                        m.insert(k, v);
                    }
                    Value::Map(m)
                }),
            ]
        },
    )
}

// ---------------------------------------------------------------------------
// Property: roundtrip JSON ↔ Value is idempotent
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn roundtrip_json_value(v in arb_value()) {
        let j = to_json(&v);
        let back = from_json(&j).expect("from_json must succeed on to_json output");
        prop_assert_eq!(&v, &back, "roundtrip failed");
    }

    #[test]
    fn roundtrip_nrf_json_nrf(v in arb_value()) {
        let nrf1 = nrf_core::encode(&v);
        let j = ubl_json_view::nrf_bytes_to_json(&nrf1).expect("nrf_bytes_to_json");
        let nrf2 = ubl_json_view::json_to_nrf_bytes(&j).expect("json_to_nrf_bytes");
        prop_assert_eq!(nrf1, nrf2, "NRF roundtrip failed");
    }
}

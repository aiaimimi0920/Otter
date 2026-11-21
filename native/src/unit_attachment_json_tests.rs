//! 规范摘要与主干 JSON.stringify 边界；可选语料输出由验证脚本交给真实 Node 作独立判定。
use super::*;
use serde_json::json;
#[test]
fn canonical_ecmascript_number_boundaries_and_utf16_keys() {
    for (raw, expected) in [
        ("-0.0", "0"),
        ("1.0", "1"),
        ("0.1", "0.1"),
        ("1e-6", "0.000001"),
        ("1e-7", "1e-7"),
        ("1e20", "100000000000000000000"),
        ("1e21", "1e+21"),
        ("1e30", "1e+30"),
        ("333333333.33333329", "333333333.3333333"),
        ("4.50", "4.5"),
        ("9007199254740993", "9007199254740992"),
        ("1e-27", "1e-27"),
        ("1.7976931348623157e308", "1.7976931348623157e+308"),
        ("5e-324", "5e-324"),
    ] {
        let value: Value = serde_json::from_str(raw).unwrap();
        assert_eq!(
            String::from_utf8(canonical(&value)).unwrap(),
            expected,
            "{raw}"
        );
    }
    let value = json!({"\u{e000}":3,"😀":2,"a":[-0.,1.0,"换行\n"]});
    assert_eq!(
        String::from_utf8(canonical(&value)).unwrap(),
        "{\"a\":[0,1,\"换行\\n\"],\"😀\":2,\"\u{e000}\":3}"
    );
    assert_eq!(
        digest(&json!(null)),
        "74234e98afe7498fb5daf1f36ac2d78acc339464f950703b8c019892f982b90b"
    );
}
#[test]
fn json_budget_rejects_depth_nodes_and_bytes() {
    let mut value = json!(0);
    for _ in 0..32 {
        value = json!([value]);
    }
    assert!(bounded(&value, 1024).is_ok());
    assert!(bounded(&json!([value]), 1024).is_err());
    assert!(bounded(&json!("x".repeat(64)), 64).is_err());
    assert!(bounded(&json!([0, 0, 0, 0]), 4).is_err());
}
#[test]
fn finite_number_corpus_retains_roundtrip_and_exports_optional_node_oracle() {
    let mut bits = 0x4da5_163b_234f_c821u64;
    let mut rows = Vec::new();
    for _ in 0..16384 {
        bits = bits
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let value = f64::from_bits(bits);
        if !value.is_finite() {
            continue;
        }
        let encoded = number(&serde_json::Number::from_f64(value).unwrap());
        assert_eq!(encoded.parse::<f64>().unwrap().to_bits(), value.to_bits());
        rows.push((format!("{bits:016x}"), encoded));
    }
    if let Some(path) = std::env::var_os("OTTER_ATTACHMENT_JSON_TEST_OUTPUT") {
        std::fs::write(path, serde_json::to_vec(&rows).unwrap()).unwrap();
    }
}

//! Font ordering, exact deduplication, caching/retry, and actual local enumeration; no user-image/font-file reads.
use super::*;
#[test]
fn presets_are_fixed_then_trimmed_exact_duplicates_and_numeric_chinese_order() {
    let fonts = merge(
        [
            "  ", " Arial ", "arial", "Font10", "Font2", "Font2", "中字", "阿字", "École", "ecole",
        ]
        .map(str::to_owned)
        .to_vec(),
    )
    .unwrap();
    assert_eq!(&fonts[..9], &PRESETS.map(str::to_owned));
    assert!(fonts.contains(&"arial".to_owned()));
    assert_eq!(fonts.iter().filter(|n| *n == "Font2").count(), 1);
    let at = |name| fonts.iter().position(|n| n == name).unwrap();
    assert!(at("Font2") < at("Font10"));
    assert!(at("阿字") < at("中字"));
    let source = vec![
        "Font10", "Font2", "字体10", "字体2", "阿字", "中字", "École", "ecole", "a-b", "ab", "A B",
    ];
    let sorted = merge(source.iter().map(|n| (*n).to_owned()).collect()).unwrap();
    assert_eq!(
        &sorted[9..],
        &[
            "阿字", "中字", "字体2", "字体10", "A B", "a-b", "ab", "École", "ecole", "Font2",
            "Font10"
        ]
    );
    println!(
        "FONT_COLLATION_FIXTURE={}",
        serde_json::json!({"source":source, "sorted":sorted})
    );
}
#[test]
fn cache_only_keeps_success_and_reuses_the_same_catalog() {
    let mut catalog = Catalog::default();
    assert!(catalog
        .get(|| Err(Error::new(E_FAIL, "enumeration failed")))
        .is_err());
    assert!(catalog.0.is_none());
    let first = catalog.get(|| Ok(vec!["Font2".into()])).unwrap();
    let second = catalog
        .get(|| panic!("Cached catalog must not enumerate again"))
        .unwrap();
    assert!(Rc::ptr_eq(&first, &second));
}
#[test]
fn actual_gdi_catalog_contains_no_vertical_or_empty_names_and_is_success_cached() {
    let source = enumerate().unwrap();
    assert!(!source.is_empty());
    assert!(source
        .iter()
        .all(|s| !s.is_empty() && !s.starts_with('@') && s == s.trim()));
    let sorted = merge(source.clone()).unwrap();
    assert_eq!(&sorted[..9], &PRESETS.map(str::to_owned));
    let first = list().unwrap();
    let second = list().unwrap();
    assert!(Rc::ptr_eq(&first, &second));
    println!(
        "FONT_COLLATION_INSTALLED={}",
        serde_json::json!({"source":source, "sorted":sorted})
    );
}

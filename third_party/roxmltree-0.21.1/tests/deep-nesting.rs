//! Iterative element tokenization must preserve structure and malformed-input errors.
use roxmltree::Document;

#[test]
fn deeply_nested_elements_keep_names_text_and_siblings() {
    for depth in [512, 4096] {
        let xml = format!(
            "<root>{}<leaf/>text{}<tail/></root>",
            "<n>".repeat(depth),
            "</n>".repeat(depth)
        );
        let document = Document::parse(&xml).unwrap();
        let root = document.root_element();
        assert_eq!(root.tag_name().name(), "root");
        assert_eq!(root.children().filter(|n| n.is_element()).count(), 2);
        assert_eq!(root.last_element_child().unwrap().tag_name().name(), "tail");
        assert_eq!(
            document
                .descendants()
                .filter(|n| n.has_tag_name("n"))
                .count(),
            depth
        );
        let leaf = document
            .descendants()
            .find(|n| n.has_tag_name("leaf"))
            .unwrap();
        assert_eq!(leaf.tail(), Some("text"));
        assert_eq!(
            leaf.ancestors().filter(|n| n.has_tag_name("n")).count(),
            depth
        );
    }
}

#[test]
fn deeply_nested_mismatches_and_truncation_still_fail() {
    let prefix = "<n>".repeat(512);
    for suffix in ["</wrong>", "", "<", "</n>", "</n></extra>"] {
        assert!(Document::parse(&format!("{prefix}{suffix}")).is_err());
    }
}

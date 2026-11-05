use super::*;
fn asset(id: &str, mime: &str, bytes: Vec<u8>, dimensions: [u32; 2]) -> Asset {
    Asset::accept(
        Resource {
            id: id.into(),
            mime_type: mime.into(),
            size: bytes.len() as u64,
            digest: format!("{:x}", Sha256::digest(&bytes)),
            dimensions,
        },
        bytes,
    )
    .unwrap()
}
fn content() -> Content {
    Content::empty(crate::surface_javascript::fixture::snapshot(1, 1))
}
#[test]
fn package_text_binary_and_png_join_graph_inputs_without_alias_or_copying_png() {
    let png = crate::surface_javascript_wire::images::tests::resource();
    let assets = vec![
        asset(
            "config",
            "application/json",
            br#"{"answer":42}"#.to_vec(),
            [0, 0],
        ),
        asset("data", "application/octet-stream", vec![0, 255], [0, 0]),
        asset("pixel", "image/png", png.png.to_vec(), [2, 2]),
    ];
    let mut c = content();
    c.images.push(png);
    c.resources = json!({"art-input:0":"old"});
    let c = merge(c, &assets).unwrap();
    assert_eq!(c.resources["art-package:config"], r#"{"answer":42}"#);
    assert_eq!(
        c.resources["art-package:data"],
        "data:application/octet-stream;base64,AP8="
    );
    assert_eq!(c.resources["art-input:0"], "old");
    assert!(c.images[1].png.same_storage(&assets[2].bytes));
    assert_eq!(
        c.snapshot["state"]["package"]["pixel"]["resourceId"],
        "art-package:pixel"
    );
}
#[test]
fn package_merge_enforces_combined_graph_image_and_json_budget_and_reserved_namespace() {
    let png = crate::surface_javascript_wire::images::tests::resource();
    let assets = vec![asset("pixel", "image/png", png.png.to_vec(), [2, 2])];
    let mut c = content();
    for i in 0..8 {
        c.images.push(js::ImageResource {
            id: format!("art-input:{i}"),
            width: 2,
            height: 2,
            png: png.png.clone(),
        });
    }
    assert!(merge(c, &assets).is_err());
    let mut c = content();
    c.resources = json!({"art-package:pixel":"forged"});
    assert!(merge(c, &assets).is_err());
    let mut c = content();
    c.snapshot["state"]["package"] = json!({});
    assert!(merge(c, &assets).is_err());
    let text = asset("text", "text/plain", vec![1; 128 * 1024], [0, 0]);
    assert!(merge(content(), &[text]).is_err());
}
#[test]
fn package_assets_reject_wrong_digest_utf8_json_and_png_dimensions_before_mount() {
    for (mime, bytes, dim) in [
        ("text/plain", vec![255], [0, 0]),
        ("application/json", b"not json".to_vec(), [0, 0]),
        ("image/png", vec![0; 33], [1, 1]),
    ] {
        let r = Resource {
            id: "bad".into(),
            mime_type: mime.into(),
            size: bytes.len() as u64,
            digest: format!("{:x}", Sha256::digest(&bytes)),
            dimensions: dim,
        };
        assert!(Asset::accept(r, bytes).is_err());
    }
    let mut r = Resource {
        id: "bad".into(),
        mime_type: "text/plain".into(),
        size: 1,
        digest: "a".repeat(64),
        dimensions: [0, 0],
    };
    assert!(Asset::accept(r.clone(), vec![0]).is_err());
    r.digest = format!("{:x}", Sha256::digest([0]));
    assert!(Asset::accept(r, vec![0]).is_err());
}

//! WIC/Direct2D 实际像素验证字形、主题色、居中与遮挡，不依赖截图猜测。
use super::paint_pixels::{render, WIDTH};
#[test]
fn surface_icon_large_glyph_stays_inside_narrow_parent_clip() {
    let pixels = render(br#"{"id":"root","type":"stack","layout":{"width":16},"style":{"fontSize":64,"whiteSpace":"nowrap","overflow":"hidden"},"children":[{"id":"icon","type":"icon","props":{"glyph":"X"}}]}"#);
    assert!(pixels.chunks_exact(4).any(|p| p[3] > 0));
    assert!(pixels
        .chunks_exact(4)
        .enumerate()
        .filter(|(i, _)| i % WIDTH >= 16)
        .all(|(_, p)| p == [0, 0, 0, 0]));
}
#[test]
fn surface_icon_centers_content_box_without_overriding_multiline_text_alignment() {
    let draw = |align| {
        let json = format!(
            r#"{{"id":"root","type":"view","style":{{"whiteSpace":"pre","textAlign":"{align}"}},"children":[{{"id":"icon","type":"icon","props":{{"glyph":"XX\nX"}}}}]}}"#
        );
        render(json.as_bytes())
    };
    let left = draw("left");
    let right = draw("right");
    assert_ne!(left, right);
    let occupied = |pixels: &[u8]| {
        pixels
            .chunks_exact(4)
            .enumerate()
            .filter(|(_, p)| p[3] > 20)
            .map(|(i, _)| i % WIDTH)
            .collect::<Vec<_>>()
    };
    for pixels in [&left, &right] {
        let xs = occupied(pixels);
        assert!(!xs.is_empty());
        assert!(*xs.iter().min().unwrap() > 15 && *xs.iter().max().unwrap() < 48);
    }
}
#[test]
fn surface_icon_paints_signal_glyph_centered_with_empty_and_hidden_variants() {
    let pixels = render(br#"{"id":"root","type":"stack","children":[{"id":"icon","type":"icon","props":{"glyph":"X"}}]}"#);
    let occupied: Vec<_> = pixels
        .chunks_exact(4)
        .enumerate()
        .filter(|(_, p)| p[3] > 20)
        .collect();
    assert!(!occupied.is_empty());
    assert!(occupied.iter().all(|(i, p)| (20..44).contains(&(i % WIDTH))
        && (20..44).contains(&(i / WIDTH))
        && p[1] > p[2]
        && p[2] > p[0]));
    for props in [r#"{"glyph":""}"#, r#"{"glyph":"X","visible":false}"#] {
        let json = format!(r#"{{"id":"icon","type":"icon","props":{props}}}"#);
        assert!(render(json.as_bytes()).iter().all(|b| *b == 0));
    }
}
#[test]
fn surface_icon_defaults_and_ignored_style_render_identically() {
    let default = render(br#"{"id":"icon","type":"icon"}"#);
    let explicit = render("{\"id\":\"icon\",\"type\":\"icon\",\"props\":{\"glyph\":\"•\"},\"style\":{\"color\":\"red\",\"opacity\":0},\"layout\":{\"width\":1}}".as_bytes());
    assert_eq!(default, explicit);
    assert!(default.iter().any(|b| *b != 0));
}
#[test]
fn surface_icon_obeys_parent_opacity_and_later_sibling_occlusion() {
    let covered = render(br#"{"id":"root","type":"stack","children":[{"id":"icon","type":"icon"},{"id":"cover","type":"view","style":{"background":"blue"}}]}"#);
    assert!(covered.chunks_exact(4).all(|p| p == [255, 0, 0, 255]));
    let half = render(br#"{"id":"root","type":"view","style":{"opacity":0.5},"children":[{"id":"icon","type":"icon","props":{"glyph":"X"}}]}"#);
    assert!(half.chunks_exact(4).any(|p| p[3] > 0));
    assert!(half.chunks_exact(4).all(|p| p[3] <= 128));
}

//! 生产 Mount 仅映射 Web 已明确应用的 textarea 自身 textAlign；不扩大 input/number。
use super::*;
fn aligned(kind: &str, align: &str) -> Scene {
    Scene::parse(
        serde_json::json!({"id":"field","type":kind,"props":{"value":"42","placeholder":"提示"},
        "style":{"textAlign":align},"layout":{"width":180,"height":80},"events":{"input":"typing"}})
        .to_string()
        .as_bytes(),
    )
    .unwrap()
}
#[test]
fn surface_edit_alignment_mount_applies_textarea_only_and_keeps_draft_on_same_contract() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for (align, expected) in [
        ("left", ES_LEFT),
        ("center", ES_CENTER),
        ("right", ES_RIGHT),
        ("bad", ES_LEFT),
    ] {
        for kind in ["textarea", "input", "number"] {
            let window = SurfaceWindow::new(aligned(kind, align), 240, 160).unwrap();
            window.bind_edits(&binding()).unwrap();
            window.set_input_policy(true, true).unwrap();
            let edit = child(&window);
            let flags = unsafe { GetWindowLongPtrW(edit, GWL_STYLE) } as u32;
            assert_eq!(
                flags & ((ES_CENTER | ES_RIGHT) as u32),
                if kind == "textarea" {
                    expected as u32
                } else {
                    ES_LEFT as u32
                }
            );
            type_text(edit, "73");
            window.replace(aligned(kind, align)).unwrap();
            assert_eq!(child(&window), edit);
            assert_eq!(text(edit), "73");
            assert_eq!(window.prepare_edit(&binding(), 1).unwrap().len(), 1);
            assert!(window.take_error().is_none());
        }
    }
}

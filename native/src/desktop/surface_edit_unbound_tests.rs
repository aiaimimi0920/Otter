//! 真实EDIT复用不授普通Surface Binding；wire仍显式使用宿主自己的身份。
use super::*;
#[test]
fn unbound_session_never_accepts_bound_prepare_or_bound_synchronization() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (parent, mut bound) = setup();
    let scene = scene("source");
    let identity = crate::surface_click_event::Identity {
        instance: "wall-art",
        attachment: "wall-view",
        generation: 3,
    };
    assert!(bound
        .prepare_unbound(scene.root(), identity, 1, Phase::Input, None)
        .is_err());
    assert!(bound.synchronize_unbound(scene.root()).is_err());
    let mut local = Session::unbound(
        parent.dialog_owner().unwrap(),
        scene.root(),
        RECT {
            left: 0,
            top: 0,
            right: 100,
            bottom: 30,
        },
    )
    .unwrap();
    assert!(local
        .prepare(scene.root(), &binding(), 1, Phase::Input)
        .is_err());
    assert!(local.synchronize(scene.root(), &binding()).is_err());
    local.control().set_text("中文🙂").unwrap();
    let (id, bytes) = local
        .prepare_unbound(scene.root(), identity, 1, Phase::Input, None)
        .unwrap()
        .unwrap();
    let event: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(event["instanceId"], "wall-art");
    assert_eq!(event["attachmentId"], "wall-view");
    assert_eq!(event["generation"], 3);
    assert_eq!(event["payload"]["value"], "中文🙂");
    local.control().set_text("later").unwrap();
    local.capture().unwrap();
    local
        .synchronize_unbound(super::scene("remote").root())
        .unwrap();
    local.settle(&id).unwrap();
    assert_eq!(local.value(), "later");
}

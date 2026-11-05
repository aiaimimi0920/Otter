//! 真实隐藏 HWND 消费同 child 文字帧；省略/clip/恢复像素、盒命中与旧帧原子性。
use super::*;
use crate::surface_javascript::text_overflow_tests::{view, SOURCE};
#[test]
fn javascript_text_overflow_real_hwnd_pixels_geometry_hits_and_atomic_replacement() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let (mut session, frame) = Session::mount_in(
        fixture::binding(),
        SOURCE.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        Some(view()),
        &|| false,
    )
    .unwrap();
    let id = frame.scene.root().children()[0].id().to_owned();
    let window = SurfaceWindow::new(frame.scene, 400, 80).unwrap();
    assert_eq!(
        css_tests::geometry(&window, &id),
        Some(([0., 0.], [84., 32.]))
    );
    let first = css_tests::pixels(&window);
    assert!((0..32).any(|y| (0..84).any(|x| css_tests::pixel(&first, x, y)[0] > 0)));
    {
        let host = window.state.host.borrow();
        let presentation = host.as_ref().unwrap().presentation();
        assert_eq!(presentation.hit_test([82., 12.]), Some(id.as_str()));
        assert_eq!(
            presentation.hit_test([85., 12.]),
            Some(presentation.scene().root().id())
        );
    }
    let bad=Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"bad","type":"text","props":{"text":"bad"},"style":{"textOverflow":"ellipsis","whiteSpace":"normal"},"layout":{"width":84,"height":32,"overflowX":"hidden","overflowY":"hidden"}}]}"#).unwrap();
    assert!(window.replace(bad).is_err());
    assert_eq!(
        css_tests::pixels(&window),
        first,
        "a failed final layout must not replace installed pixels"
    );
    let next = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    window.replace(next.scene).unwrap();
    assert_eq!(
        css_tests::geometry(&window, &id),
        Some(([0., 0.], [84., 32.]))
    );
    assert_ne!(
        css_tests::pixels(&window),
        first,
        "clip must not draw the same pixels as ellipsis"
    );
    session
        .update(fixture::snapshot(5, 46), fixture::resources(), &|| false)
        .unwrap();
    let restored = session
        .update(fixture::snapshot(6, 47), fixture::resources(), &|| false)
        .unwrap();
    window.replace(restored.scene).unwrap();
    assert_eq!(css_tests::pixels(&window), first);
    assert!(window.take_error().is_none());
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
    assert!(!window.is_alive());
}

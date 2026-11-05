//! 同 child 的字体 shorthand/行高真正进入 HWND Presentation 的度量和像素。
use super::*;
#[test]
fn javascript_typography_real_hwnd_uses_actual_line_spacing_and_font_reset() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let (mut session, frame) = Session::mount(
        fixture::binding(),
        include_str!("../../tests/fixtures/javascript-typography.js").into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    let ids: Vec<_> = frame
        .scene
        .root()
        .children()
        .iter()
        .map(|node| node.id().to_owned())
        .collect();
    let window = SurfaceWindow::new(frame.scene, 200, 120).unwrap();
    let check = |updated| {
        let host = window.state.host.borrow();
        let presentation = host.as_ref().unwrap().presentation();
        let engine = crate::surface_text::Engine::new().unwrap();
        for (index, id) in ids.iter().enumerate() {
            let item = presentation
                .text()
                .items()
                .iter()
                .find(|item| item.node_id() == id)
                .unwrap();
            let (size, height, weight, family) = match (updated, index) {
                (false, 0) => (20., Some(30.), 700, "Segoe UI"),
                (false, _) => (12., None, 600, "Consolas"),
                (true, 0) => (24., Some(30.), 400, "Segoe UI"),
                (true, _) => (14., Some(28.), 700, "Segoe UI"),
            };
            let expected = engine
                .layout(
                    "WWii 0123",
                    crate::surface_text::Style {
                        size,
                        line_height: height,
                        weight,
                        family,
                        ..crate::surface_viewport::style()
                    },
                    item.allocated()[0],
                    32768.,
                )
                .unwrap();
            assert_eq!(item.layout().size(), expected.size());
            assert_eq!(item.layout().baseline(), expected.baseline());
        }
    };
    check(false);
    let before = css_tests::pixels(&window);
    assert!(before.chunks_exact(4).any(|p| p[3] != 0));
    let frame = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    window.replace(frame.scene).unwrap();
    check(true);
    assert_ne!(before, css_tests::pixels(&window));
    unsafe {
        SendMessageW(window.hwnd, WM_PAINT, None, None);
    }
    assert!(window.take_error().is_none());
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
    assert!(!window.is_alive());
}

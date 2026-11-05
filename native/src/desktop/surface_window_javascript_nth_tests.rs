//! 同一 child 的位置规则经真实 HWND/Presentation/D2D 验证；不注入全局输入。
use super::*;
#[test]
fn javascript_nth_child_real_hwnd_grid_pixels_follow_reorder_and_same_child_update() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let binding = fixture::binding();
    let (mut session, frame) = Session::mount(
        binding.clone(),
        include_str!("../../tests/fixtures/javascript-nth-child.js").into(),
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
        .map(|n| n.id().to_owned())
        .collect();
    let window = SurfaceWindow::new(frame.scene, 200, 80).unwrap();
    let check = |shift: usize| {
        let pixels = css_tests::pixels(&window);
        for index in 0..8 {
            let origin = [(index % 4) as f32 * 30., (index / 4) as f32 * 24.];
            assert_eq!(
                css_tests::geometry(&window, &ids[(index + shift) % 8]),
                Some((origin, [30., 24.]))
            );
            let color = if index % 4 == 3 {
                [0, 255, 0, 255]
            } else {
                [255, 0, 0, 255]
            };
            assert_eq!(
                css_tests::pixel(&pixels, origin[0] as usize + 8, origin[1] as usize + 8),
                color
            );
        }
        let host = window.state.host.borrow();
        let presentation = host.as_ref().unwrap().presentation();
        assert_eq!(
            presentation.hit_test([98., 8.]),
            Some(ids[(3 + shift) % 8].as_str())
        );
    };
    check(0);
    window.set_pointer_enabled(true).unwrap();
    window.javascript_queue_click_for_test(&ids[0]);
    let candidate = window.take_click().unwrap();
    let first = candidate.node_id.clone();
    assert!(window
        .click_events_for_candidate(candidate, &binding, 3)
        .unwrap()
        .is_empty());
    assert_eq!(first, ids[0]);
    let frame = session.click(&first, &|| false).unwrap();
    assert!(frame.events.is_empty());
    window.replace(frame.scene).unwrap();
    check(1);
    let frame = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    window.replace(frame.scene).unwrap();
    check(0);
    unsafe {
        SendMessageW(window.hwnd, WM_PAINT, None, None);
    }
    assert!(window.take_error().is_none());
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
    assert!(!window.is_alive());
    println!("native-javascript-nth-child-hwnd: grid=true pixels=true hits=true reorder=true same-child=true closed=true");
}

//! 同 JS child 的二维 CSS → 生产 Grid/DirectWrite/D2D/命中；不抢前台或模拟全局输入。
use super::{
    css_tests::{geometry, pixel, pixels},
    *,
};
#[test]
fn javascript_grid_real_hwnd_pixels_hits_click_reflow_hidden_and_update() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let binding = fixture::binding();
    let (mut session, frame) = Session::mount(
        binding.clone(),
        include_str!("../../tests/fixtures/javascript-grid.js").into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    let first = frame.scene.root().children()[0].id().to_owned();
    let second = frame.scene.root().children()[1].id().to_owned();
    let third = frame.scene.root().children()[2].id().to_owned();
    let root = frame.scene.root().id().to_owned();
    // Window::new 参数是外框尺寸；剩余 fr 宽使用真实 client 的原几何，不假定边框/DPI。
    let window = SurfaceWindow::new(frame.scene, 200, 140).unwrap();
    let width = geometry(&window, &root).unwrap().1[0];
    assert!(width > 120. && width <= 200.);
    assert_eq!(geometry(&window, &first), Some(([0., 0.], [40., 30.])));
    assert_eq!(
        geometry(&window, &second),
        Some(([48., 0.], [width - 48., 30.]))
    );
    assert_eq!(geometry(&window, &third), Some(([0., 34.], [width, 30.])));
    let before = pixels(&window);
    assert_eq!(pixel(&before, 1, 1), [0, 255, 0, 255]);
    assert_eq!(pixel(&before, 45, 1), [16, 16, 16, 255]);
    assert_eq!(pixel(&before, 1, 35), [0, 255, 0, 255]);
    assert!((5..28).any(|y| (1..39).any(|x| pixel(&before, x, y)[..3].iter().all(|v| *v > 220))));
    window.set_pointer_enabled(true).unwrap();
    window.javascript_queue_click_for_test(&first);
    let candidate = window.take_click().unwrap();
    assert_eq!(candidate.node_id, first);
    assert!(window
        .click_events_for_candidate(candidate, &binding, 3)
        .unwrap()
        .is_empty());
    let frame = session.click(&first, &|| false).unwrap();
    assert_eq!(frame.events.len(), 1);
    window.replace(frame.scene).unwrap();
    assert_eq!(geometry(&window, &first), Some(([0., 0.], [60., 30.])));
    assert_eq!(geometry(&window, &third), Some(([0., 36.], [width, 30.])));
    assert!(geometry(&window, &second).is_none());
    let after = pixels(&window);
    assert_eq!(pixel(&after, 1, 1), [255, 0, 0, 255]);
    assert_eq!(pixel(&after, 70, 1), [16, 16, 16, 255]);
    {
        let host = window.state.host.borrow();
        let presentation = host.as_ref().unwrap().presentation();
        assert_eq!(
            presentation.hit_test([70., 10.]),
            Some(presentation.scene().root().id())
        );
        assert_eq!(presentation.hit_test([10., 40.]), Some(third.as_str()));
    }
    let frame = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    window.replace(frame.scene).unwrap();
    assert_eq!(geometry(&window, &first), Some(([0., 0.], [80., 24.])));
    assert_eq!(
        geometry(&window, &second),
        Some(([92., 0.], [width - 92., 24.]))
    );
    assert_eq!(
        geometry(&window, &third),
        Some(([92., 32.], [width - 92., 40.]))
    );
    assert_eq!(pixel(&pixels(&window), 1, 1), [0, 255, 255, 255]);
    window.javascript_queue_click_for_test(&third);
    assert_eq!(window.take_click().unwrap().node_id, third);
    unsafe {
        SendMessageW(window.hwnd, WM_PAINT, None, None);
    }
    assert!(window.take_error().is_none());
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
    assert!(!window.is_alive());
    println!("native-javascript-grid-hwnd: geometry=true pixels=true inherited-ink=true hidden-hits=true same-child=true closed=true");
}
#[test]
fn javascript_grid_real_hwnd_self_normal_stretches_instead_of_inheriting_center() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let source = r#"NeuroSurface.define({mount({root}){root.style.cssText='display:grid;grid-template-columns:100px 100px;grid-template-rows:100px;place-items:center';root.innerHTML='<div></div><div></div>';root.children[0].style.placeSelf='normal';root.children[1].style.placeSelf='auto';}});"#;
    let (session, frame) = Session::mount(
        fixture::binding(),
        source.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    let first = frame.scene.root().children()[0].id().to_owned();
    let second = frame.scene.root().children()[1].id().to_owned();
    let window = SurfaceWindow::new(frame.scene, 216, 140).unwrap();
    assert_eq!(geometry(&window, &first), Some(([0., 0.], [100., 100.])));
    let (origin, size) = geometry(&window, &second).unwrap();
    assert!(origin[0] > 100. && origin[1] > 0. && size[0] < 100. && size[1] < 100.);
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
    assert!(!window.is_alive());
}
#[test]
fn javascript_grid_real_hwnd_hidden_items_do_not_reserve_auto_cells() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let source = r#"NeuroSurface.define({mount({root}){root.style.cssText='display:grid;grid-template-columns:40px 40px;gap:4px 8px';root.innerHTML='<button>hide</button><button>two</button><button>three</button>';root.firstChild.style.display='none';}});"#;
    let (session, frame) = Session::mount(
        fixture::binding(),
        source.into(),
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
    assert!(geometry(&window, &ids[0]).is_none());
    assert_eq!(geometry(&window, &ids[1]).unwrap().0, [0., 0.]);
    assert_eq!(geometry(&window, &ids[2]).unwrap().0, [48., 0.]);
    // 语法合法但原生不支持的依赖轴/负隐式轨道必须整次拒绝，保持旧几何与命中。
    for extra in [
        "root.style.gridTemplateRows='20%'",
        "root.firstChild.style.cssText='display:flex;grid-column:-4'",
    ] {
        let source = format!("NeuroSurface.define({{mount({{root}}){{root.style.cssText='display:grid;grid-template-columns:40px 40px';root.innerHTML='<div>x</div>';{extra};}}}});");
        let (bad, frame) = Session::mount(
            fixture::binding(),
            source,
            fixture::snapshot(3, 42),
            fixture::resources(),
            &|| false,
        )
        .unwrap();
        assert!(window.replace(frame.scene).is_err());
        assert_eq!(geometry(&window, &ids[2]).unwrap().0, [48., 0.]);
        bad.dispose(&|| false).unwrap();
    }
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
    assert!(!window.is_alive());
}

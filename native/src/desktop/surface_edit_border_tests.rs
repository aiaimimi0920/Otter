//! 真实 EDIT 四边 HWND 投影/非对称内角 region；更新不重建 HWND、不丢 draft。
use super::*;
fn scene(updated: bool) -> Scene {
    // 只改变祖先计算字体；字段自身合同不变，才能合法继承 HWND/draft。
    let field = serde_json::json!({"id":"field","type":"textarea","props":{"value":"source"},
        "events":{"input":"typing"},"layout":{"width":100,"height":64,"padding":3},
        "style":{"borderLeftWidth":"1.125em","borderTopWidth":"0.125em","borderRightWidth":"0.375em",
        "borderBottomWidth":"0.75em","borderRadius":24}});
    Scene::parse(
        serde_json::json!({"id":"root","type":"column","style":{"fontSize":if updated {8} else {16}},"children":[field]})
        .to_string()
        .as_bytes(),
    )
    .unwrap()
}
#[test]
fn surface_edit_border_sides_real_hwnd_projection_corner_region_update_and_draft() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = SurfaceWindow::new(scene(false), 240, 160).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    let edit = child(&window);
    let check = |updated| unsafe {
        let scale = windows::Win32::UI::HiDpi::GetDpiForWindow(window.hwnd) as f32 / 96.;
        let mut bounds = RECT::default();
        GetWindowRect(edit, &mut bounds).unwrap();
        let mut origin = POINT {
            x: bounds.left,
            y: bounds.top,
        };
        ScreenToClient(window.hwnd, &mut origin).ok().unwrap();
        let [left, top, right, bottom] = if updated {
            [9., 1., 3., 6.]
        } else {
            [18., 2., 6., 12.]
        };
        assert_eq!(
            [origin.x, origin.y],
            [left, top].map(|n| (n * scale).ceil() as i32)
        );
        assert_eq!(
            bounds.right - bounds.left,
            (100. * scale).round() as i32 - ((left * scale).ceil() + (right * scale).ceil()) as i32
        );
        assert_eq!(
            bounds.bottom - bounds.top,
            (64. * scale).round() as i32 - ((top * scale).ceil() + (bottom * scale).ceil()) as i32
        );
        let region = CreateRectRgn(0, 0, 0, 0);
        assert_ne!(GetWindowRgn(edit, region), RGN_ERROR);
        let at = |x: f32, y: f32| {
            PtInRegion(
                region,
                (x * scale).round() as i32,
                (y * scale).round() as i32,
            )
            .as_bool()
        };
        assert!(!at(0., 0.));
        assert!(at(30., 25.));
        if !updated {
            assert!(
                at(3., 5.),
                "narrow left inner ellipse must include this point"
            );
            assert!(
                !at(73., 5.),
                "wide right inner ellipse must exclude its mirror"
            );
        }
        let _ = DeleteObject(region.into());
        let mut format = RECT::default();
        SendMessageW(
            edit,
            EM_GETRECT,
            None,
            Some(LPARAM((&mut format as *mut RECT) as isize)),
        );
        assert_eq!(format.left, (3. * scale).round() as i32);
        assert_eq!(format.top, (3. * scale).round() as i32);
    };
    check(false);
    type_text(edit, "保留 draft");
    window.replace(scene(true)).unwrap();
    assert_eq!(child(&window), edit);
    assert_eq!(text(edit), "保留 draft");
    check(true);
    window.replace(scene(false)).unwrap();
    assert_eq!(child(&window), edit);
    check(false);
    assert_eq!(window.prepare_edit(&binding(), 1).unwrap().len(), 1);
    assert!(window.take_error().is_none());
    window.close().unwrap();
    assert!(!window.is_alive());
}

//! 真实 EDIT 字体、继承、DPI 和释放验证；不声称跨显示器人工验收。
use super::*;
unsafe fn current(hwnd: HWND) -> HGDIOBJ {
    HGDIOBJ(SendMessageW(hwnd, WM_GETFONT, None, None).0 as *mut _)
}
unsafe fn read(font: HGDIOBJ) -> Option<LOGFONTW> {
    let mut value = LOGFONTW::default();
    (GetObjectW(
        font,
        std::mem::size_of::<LOGFONTW>() as i32,
        Some((&mut value as *mut LOGFONTW).cast()),
    ) != 0)
        .then_some(value)
}
fn styled(kind: &str, size: u32) -> Scene {
    Scene::parse(serde_json::json!({"id":"root","type":"column","style":{"fontSize":size,"fontWeight":700},
        "children":[{"id":"field","type":kind,"props":{"value":"source","placeholder":"提示"},
        "style":{"fontSize":"150%"},"layout":{"width":180,"height":80},"events":{"input":"typing"}}]})
        .to_string().as_bytes()).unwrap()
}
#[test]
fn surface_edit_font_mount_uses_resolved_inheritance_and_releases_on_removal() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for kind in ["input", "textarea", "number"] {
        let window = SurfaceWindow::new(styled(kind, 20), 240, 160).unwrap();
        window.bind_edits(&binding()).unwrap();
        window.set_input_policy(true, true).unwrap();
        let edit = child(&window);
        unsafe {
            let font = current(edit);
            let actual = read(font).unwrap();
            let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(window.hwnd);
            assert_eq!(actual.lfHeight, -((30. * dpi as f32 / 96.).round() as i32));
            assert_eq!(actual.lfWeight, 700);
            type_text(edit, "42");
            assert!(window.has_pending_edit());
            window.replace(styled(kind, 24)).unwrap();
            let new_font = current(child(&window));
            assert_eq!(
                read(new_font).unwrap().lfHeight,
                -((36. * dpi as f32 / 96.).round() as i32)
            );
            // 祖先字号变化不改字段合同；同一 EDIT 与未提交草稿保留。
            assert_eq!(child(&window), edit);
            assert_eq!(text(edit), "42");
            assert!(read(font).is_none());
            assert_eq!(window.prepare_edit(&binding(), 1).unwrap().len(), 1);
            window.close().unwrap();
            assert!(read(new_font).is_none());
            assert!(read(GetStockObject(DEFAULT_GUI_FONT)).is_some());
        }
    }
}
#[test]
fn surface_edit_font_dpi_roundtrip_keeps_text_selection_and_reuses_handles() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    let control = crate::desktop::surface_edit_control::Control::new(
        window.hwnd,
        RECT {
            left: 0,
            top: 0,
            right: 180,
            bottom: 80,
        },
        true,
        100,
    )
    .unwrap();
    control.set_text("中文🙂").unwrap();
    control.select((1, 3)).unwrap();
    let style = crate::surface_presentation::ControlFont {
        family: "Segoe UI".into(),
        size: 20.,
        weight: 700,
    };
    let hwnd = control.hwnd().unwrap();
    unsafe {
        for dpi in [96, 144, 192, 288, 96] {
            control.set_font(dpi, &style).unwrap();
            let font = current(hwnd);
            assert_eq!(read(font).unwrap().lfHeight, -(20 * dpi as i32 / 96));
            control.set_font(dpi, &style).unwrap();
            assert_eq!(current(hwnd), font);
            assert_eq!(control.text().unwrap(), "中文🙂");
            assert_eq!(control.selection().unwrap(), (1, 3));
        }
        let font = current(hwnd);
        assert!(control.set_font(0, &style).is_err());
        assert_eq!(current(hwnd), font);
        window.close().unwrap();
        assert!(read(font).is_none());
    }
}
#[test]
fn font_family_edit_native_descriptor_inherits_selection_and_replaces_without_losing_draft() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = |family| {
        Scene::parse(&serde_json::to_vec(&serde_json::json!({
        "id":"root","type":"column","style":{"fontFamily":family},
        "children":[{"id":"field","type":"input","props":{"value":"original"},"events":{"input":"typing"}}]
    })).unwrap()).unwrap()
    };
    let window = SurfaceWindow::new(scene("MissingOtterFamily,monospace"), 240, 160).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    let edit = child(&window);
    unsafe {
        let name = |font| {
            let value = read(font).unwrap();
            let end = value.lfFaceName.iter().position(|u| *u == 0).unwrap();
            String::from_utf16(&value.lfFaceName[..end]).unwrap()
        };
        assert_eq!(name(current(edit)), "Consolas");
        type_text(edit, "draft");
        window.replace(scene("Segoe UI,Consolas")).unwrap();
        assert_eq!(child(&window), edit);
        assert_eq!(name(current(edit)), "Segoe UI");
        assert_eq!(text(edit), "draft");
        let font = current(edit);
        window.replace(scene("Segoe UI,Consolas")).unwrap();
        assert_eq!(current(edit), font);
        window.close().unwrap();
        assert!(read(font).is_none());
    }
}

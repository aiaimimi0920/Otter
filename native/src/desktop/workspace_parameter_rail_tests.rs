//! Actual Application -> attached multiline controls -> local scrolling; owned-message tests are not ordinary-desktop appearance acceptance.
use super::*;
use windows::Win32::{Graphics::Gdi::*, UI::Input::KeyboardAndMouse::*};
#[path = "workspace_chrome_backdrop_tests.rs"]
mod backdrops;
unsafe fn app() -> Application {
    let mut app = super::super::tests::app();
    let (_listener, identity, _) = crate::loom_bridge_client::tests::fixture();
    let mut value = crate::loom_handshake::tests::fixture();
    value["capabilities"]["artDefinitions"][0]["inputs"] = serde_json::json!([
        {"name":"raw","type":"image"},{"name":"dup","type":"string"},{"name":"image","type":"image"}
    ]);
    let mut params = vec![
        serde_json::json!({"id":"dup","type":"string","default":"same"}),
        serde_json::json!({"id":"image","widget":"image_link","default":""}),
    ];
    params.extend(
        (0..24).map(|i| serde_json::json!({"id":format!("p{i}"),"type":"string","default":"v"})),
    );
    value["capabilities"]["artDefinitions"][0]["parameters"] = params.into();
    let handshake = crate::loom_handshake::response(&serde_json::to_vec(&value).unwrap())
        .unwrap()
        .unwrap();
    *app.documents[0].state.selected_art.borrow_mut() =
        Some(desktop::capability_selection::Selection {
            execution: Default::default(),
            identity: desktop::document_state::new_unit_id().unwrap(),
            parameters: crate::loom_parameter_state::Parameters::new(
                &handshake.capability(0).unwrap(),
            )
            .unwrap(),
            catalog: Rc::new(desktop::capability_job::Loaded {
                identity,
                handshake,
            }),
        });
    SetWindowPos(
        app.documents[0].window.0,
        None,
        500,
        120,
        240,
        180,
        SWP_NOACTIVATE | SWP_NOZORDER,
    )
    .unwrap();
    let _ = ShowWindow(app.documents[0].window.0, SW_SHOWNOACTIVATE);
    app.sync_link_menus();
    app.documents[0].state.parameters_ui.toggle.set(true);
    app.process_parameter_panels(true);
    app.sync_workspace_chrome(true);
    assert!(app.chrome.ui.failure.borrow().is_none());
    app
}
fn port(app: &Application, id: &str) -> Port {
    Port {
        node: 0,
        side: Side::Input,
        index: app
            .chrome
            .ui
            .scene
            .borrow()
            .as_ref()
            .unwrap()
            .catalog
            .endpoints[0]
            .inputs
            .iter()
            .rposition(|name| name == id)
            .unwrap(),
    }
}
fn lp(x: i32, y: i32) -> LPARAM {
    LPARAM(x as u16 as isize | ((y as u16 as isize) << 16))
}
unsafe fn strip(app: &Application) -> HWND {
    app.chrome
        .ui
        .input_windows
        .borrow()
        .iter()
        .find(|(_, n, s)| *n == 0 && *s == Side::Input)
        .unwrap()
        .0
}
#[test]
fn real_tab_is_child_only_and_fixed_inputs_do_not_scroll_with_public_parameters() {
    unsafe {
        let mut app = app();
        let ui = &app.chrome.ui;
        let entry = ui.scene.borrow().as_ref().unwrap().catalog.endpoints[0]
            .inputs
            .clone();
        assert_eq!(&entry[..2], ["raw", "image"]);
        assert_eq!(entry.iter().filter(|name| *name == "dup").count(), 1);
        assert_eq!(
            entry.iter().filter(|name| *name == "image").count(),
            2,
            "primary image has an explicit port and a parameter row"
        );
        let panel = app.documents[0].parameters.as_ref().unwrap().handle();
        assert_eq!(GetParent(panel).unwrap(), strip(&app));
        assert_eq!(GetWindowLongW(panel, GWL_STYLE) as u32 & WS_CAPTION.0, 0);
        let fixed = port_visual::layout(
            ui,
            Port {
                node: 0,
                side: Side::Input,
                index: 0,
            },
        )
        .unwrap();
        let row = port_visual::unclipped(ui, port(&app, "p0")).unwrap();
        let rail = layout(ui, &ui.node(0).unwrap()).unwrap();
        let point = (rail.viewport.x + 120, rail.viewport.y + 40);
        SendMessageW(
            strip(&app),
            WM_MOUSEWHEEL,
            Some(WPARAM(((-30i16) as u16 as usize) << 16)),
            Some(lp(point.0, point.1)),
        );
        assert_eq!(
            port_visual::layout(
                ui,
                Port {
                    node: 0,
                    side: Side::Input,
                    index: 0
                }
            )
            .unwrap(),
            fixed
        );
        assert_eq!(
            port_visual::unclipped(ui, port(&app, "p0"))
                .unwrap()
                .frame
                .y,
            row.frame.y - 12
        );
        assert_eq!(
            app.documents[0].parameters.as_ref().unwrap().handle(),
            panel
        );
        assert!(port_visual::layout(ui, port(&app, "p23")).is_none());
        assert_eq!(app.documents[0].state.parameters_ui.window(), panel);
        assert_eq!(
            ui.rails.borrow().len(),
            1,
            "all rows share the document owner"
        );
        app.shutdown();
    }
}
#[test]
fn scroll_reveal_tab_roundtrip_and_frozen_thumb_drag_keep_the_same_field_windows() {
    unsafe {
        let mut app = app();
        let host = app.documents[0].window.0;
        let panel = app.documents[0].parameters.as_ref().unwrap().handle();
        let child = GetWindow(panel, GW_CHILD).unwrap();
        desktop::application::chrome::reveal_parameter(&app.documents[0].state.chrome, host, "p23");
        assert!(port_visual::layout(&app.chrome.ui, port(&app, "p23")).is_some());
        assert!(IsWindow(Some(child)).as_bool());
        let rail = layout(&app.chrome.ui, &app.chrome.ui.node(0).unwrap()).unwrap();
        assert!(rail.offset > 0.0);
        let saved = rail.offset;
        for _ in 0..2 {
            app.documents[0].state.parameters_ui.toggle.set(true);
            app.process_parameter_panels(true);
            app.sync_workspace_chrome(true);
        }
        assert_eq!(
            layout(&app.chrome.ui, &app.chrome.ui.node(0).unwrap())
                .unwrap()
                .offset,
            saved
        );
        let rail = layout(&app.chrome.ui, &app.chrome.ui.node(0).unwrap()).unwrap();
        let thumb = rail.thumb().unwrap();
        let parent = strip(&app);
        let mut p = POINT {
            x: thumb.x + thumb.width / 2,
            y: thumb.y + thumb.height / 2,
        };
        ScreenToClient(parent, &mut p).ok().unwrap();
        SendMessageW(parent, WM_LBUTTONDOWN, None, Some(lp(p.x, p.y)));
        assert!(app.chrome.ui.rail_drag.get().is_some());
        assert!(!app.chrome.idle());
        SendMessageW(parent, WM_MOUSEMOVE, None, Some(lp(p.x, p.y - 20)));
        SendMessageW(parent, WM_LBUTTONUP, None, Some(lp(p.x, p.y - 20)));
        assert!(app.chrome.ui.rail_drag.get().is_none());
        assert_ne!(GetCapture(), parent);
        assert!(
            layout(&app.chrome.ui, &app.chrome.ui.node(0).unwrap())
                .unwrap()
                .offset
                < saved
        );
        app.shutdown();
    }
}

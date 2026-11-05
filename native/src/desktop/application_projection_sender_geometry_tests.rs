//! 无 raster 目标的实时逻辑框：独立像素 oracle 与隐藏菜单/TLS，不等待保存或占用前台。
use super::*;
use crate::{crop_geometry::Size, session_window::Rect};

unsafe fn resize(app: &Application, width: f64, height: f64) {
    let frame = app
        .restored_session
        .as_ref()
        .unwrap()
        .space
        .unwrap()
        .physical(Rect {
            x: 100.0,
            y: 100.0,
            w: width,
            h: height,
        })
        .unwrap();
    SetWindowPos(
        app.documents[0].window.0,
        None,
        frame.x,
        frame.y,
        frame.width as i32,
        frame.height as i32,
        SWP_NOACTIVATE | SWP_NOZORDER,
    )
    .unwrap();
    let mut actual = RECT::default();
    GetWindowRect(app.documents[0].window.0, &mut actual).unwrap();
    assert_eq!(
        (actual.right - actual.left, actual.bottom - actual.top),
        (frame.width as i32, frame.height as i32)
    );
}
unsafe fn mini(app: &Application, expanded: Size) {
    use crate::session_minified::{View, Views};
    let space = app.restored_session.as_ref().unwrap().space.unwrap();
    let mut rect = RECT::default();
    GetWindowRect(app.documents[0].window.0, &mut rect).unwrap();
    let initial = crate::pin_geometry::PinRect {
        x: rect.left,
        y: rect.top,
        width: (rect.right - rect.left) as u32,
        height: (rect.bottom - rect.top) as u32,
    };
    let current = space.logical(initial, None).unwrap();
    let mut canvas = app.documents[0].state.canvas.borrow_mut();
    canvas.minified_restore = Some(crate::minified_transition::Transition {
        initial,
        expanded: space
            .physical(Rect {
                w: expanded.width,
                h: expanded.height,
                ..current
            })
            .unwrap(),
        opacity: canvas.normal_opacity,
        persist_expanded: true,
    });
    let view = View {
        size: expanded,
        offset: (0.0, 0.0),
    };
    canvas.minified = Some(
        crate::desktop::canvas::minified::Presentation::restored(
            None,
            Views {
                image: view,
                annotations: view,
            },
            Size {
                width: current.w,
                height: current.h,
            },
            &canvas.editor,
        )
        .unwrap(),
    );
}
pub(super) unsafe fn install(app: &mut Application, case: source::Case) -> Image {
    app.restored_session.as_mut().unwrap().space = Some(crate::session_placement::Space {
        origin: (-120, -60),
        scale: 1.5,
    });
    app.documents[0].state.canvas.borrow_mut().editor = crate::editor::Editor::default();
    resize(app, 64.0, 32.0);
    let canonical = app
        .workspace_checkpoint
        .as_ref()
        .unwrap()
        .sender_source()
        .unwrap()
        .clone();
    let mut value: serde_json::Value = serde_json::from_slice(&canonical).unwrap();
    value["stickers"][0]["imageEditState"] =
        serde_json::json!({"borderWidth":2,"borderColor":"#ff0000"});
    if matches!(case, source::Case::ArtLive) {
        value["stickers"][0]["type"] = "art".into();
        app.documents[0].state.restored_art = true;
    }
    app.restored_session
        .as_mut()
        .unwrap()
        .edit_original(serde_json::to_vec(&value).unwrap().into());
    settle(app);
    let canonical = app
        .workspace_checkpoint
        .as_ref()
        .unwrap()
        .sender_source()
        .unwrap()
        .clone();
    let before: serde_json::Value = serde_json::from_slice(&canonical).unwrap();
    assert_eq!(before["stickers"][0]["w"], 64.0);
    assert_eq!(before["stickers"][0]["h"], 32.0);
    resize(app, 32.0, 16.0);
    if matches!(case, source::Case::TargetMini | source::Case::ArtLive) {
        // 当前 expanded 与 canonical/mini HWND 三者均不同；Art 必须忽略 sticker savedRect 分支。
        mini(
            app,
            Size {
                width: 16.0,
                height: 8.0,
            },
        );
    }
    let image = Image {
        width: 128,
        height: 64,
        pixels: vec![255; 128 * 64 * 4].into(),
    };
    *app.documents[0].state.resolved_display.borrow_mut() =
        Some(crate::desktop::display_frame::Frame::capture(
            crate::workspace_id::UnitId::parse("upstream").unwrap(),
            image,
            None,
        ));
    assert!(
        app.workspace_checkpoint
            .as_ref()
            .unwrap()
            .sender_source()
            .unwrap()
            .same_storage(&canonical),
        "fixture must not save the live resize before sending"
    );
    let border = if matches!(case, source::Case::TargetMini) {
        16
    } else {
        8
    };
    let mut pixels = vec![255; 128 * 64 * 4];
    for y in 0..64 {
        for x in 0..128 {
            if x < border || x >= 128 - border || y < border || y >= 64 - border {
                pixels[(y * 128 + x) * 4..(y * 128 + x) * 4 + 4].copy_from_slice(&[0, 0, 255, 255]);
            }
        }
    }
    Image {
        width: 128,
        height: 64,
        pixels: pixels.into(),
    }
}
#[test]
fn native_sender_no_raster_unsaved_resize_uses_live_session_logical_frame() {
    tls::run_source(0, source::Case::TargetLive);
}
#[test]
fn native_sender_no_raster_minified_uses_current_expanded_frame_not_canonical_or_mini() {
    tls::run_source(0, source::Case::TargetMini);
}
#[test]
fn native_sender_no_raster_art_ignores_sticker_expanded_frame() {
    tls::run_source(0, source::Case::ArtLive);
}
#[test]
fn native_sender_no_raster_scope_rejects_resize_minify_and_changed_session_space() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (files, _, _) = Files::new();
    unsafe {
        let (_class, mut app) = application(files.root());
        drop(install(&mut app, source::Case::TargetLive));
        let unit = app.documents[0].state.unit_id.clone();
        let (guard, _) = scope::Scope::capture(&app, &unit).unwrap();
        assert!(guard.matches(&app));
        resize(&app, 40.0, 20.0);
        assert!(
            !guard.matches(&app),
            "HWND resize without editor revision is stale"
        );
        resize(&app, 32.0, 16.0);
        // resize 会改变 viewport；恢复同一 HWND 外框不表示旧完整 Document 凭证复活。
        let (guard, _) = scope::Scope::capture(&app, &unit).unwrap();
        assert!(guard.matches(&app));
        mini(
            &app,
            Size {
                width: 16.0,
                height: 8.0,
            },
        );
        assert!(
            !guard.matches(&app),
            "minify without editor revision is stale"
        );
        let (mini_guard, _) = scope::Scope::capture(&app, &unit).unwrap();
        mini(
            &app,
            Size {
                width: 20.0,
                height: 10.0,
            },
        );
        assert!(
            !mini_guard.matches(&app),
            "expanded view changed while HWND stayed equal"
        );
        app.documents[0].state.canvas.borrow_mut().minified = None;
        app.documents[0].state.canvas.borrow_mut().minified_restore = None;
        assert!(guard.matches(&app));
        app.restored_session
            .as_mut()
            .unwrap()
            .space
            .as_mut()
            .unwrap()
            .scale = 2.0;
        assert!(
            !guard.matches(&app),
            "session scale cannot change beneath physical snapshot"
        );
        app.restored_session
            .as_mut()
            .unwrap()
            .space
            .as_mut()
            .unwrap()
            .scale = 1.5;
        assert!(guard.matches(&app));
        app.restored_session
            .as_mut()
            .unwrap()
            .space
            .as_mut()
            .unwrap()
            .origin
            .0 += 1;
        assert!(
            !guard.matches(&app),
            "session origin is part of physical snapshot ownership"
        );
        app.restored_session.as_mut().unwrap().space = None;
        assert!(!guard.matches(&app));
        assert!(
            scope::Scope::capture(&app, &unit).is_err(),
            "no monitor DPI fallback for missing Space"
        );
        app.shutdown();
    }
}
#[test]
fn native_sender_no_raster_rejects_already_stale_minified_presentation() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (files, _, _) = Files::new();
    unsafe {
        let (_class, mut app) = application(files.root());
        drop(install(&mut app, source::Case::TargetMini));
        app.documents[0].state.canvas.borrow_mut().editor = crate::editor::Editor::default();
        let unit = app.documents[0].state.unit_id.clone();
        assert!(scope::Scope::capture(&app, &unit).is_err());
        app.shutdown();
    }
}

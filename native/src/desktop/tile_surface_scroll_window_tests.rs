//! 滚动/Tab使用自有离屏HWND；预期几何与表单值是独立字面量，不从新实现反算。
use super::*;
use crate::desktop::tile_window::scroll;
fn scrolling_scene(kind: &str) -> Value {
    json!({"id":"root","type":"scroll","layout":{"width":64,"height":64},"children":[
        {"id":"first","type":kind,"layout":{"width":64,"height":20,"minHeight":20},"props":{"value":"source"}},
        {"id":"gap","type":"view","layout":{"height":80,"minHeight":80}},
        {"id":"send","type":"button","layout":{"width":64,"height":20,"minHeight":20},
        "props":{"includeSurfaceValues":true},"events":{"click":"go"}}]})
}
fn scroll(w: &Window, delta: f32) -> bool {
    scroll::wheel(w.hwnd, &w.state, [8, 8], 1, delta, false).unwrap()
}
fn command(f: &Fixture) -> crate::tile_surface_actions::Command {
    f.shared.lock().actions.pending.pop_front().unwrap()
}
#[test]
fn tab_reveals_offscreen_button_and_reverse_preserves_actual_edit_hwnd_and_form_draft() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (f, w) = setup(scrolling_scene("input"));
    navigate(w.hwnd, &w.state, false);
    let h = text::active_child(&w.state);
    unsafe {
        SendMessageW(h, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
        let value = windows::core::HSTRING::from("滚动🙂草稿");
        SendMessageW(
            h,
            EM_REPLACESEL,
            Some(WPARAM(1)),
            Some(LPARAM(value.as_ptr() as isize)),
        );
        key(h, WM_KEYDOWN, 9, false);
    }
    assert_eq!(focus(&w), "send");
    assert_eq!(
        w.state.keyboard.borrow().focus.as_ref().unwrap().rect,
        [0, 44, 64, 64]
    );
    unsafe {
        key(w.hwnd, WM_KEYDOWN, 13, false);
    }
    let mut c = command(&f);
    let Action::Event(e) = &c.kind else { panic!() };
    assert_eq!(e["nodeId"], "send");
    assert_eq!(e["payload"]["surfaceValues"]["first"], "滚动🙂草稿");
    assert!(crate::tile_surface_action_dispatch::validate(
        &mut c,
        &f.state,
        Some(&f.art),
        &f.job,
        &f.shared
    ));
    navigate(w.hwnd, &w.state, true);
    assert_eq!(focus(&w), "first");
    assert_eq!(text::active_child(&w.state), h);
    assert_eq!(
        w.state.keyboard.borrow().focus.as_ref().unwrap().rect,
        [0, 0, 64, 20]
    );
    assert_eq!(
        f.art.snapshot.scene,
        serde_json::to_vec(&scrolling_scene("input")).unwrap()
    );
}
#[test]
fn wheel_repaints_then_click_dispatch_uses_saved_scroll_and_old_press_does_not_retarget() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (f, w) = setup(scrolling_scene("button"));
    unsafe {
        SendMessageW(w.hwnd, WM_LBUTTONDOWN, None, Some(LPARAM(8 | (8 << 16))));
    }
    assert!(scroll(&w, 1000.));
    assert!(w.state.painted.get());
    assert!(actions::active(&w.state).is_some());
    unsafe {
        SendMessageW(w.hwnd, WM_LBUTTONUP, None, Some(LPARAM(8 | (52 << 16))));
    }
    assert!(events(&f).is_empty());
    assert!(w.test_art_click([8, 52]));
    let mut c = command(&f);
    assert!(crate::tile_surface_action_dispatch::validate(
        &mut c,
        &f.state,
        Some(&f.art),
        &f.job,
        &f.shared
    ));
    c.anchor.scroll = Default::default();
    assert!(!crate::tile_surface_action_dispatch::validate(
        &mut c,
        &f.state,
        Some(&f.art),
        &f.job,
        &f.shared
    ));
    assert!(!scroll(&w, 1000.), "boundary does not claim movement");
    assert!(scroll(&w, -1000.));
}
#[test]
fn ime_rejects_scroll_or_reveal_without_losing_current_focus_hwnd_or_draft() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (f, w) = setup(scrolling_scene("input"));
    navigate(w.hwnd, &w.state, false);
    let h = text::active_child(&w.state);
    unsafe {
        SendMessageW(h, WM_IME_STARTCOMPOSITION, None, None);
    }
    assert!(!scroll(&w, 40.));
    navigate(w.hwnd, &w.state, false);
    assert_eq!(focus(&w), "first");
    assert_eq!(text::active_child(&w.state), h);
    assert_eq!(
        w.state.keyboard.borrow().focus.as_ref().unwrap().rect,
        [0, 0, 64, 20]
    );
    assert!(events(&f).is_empty());
    unsafe {
        SendMessageW(h, WM_IME_ENDCOMPOSITION, None, None);
    }
    navigate(w.hwnd, &w.state, false);
    assert_eq!(focus(&w), "send");
}
#[test]
fn handlerless_value_survives_hidden_scroll_and_does_not_gain_network_action() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let mut scene = scrolling_scene("input");
    // Switch沿既有label/span合同忽略自身layout；由固定行容器承载，不能靠无效minHeight防收缩。
    scene["children"][0] = json!({"id":"row","type":"view","layout":{"width":64,"height":20,"minHeight":20},
        "children":[{"id":"first","type":"switch","props":{"value":false}}]});
    let (f, w) = setup(scene);
    let initial = local::focus(&w.state, [8, 8]).expect("initial switch hit");
    assert_eq!(initial.node, "first");
    assert_eq!(initial.rect, [0, 0, 64, 19]);
    assert_eq!(
        local::value(&w.state, &initial),
        Some(crate::tile_surface_local::Value::Switch(false))
    );
    assert!(!w.test_art_click([8, 8]));
    assert_eq!(
        w.state.local.borrow().snapshot().len(),
        1,
        "{}",
        f.shared.lock().actions.notice
    );
    assert_eq!(w.test_art_local_values()["first"], true);
    assert!(scroll(&w, 10.));
    w.tick().unwrap();
    assert_eq!(w.test_art_local_values()["first"], true);
    assert_eq!(local::focus(&w.state, [8, 4]).unwrap().rect, [0, 0, 64, 9]);
    assert!(!w.test_art_click([8, 4]));
    assert_eq!(w.test_art_local_values()["first"], false);
    assert!(!w.test_art_click([8, 4]));
    assert_eq!(w.test_art_local_values()["first"], true);
    assert!(scroll(&w, 1000.));
    w.tick().unwrap();
    assert_eq!(w.test_art_local_values()["first"], true);
    assert!(scroll(&w, -1000.));
    assert_eq!(w.test_art_local_values()["first"], true);
    assert!(events(&f).is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&f.art.snapshot.scene).unwrap()["children"][0]["children"]
            [0]["props"]["value"],
        false
    );
}
#[test]
fn ctrl_wheel_hidden_frozen_expired_and_preparation_gap_do_not_scroll() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for case in 0..5 {
        let (f, w) = setup(scrolling_scene("input"));
        match case {
            0 => unsafe {
                let _ = ShowWindow(w.hwnd, SW_HIDE);
            },
            1 => f.shared.lock().frozen = true,
            2 => {
                let until = f.shared.lock().until.unwrap();
                f.shared.lock().expire(until);
            }
            3 => f.shared.lock().presented_frame = None,
            _ => {
                let mut point = POINT { x: 8, y: 8 };
                unsafe {
                    assert!(ClientToScreen(w.hwnd, &mut point).as_bool());
                    let lp = LPARAM(point.x as u16 as isize | ((point.y as u16 as isize) << 16));
                    assert!(!scroll::message(
                        w.hwnd,
                        &w.state,
                        WM_MOUSEWHEEL,
                        WPARAM(((-120i16) as u16 as usize) << 16 | 8),
                        lp
                    ));
                }
                continue;
            }
        }
        if case != 0 {
            assert!(!scroll(&w, 40.));
        } else {
            assert!(!unsafe {
                scroll::message(
                    w.hwnd,
                    &w.state,
                    WM_MOUSEWHEEL,
                    WPARAM(120 << 16),
                    LPARAM(0),
                )
            });
        }
        assert!(events(&f).is_empty());
    }
}
#[test]
fn source_crop_and_each_output_rotation_keep_revealed_target_and_fresh_dispatch() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for rotation in ["deg0", "deg90", "deg180", "deg270"] {
        let (mut f, w) = setup(scrolling_scene("input"));
        let mut frame = Frame {
            key: f.frame.key.clone(),
            layout: f.frame.layout.clone(),
            size: f.frame.size,
            images: f.frame.images.clone(),
            arts: f.frame.arts.clone(),
            live_until: None,
        };
        frame.layout.tiles[0].rotation = serde_json::from_value(json!(rotation)).unwrap();
        frame.layout.placements[0].source_crop.y = 0.5;
        frame.layout.placements[0].source_crop.height = 0.5;
        f.job.layout = frame.layout.clone();
        f.shared.lock().frame = Some(Arc::new(frame));
        w.tick().unwrap();
        navigate(w.hwnd, &w.state, false);
        assert_eq!(focus(&w), "send", "{rotation}");
        unsafe {
            key(w.hwnd, WM_KEYDOWN, 13, false);
        }
        let mut c = command(&f);
        assert!(
            crate::tile_surface_action_dispatch::validate(
                &mut c,
                &f.state,
                Some(&f.art),
                &f.job,
                &f.shared
            ),
            "{rotation}"
        );
    }
}
#[test]
fn snapshot_preparation_gap_preserves_revealed_focus_draft_and_queued_form_suffix() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let mut scene = scrolling_scene("input");
    scene["children"][0]["events"] = json!({"input":"preview","change":"save"});
    let (f, w) = setup(scene.clone());
    w.test_art_text([8, 8], "滚动🙂草稿", true, false);
    assert!(w.test_art_text_keyboard_submit());
    assert_eq!(focus(&w), "send");
    assert_eq!(events(&f).len(), 3);
    scene["children"][0]["props"]["value"] = json!("滚动🙂草稿");
    let mut dto = crate::tile_surface_actions::tests::value(&f.frame.key, "art");
    dto["snapshot"]["scene"] = scene;
    dto["snapshot"]["revision"] = json!(2);
    dto["sequence"] = json!(1);
    let next = Arc::new(
        crate::tile_surface_wire::State::parse(
            &serde_json::to_vec(&dto).unwrap(),
            &(&f.frame.key).into(),
            "art",
            Some(&f.state),
        )
        .unwrap(),
    );
    let art = Arc::new(crate::tile_surface_owner::Art {
        view: next.view.clone(),
        generation: next.generation,
        snapshot: next.snapshot.clone(),
        size: next.size,
        images: Default::default(),
    });
    let frame = Arc::new(Frame {
        key: f.frame.key.clone(),
        layout: f.frame.layout.clone(),
        size: f.frame.size,
        images: Default::default(),
        arts: [("art".into(), art.clone())].into(),
        live_until: None,
    });
    {
        let mut s = f.shared.lock();
        s.actions
            .observe(next.clone(), "https://localhost", "device", Instant::now());
        s.unavailable(f.job.generation, &f.frame.key, "prepare");
    }
    w.tick().unwrap();
    assert_eq!(focus(&w), "send");
    assert!(actions::active(&w.state).is_none());
    f.shared.lock().frame = Some(frame);
    w.tick().unwrap();
    assert_eq!(focus(&w), "send");
    assert_eq!(
        w.state.keyboard.borrow().focus.as_ref().unwrap().rect,
        [0, 44, 64, 64]
    );
    assert_eq!(events(&f).len(), 3);
    let commands = std::mem::take(&mut f.shared.lock().actions.pending);
    for mut c in commands {
        assert!(c.edit.as_ref().unwrap().active());
        assert!(crate::tile_surface_action_dispatch::validate(
            &mut c,
            &next,
            Some(&art),
            &f.job,
            &f.shared
        ));
        let Action::Event(e) = &c.kind else { panic!() };
        assert_eq!(e["baseRevision"], 2);
        if e["event"] == "click" {
            assert_eq!(e["payload"]["surfaceValues"]["first"], "滚动🙂草稿");
        }
    }
}

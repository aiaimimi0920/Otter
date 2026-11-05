//! 自有离屏HWND/EDIT真实通知；IME只模拟系统消息边沿，不冒充人工输入法验收。
use super::*;
use crate::tile_surface_actions::{tests::Fixture, Kind};
use crate::tile_surface_value::tests::fixture;
use serde_json::{json, Value};
use windows::Win32::UI::Controls::{EM_REPLACESEL, EM_SETSEL};
fn scene(kind: &str) -> Value {
    json!({"id":"field","type":kind,"layout":{"width":64,"height":64},
    "props":{"value":"source"},"events":{"input":"typing","change":"save"}})
}
fn setup(scene: Value) -> (Fixture, Window) {
    let mut f = fixture(scene);
    let frame = Frame {
        size: crate::tile_wire::Size {
            width: 64,
            height: 64,
        },
        key: f.frame.key.clone(),
        layout: f.frame.layout.clone(),
        images: f.frame.images.clone(),
        arts: f.frame.arts.clone(),
        live_until: None,
    };
    f.job.size = frame.size;
    // 夹具重设初始输出，不经同key内容去重（生产输出几何换代由新租约/key负责）。
    {
        let mut s = f.shared.lock();
        s.frame = Some(Arc::new(frame));
        s.presented_frame = None;
    }
    f.frame = f.shared.lock().frame.clone().unwrap();
    let mut output = crate::tile_tests::output();
    (output.x, output.y, output.width, output.height) = (-30000, -30000, 64, 64);
    let window = Window::new(f.shared.clone(), &output).unwrap();
    window.tick().unwrap();
    window.tick().unwrap();
    assert!(window.state.painted.get());
    (f, window)
}
fn edit(w: &Window, pixel: [i32; 2]) -> HWND {
    assert!(text::begin(w.hwnd, &w.state, pixel));
    let o = w.state.text.borrow();
    let o = o.as_ref().unwrap();
    o.fields[o.active.as_ref().expect("active field")]
        .session
        .control()
        .hwnd()
        .unwrap()
}
unsafe fn replace(hwnd: HWND, value: &str) {
    SendMessageW(hwnd, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
    let value = windows::core::HSTRING::from(value);
    SendMessageW(
        hwnd,
        EM_REPLACESEL,
        Some(WPARAM(1)),
        Some(LPARAM(value.as_ptr() as isize)),
    );
}
fn events(f: &Fixture) -> Vec<Value> {
    f.shared
        .lock()
        .actions
        .pending
        .iter()
        .filter_map(|c| match &c.kind {
            Kind::Event(e) => Some(e.clone()),
            _ => None,
        })
        .collect()
}
#[test]
fn unicode_notifications_coalesce_then_enter_commits_once_without_changing_scene() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (f, w) = setup(scene("input"));
    let source = f.art.snapshot.scene.clone();
    let foreground = unsafe { GetForegroundWindow() };
    let h = edit(&w, [4, 4]);
    for i in 0..100 {
        unsafe {
            replace(h, &format!("中文🙂{i}"));
        }
    }
    assert_eq!(events(&f).len(), 1);
    assert_eq!(events(&f)[0]["payload"]["value"], "中文🙂99");
    unsafe {
        SendMessageW(h, WM_KEYDOWN, Some(WPARAM(13)), None);
        SendMessageW(h, WM_KEYDOWN, Some(WPARAM(13)), None);
    }
    let actual = events(&f);
    assert_eq!(actual.len(), 2);
    assert_eq!(actual[1]["event"], "change");
    assert_eq!(actual[1]["payload"]["value"], "中文🙂99");
    assert_eq!(f.art.snapshot.scene, source);
    assert_eq!(unsafe { GetForegroundWindow() }, foreground);
    w.tick().unwrap();
    assert_eq!(w.state.text_preview.borrow()[0].value.as_ref(), "中文🙂99");
}
#[test]
fn composition_suppresses_intermediate_events_and_escape_does_not_close_it() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (f, w) = setup(scene("input"));
    let h = edit(&w, [4, 4]);
    unsafe {
        SendMessageW(h, WM_IME_STARTCOMPOSITION, None, None);
        replace(h, "组合🙂");
        SendMessageW(h, WM_KEYDOWN, Some(WPARAM(27)), None);
    }
    assert!(events(&f).is_empty());
    assert!(w.state.text.borrow().as_ref().unwrap().active.is_some());
    unsafe {
        SendMessageW(h, WM_IME_ENDCOMPOSITION, None, None);
    }
    let e = events(&f);
    assert_eq!(e.len(), 1);
    assert_eq!(e[0]["payload"]["value"], "组合🙂");
    unsafe {
        SendMessageW(h, WM_KEYDOWN, Some(WPARAM(27)), None);
    }
    assert!(w.state.text.borrow().is_none());
    assert!(events(&f).is_empty());
}
#[test]
fn textarea_normalizes_crlf_and_number_rejects_invalid_then_steps_and_commits() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (f, w) = setup(scene("textarea"));
    let h = edit(&w, [4, 4]);
    unsafe {
        replace(h, "第一行\r\n🙂第二行");
        SendMessageW(h, WM_CHAR, Some(WPARAM(13)), None);
    }
    assert!(events(&f).iter().all(|e| e["event"] == "input"));
    assert!(events(&f)[0]["payload"]["value"]
        .as_str()
        .unwrap()
        .starts_with("第一行\n🙂第二行"));
    assert!(text::commit_for_click(&w.state));
    assert_eq!(events(&f).last().unwrap()["event"], "change");
    drop(w);
    let mut n = scene("number");
    n["props"] = json!({"value":"2","min":0,"max":10,"step":2});
    let (f, w) = setup(n);
    let h = edit(&w, [4, 4]);
    unsafe {
        replace(h, "invalid");
    }
    assert!(events(&f).is_empty());
    assert_eq!(
        w.state
            .text
            .borrow()
            .as_ref()
            .unwrap()
            .fields
            .values()
            .next()
            .unwrap()
            .session
            .value(),
        "2"
    );
    unsafe {
        SendMessageW(h, WM_KEYDOWN, Some(WPARAM(38)), None);
    }
    let actual = events(&f);
    assert_eq!(actual.len(), 2);
    assert_eq!(actual[0]["payload"]["value"], 4.0);
    assert_eq!(actual[1]["event"], "change");
}
#[test]
fn stale_popup_epoch_is_ignored_and_hide_dpi_freeze_stop_destroy_release_all_fields() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for case in 0..5 {
        let (f, w) = setup(scene("input"));
        let h = edit(&w, [4, 4]);
        unsafe {
            replace(h, "local");
        }
        let popup = w
            .state
            .text
            .borrow()
            .as_ref()
            .unwrap()
            .popup
            .hwnd()
            .unwrap();
        let epoch =
            unsafe { GetPropW(popup, windows::core::w!("Otter.Tile.Text.Epoch")).0 as usize };
        edit(&w, [4, 4]);
        unsafe {
            SendMessageW(
                w.hwnd,
                text::DISMISS,
                Some(WPARAM(epoch)),
                Some(LPARAM(popup.0 as isize)),
            );
        }
        assert!(w.state.text.borrow().as_ref().unwrap().active.is_some());
        unsafe {
            match case {
                0 => {
                    let _ = ShowWindow(w.hwnd, SW_HIDE);
                }
                1 => {
                    SendMessageW(w.hwnd, WM_DPICHANGED, None, None);
                }
                2 => {
                    f.shared.lock().frozen = true;
                    w.tick().unwrap();
                }
                3 => {
                    f.shared.stop();
                    w.tick().unwrap();
                }
                _ => {
                    DestroyWindow(w.hwnd).unwrap();
                }
            }
        }
        assert!(events(&f).is_empty(), "case {case}");
        assert!(
            w.state
                .text
                .borrow()
                .as_ref()
                .is_none_or(|o| o.fields.is_empty()),
            "case {case}"
        );
        drop(w);
        assert!(!unsafe { IsWindow(Some(h)) }.as_bool());
        assert!(!unsafe { IsWindow(Some(popup)) }.as_bool());
    }
}
#[test]
fn preparation_failure_cancels_all_tickets_of_the_unpublished_batch() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (f, w) = setup(scene("input"));
    edit(&w, [4, 4]);
    text::run(&w.state, |owner| {
        let id = owner.active.clone().unwrap();
        let slot = owner.fields.get_mut(&id).unwrap();
        let s = crate::surface_scene::Scene::parse(&slot.field.anchor.snapshot.scene).unwrap();
        let a = &slot.field.anchor;
        let identity = crate::surface_click_event::Identity {
            instance: &a.view.instance_id,
            attachment: &a.view.attachment_id,
            generation: a.generation,
        };
        for _ in 0..62 {
            slot.session
                .prepare_unbound(
                    s.root(),
                    identity,
                    1,
                    crate::surface_input_event::Phase::Input,
                    Some("held"),
                )
                .unwrap();
        }
        slot.session.control().set_text("local")?;
        slot.capture(true)?;
        assert!(owner.flush_one(&id, &w.state).is_err());
        assert_eq!(owner.fields[&id].session.pending(), 62);
        Ok(())
    });
    assert!(events(&f).is_empty());
}
#[path = "tile_surface_text_form_tests.rs"]
mod forms;

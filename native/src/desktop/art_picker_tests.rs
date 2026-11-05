//! 仅自有隐藏宿主/控件与线程键表；不注入系统输入，不把合成 IME 当人工输入法验收。
use super::*;
use desktop::{document_window::DocumentWindow, Apartment};
use windows::{core::w, Win32::System::LibraryLoader::GetModuleHandleW};
pub(in crate::desktop) fn catalog(mut edit: impl FnMut(&mut serde_json::Value)) -> Rc<Loaded> {
    let (_, identity, _) = crate::loom_bridge_client::tests::fixture();
    let mut raw = crate::loom_handshake::tests::fixture();
    raw["capabilities"]["artDefinitions"][0]["parameters"] =
        serde_json::json!([{"id":"amount","type":"number","default":42}]);
    raw["capabilities"]["artDefinitions"][0]["inputs"] =
        serde_json::json!([{"name":"context","type":"text"},{"name":"pixels","type":"image"}]);
    edit(&mut raw);
    Rc::new(Loaded {
        identity,
        handshake: crate::loom_handshake::response(&serde_json::to_vec(&raw).unwrap())
            .unwrap()
            .unwrap(),
    })
}
pub(in crate::desktop) unsafe fn document() -> (HINSTANCE, DocumentWindow) {
    let instance = HINSTANCE(GetModuleHandleW(None).unwrap().0);
    let registered = RegisterClassW(&WNDCLASSW {
        hInstance: instance,
        lpszClassName: w!("Otter.Native.Workspace"),
        lpfnWndProc: Some(fixture_proc),
        ..Default::default()
    });
    assert!(registered != 0 || GetLastError() == ERROR_CLASS_ALREADY_EXISTS);
    let state = State::new(
        Some(crate::image_decode::Image {
            width: 2,
            height: 2,
            pixels: [10, 20, 30, 255].repeat(4).into(),
        }),
        false,
    )
    .unwrap();
    let document = DocumentWindow::create(instance, state, false).unwrap();
    (instance, document)
}
thread_local! {
    static CREATE: std::cell::RefCell<Option<Box<dyn FnOnce()>>> = const { std::cell::RefCell::new(None) };
    static CANDIDATE: std::cell::Cell<HWND> = const { std::cell::Cell::new(HWND(std::ptr::null_mut())) };
}
pub(in crate::desktop) fn replace_during_create(
    catalog: Rc<std::cell::RefCell<desktop::capability_catalog::Catalog>>,
) {
    during_create(move || catalog.borrow_mut().loaded = Some(self::catalog(|_| {})));
}
pub(in crate::desktop) fn during_create(action: impl FnOnce() + 'static) {
    CREATE.with(|slot| {
        assert!(slot.borrow().is_none());
        *slot.borrow_mut() = Some(Box::new(action));
    });
}
pub(in crate::desktop) fn candidate() -> HWND {
    CANDIDATE.with(|slot| slot.get())
}
unsafe extern "system" fn fixture_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let action = CREATE.with(|slot| slot.borrow_mut().take());
        if let Some(action) = action {
            CANDIDATE.with(|slot| slot.set(hwnd));
            action();
        }
    }
    desktop::procedure::window_proc(hwnd, msg, wp, lp)
}
struct Keys([u8; 256]);
impl Keys {
    unsafe fn held(keys: &[VIRTUAL_KEY]) -> Self {
        let mut before = [0; 256];
        GetKeyboardState(&mut before).unwrap();
        let mut current = [0; 256];
        for key in keys {
            current[key.0 as usize] = 0x80;
        }
        SetKeyboardState(&current).unwrap();
        Self(before)
    }
}
impl Drop for Keys {
    fn drop(&mut self) {
        unsafe {
            SetKeyboardState(&self.0).unwrap();
        }
    }
}

#[test]
fn shift_digit_one_is_exact_and_repeat_does_not_open_or_reach_edit_commands() {
    assert!(chord(WM_KEYDOWN, 0x31, false, true, false, false));
    for (ctrl, shift, alt, win) in [
        (true, true, false, false),
        (false, false, false, false),
        (false, true, true, false),
        (false, true, false, true),
    ] {
        assert!(!chord(WM_KEYDOWN, 0x31, ctrl, shift, alt, win));
    }
    assert!(!chord(
        WM_KEYDOWN,
        VK_NUMPAD1.0 as usize,
        false,
        true,
        false,
        false
    ));
    assert!(!chord(WM_SYSKEYDOWN, 0x31, false, true, false, false));
    assert!(!chord(WM_KEYUP, 0x31, false, true, false, false));
    unsafe {
        let _apartment = Apartment::new().unwrap();
        let _keys = Keys::held(&[VK_SHIFT]);
        let (_, doc) = document();
        assert!(dispatch(
            doc.window.0,
            &doc.state,
            WM_KEYDOWN,
            WPARAM(0x31),
            LPARAM(1 << 30)
        )
        .is_some());
        assert!(!doc.state.file_dialog.get());
        assert!(doc.state.art_request.borrow().is_none());
    }
}
#[test]
fn search_retains_original_indices_and_disabled_or_stale_catalog_cannot_queue() {
    unsafe {
        let _apartment = Apartment::new().unwrap();
        let (_, doc) = document();
        let loaded = catalog(|raw| {
            let mut second = raw["capabilities"]["artDefinitions"][0].clone();
            second["id"] = "disabled".into();
            second["label"] = "第二项".into();
            second["enabled"] = false.into();
            raw["capabilities"]["artDefinitions"]
                .as_array_mut()
                .unwrap()
                .push(second);
        });
        doc.state.catalog.borrow_mut().loaded = Some(loaded);
        let snap = model::Snapshot::capture(doc.window.0, &doc.state);
        assert_eq!(snap.search("第二"), vec![1]);
        assert_eq!(snap.search("TEST-ART"), vec![0]);
        assert!(snap.search("missing").is_empty());
        assert!(snap.request(1, &doc.state).is_none());
        let request = snap.request(0, &doc.state).unwrap();
        doc.state.catalog.borrow_mut().loaded = Some(catalog(|_| {}));
        request.queue(doc.window.0, &doc.state).unwrap();
        assert!(doc.state.art_request.borrow().is_none());
        assert!(!snap.current(doc.window.0, &doc.state));
    }
}
#[test]
fn local_surface_requires_explicit_origin_and_never_derives_https_from_wss() {
    unsafe {
        let _apartment = Apartment::new().unwrap();
        let (_, doc) = document();
        doc.state.catalog.borrow_mut().loaded = Some(catalog(|raw| {
            raw["capabilities"]["artDefinitions"][0]["metadata"] = serde_json::json!({"capabilities":{"surface":{"variants":[{"runtime":"javascript"}]}}})
        }));
        let without = model::Snapshot::capture(doc.window.0, &doc.state);
        assert!(without.route(0, &doc.state).is_none());
        doc.state.authorization.borrow_mut().origin = Some("https://fixture.invalid".into());
        assert!(without.route(0, &doc.state).is_none());
        let explicit = model::Snapshot::capture(doc.window.0, &doc.state);
        let request = explicit.request(0, &doc.state).unwrap();
        assert!(matches!(&request.route,Route::Local(origin) if origin=="https://fixture.invalid"));
        doc.state.authorization.borrow_mut().origin = Some("https://different.invalid".into());
        assert!(!request.current(doc.window.0, &doc.state));
        assert!(doc.state.authorization.borrow().request.is_none());
    }
}
#[test]
fn cancelled_or_changed_selection_never_changes_source_and_single_pending_slot_is_bounded() {
    unsafe {
        let _apartment = Apartment::new().unwrap();
        let (_, doc) = document();
        doc.state.catalog.borrow_mut().loaded = Some(catalog(|_| {}));
        let stamp = doc.state.canvas.borrow().editor.document_stamp();
        choose(doc.window.0, &doc.state, |_| Ok(dialog::Outcome::Cancel)).unwrap();
        assert!(doc.state.art_request.borrow().is_none());
        assert!(stamp.matches(&doc.state.canvas.borrow().editor));
        choose(doc.window.0, &doc.state, |_| Ok(dialog::Outcome::Select(0))).unwrap();
        assert!(doc.state.art_request.borrow().is_some());
        let snap = model::Snapshot::capture(doc.window.0, &doc.state);
        snap.request(0, &doc.state)
            .unwrap()
            .queue(doc.window.0, &doc.state)
            .unwrap();
        assert_eq!(doc.state.art_request.borrow().as_ref().unwrap().index, 0);
        assert!(doc.state.selected_art.borrow().is_none());
        assert!(!doc.state.file_dialog.get());
    }
}
#[test]
fn blocked_or_unbound_source_is_not_available_and_primary_port_matches_contract() {
    unsafe {
        let _apartment = Apartment::new().unwrap();
        let (_, doc) = document();
        assert!(available(&doc.state));
        assert!(!owns(HWND::default(), &doc.state));
        for flag in [
            &doc.state.group_locked,
            &doc.state.group_hidden,
            &doc.state.capture_active,
            &doc.state.file_dialog,
            &doc.state.exit_requested,
        ] {
            flag.set(true);
            assert!(!available(&doc.state));
            flag.set(false);
        }
        let loaded = catalog(|_| {});
        assert_eq!(
            model::primary_input(&loaded.handshake.capability(0).unwrap()),
            "pixels"
        );
        let empty = State::new(None, false).unwrap();
        assert!(!available(&empty));
    }
}

#[path = "art_picker_ime_tests.rs"]
mod ime;

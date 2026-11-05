//! 自有 HWND/线程键表的隔离测试；不注册全局热键，不以合成消息冒充人工 IME/普通前台验收。
use super::*;
#[path = "parameter_panel_candidates_tests.rs"]
mod candidates;
#[path = "parameter_panel_embedded_tests.rs"]
mod embedded;
#[path = "parameter_panel_lifetime_tests.rs"]
mod lifetime;
#[path = "parameter_panel_paint_tests.rs"]
mod paint;
#[path = "parameter_panel_sticker_tests.rs"]
mod sticker;
#[path = "parameter_panel_transaction_tests.rs"]
mod transactions;
struct Fixture {
    panel: Panel,
    host: desktop::Window,
    state: Box<State>,
}
impl Fixture {
    unsafe fn new() -> Self {
        Self::with_parameters(serde_json::json!([
            {"id":"text","type":"string","default":"原值"},
            {"id":"n","widget":"slider","default":3,"min":0,"max":10,"step":0.5},
            {"id":"json","type":"json","multiline":true,"default":{"value":1}},
            {"id":"enabled","type":"boolean","default":false},
            {"id":"mode","widget":"select","options":[{"label":"快","value":"fast"},{"label":"数","value":2}],"default":"fast"},
            {"id":"secret","secret":true,"default":"DO_NOT_RENDER"}
        ]))
    }
    unsafe fn with_parameters(parameters: serde_json::Value) -> Self {
        let (_listener, identity, _) = crate::loom_bridge_client::tests::fixture();
        let mut value = crate::loom_handshake::tests::fixture();
        value["capabilities"]["artDefinitions"][0]["parameters"] = parameters;
        let handshake = crate::loom_handshake::response(&serde_json::to_vec(&value).unwrap())
            .unwrap()
            .unwrap();
        let state = Box::new(State::new(None, false).unwrap());
        *state.selected_art.borrow_mut() = Some(desktop::capability_selection::Selection {
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
        let instance = HINSTANCE(GetModuleHandleW(None).unwrap().0);
        let class = w!("Otter.Native.ParameterPanelFixture");
        let registered = RegisterClassW(&WNDCLASSW {
            hInstance: instance,
            lpszClassName: class,
            lpfnWndProc: Some(desktop::procedure::window_proc),
            ..Default::default()
        });
        assert!(registered != 0 || GetLastError() == ERROR_CLASS_ALREADY_EXISTS);
        let host = desktop::Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class,
                w!("parameter fixture"),
                WS_POPUP,
                0,
                0,
                320,
                240,
                None,
                None,
                Some(instance),
                Some((&*state as *const State).cast()),
            )
            .unwrap(),
        );
        let panel = Panel::new(host.0, &state).unwrap();
        Self { panel, host, state }
    }
    fn binding(&self, key: &str) -> controls::Binding {
        self.panel
            .ui
            .bindings
            .borrow()
            .iter()
            .find(|b| b.key == key)
            .unwrap_or_else(|| panic!("missing {key}"))
            .clone()
    }
    unsafe fn text(&self, key: &str, text: &str) -> HWND {
        let edit = self.binding(key).hwnd;
        SetWindowTextW(edit, &HSTRING::from(text)).unwrap();
        edit
    }
    unsafe fn click(&self, key: &str) {
        SendMessageW(self.binding(key).hwnd, BM_CLICK, None, None);
    }
    unsafe fn tick(&self) {
        self.panel.update(&self.state).unwrap();
    }
    fn value(&self, name: &str) -> serde_json::Value {
        self.state
            .selected_art
            .borrow()
            .as_ref()
            .unwrap()
            .parameters
            .get(name)
            .flatten()
            .cloned()
            .unwrap()
    }
}
#[test]
fn bare_tab_is_distinct_and_modified_tab_art_and_toolbar_are_not_claimed() {
    unsafe {
        assert!(keyboard::host_key(WM_KEYDOWN, VK_TAB.0 as usize));
        assert!(!keyboard::host_key(WM_KEYUP, VK_TAB.0 as usize));
        for key in [0x45, 0x31, 0x34, VK_F4.0 as usize] {
            assert!(!keyboard::host_key(WM_KEYDOWN, key));
        }
        let f = Fixture::new();
        assert!(dispatch(
            f.host.0,
            &f.state,
            WM_KEYDOWN,
            WPARAM(VK_TAB.0 as usize),
            LPARAM(1 << 30)
        )
        .is_some());
        assert!(!f.state.parameters_ui.toggle.get());
        dispatch(
            f.host.0,
            &f.state,
            WM_KEYDOWN,
            WPARAM(VK_TAB.0 as usize),
            LPARAM(0),
        );
        assert!(f.state.parameters_ui.toggle.replace(false));
        for flag in [
            &f.state.group_locked,
            &f.state.group_hidden,
            &f.state.capture_active,
            &f.state.file_dialog,
        ] {
            flag.set(true);
            dispatch(
                f.host.0,
                &f.state,
                WM_KEYDOWN,
                WPARAM(VK_TAB.0 as usize),
                LPARAM(0),
            );
            assert!(!f.state.parameters_ui.toggle.get());
            flag.set(false);
        }
    }
}
#[test]
fn panel_uses_real_edit_combo_slider_and_secret_free_shared_snapshot() {
    unsafe {
        let f = Fixture::new();
        assert_eq!(f.panel.ui.model.borrow().rows.len(), 5);
        assert!(f
            .panel
            .ui
            .model
            .borrow()
            .rows
            .iter()
            .all(|row| row.id != "secret"));
        for key in [
            "text:edit",
            "n:edit",
            "n:slider",
            "json:edit",
            "enabled:boolean",
            "mode:options",
        ] {
            assert!(IsWindow(Some(f.binding(key).hwnd)).as_bool());
        }
        assert!(f.panel.exit_idle());
        f.state
            .execution_results
            .borrow_mut()
            .begin(crate::graph_requests::Request {
                id: crate::workspace_id::UnitId::parse("old-run").unwrap(),
                generation: 1,
            });
        f.click("enabled:boolean");
        assert_eq!(f.value("enabled"), false);
        f.tick();
        assert_eq!(f.value("enabled"), true);
        assert!(f.state.pending_start.borrow().is_some());
        assert!(f.state.execution_cancel.borrow().is_some());
    }
}

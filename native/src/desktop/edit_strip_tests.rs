//! Actual native-toolbar/host wiring; baseline visual references do not prove screen composition or manual visual acceptance.
use super::*;
#[path = "edit_extension_toolbar_tests.rs"]
mod extension_tools;
#[path = "edit_strip_interaction_tests.rs"]
mod interaction;
#[path = "pin_settings_mirror_tests.rs"]
mod mirror;
#[path = "edit_strip_move_tests.rs"]
mod movement;
#[path = "edit_strip_overlay_tests.rs"]
mod overlays;
#[path = "edit_property_window_tests.rs"]
mod properties;
#[path = "pin_settings_window_tests.rs"]
mod settings;
use windows::{
    core::w,
    Win32::{
        System::LibraryLoader::GetModuleHandleW,
        UI::{HiDpi::GetDpiForWindow, Input::KeyboardAndMouse::IsWindowEnabled},
    },
};
struct Fixture {
    panel: Window,
    owner: Window,
    state: Box<State>,
}
#[test]
fn baseline_svg_assets_are_fully_supported_not_approximated_paths() {
    assert_eq!(catalog::choices(catalog::SHAPE), &[0x52, 0x45, 0x54, 0x50]);
    assert_eq!(catalog::choices(catalog::LINE), &[0x4c, 0x4b]);
    assert_eq!(catalog::choices(catalog::BRUSH), &[0x42, 0x4a]);
    for command in [
        0,
        0x53,
        0x52,
        0x45,
        0x54,
        0x50,
        0x4c,
        0x4b,
        0x42,
        0x4a,
        0x58,
        0x4e,
        0x4d,
        0x47,
        0x57,
        0x102e,
        0x1007,
        0x1008,
        0x1022,
        0x1023,
        catalog::OCR as usize,
        catalog::TRANSLATE as usize,
    ] {
        catalog::icon(command).validate().unwrap();
    }
}
impl Fixture {
    unsafe fn new() -> Self {
        Self::with_visibility(false)
    }
    unsafe fn with_visibility(visible: bool) -> Self {
        let instance = HINSTANCE(GetModuleHandleW(None).unwrap().0);
        let class = w!("Otter.EditStrip.TestHost");
        let registered = RegisterClassW(&WNDCLASSW {
            hInstance: instance,
            lpszClassName: class,
            lpfnWndProc: Some(crate::desktop::procedure::window_proc),
            ..Default::default()
        });
        assert!(registered != 0 || GetLastError() == ERROR_CLASS_ALREADY_EXISTS);
        let state = Box::new(
            State::new(
                Some(crate::image_decode::Image {
                    width: 64,
                    height: 64,
                    pixels: vec![255; 64 * 64 * 4].into(),
                }),
                true,
            )
            .unwrap(),
        );
        let owner = Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class,
                w!("edit strip test"),
                WS_POPUP,
                200,
                300,
                320,
                240,
                None,
                None,
                Some(instance),
                Some((&*state as *const State).cast()),
            )
            .unwrap(),
        );
        let panel = create(owner.0, &state, visible).unwrap();
        super::super::prepare(panel.0, &state).unwrap();
        Self {
            panel,
            owner,
            state,
        }
    }
}
#[test]
fn attached_strip_uses_main_layout_and_follows_move_before_position_call_returns() {
    unsafe {
        let f = Fixture::new();
        let _ = ShowWindow(f.owner.0, SW_SHOWNOACTIVATE);
        let _ = ShowWindow(f.panel.0, SW_SHOWNOACTIVATE);
        assert_eq!(GetWindow(f.panel.0, GW_OWNER).unwrap(), f.owner.0);
        assert_eq!(
            GetWindowLongW(f.panel.0, GWL_STYLE) as u32 & WS_CAPTION.0,
            0
        );
        let dpi = GetDpiForWindow(f.owner.0).max(96);
        for (x, y) in [(250, 350), (400, 450), (300, 400)] {
            SetWindowPos(
                f.owner.0,
                None,
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
            .unwrap();
            let mut bar = RECT::default();
            let mut host = RECT::default();
            GetWindowRect(f.panel.0, &mut bar).unwrap();
            GetWindowRect(f.owner.0, &mut host).unwrap();
            assert_eq!(bar.bottom, host.top);
            assert_eq!(bar.left, host.left - (12 * dpi / 96) as i32);
            assert_eq!(bar.bottom - bar.top, (38 * dpi / 96) as i32);
        }
    }
}
#[test]
fn group_remembers_actual_tool_and_body_click_uses_original_command_without_editing_document() {
    unsafe {
        let f = Fixture::new();
        let stamp = f.state.canvas.borrow().editor.document_stamp();
        for removed in [0x41, 0x55] {
            SetPropW(
                f.panel.0,
                &key(catalog::LINE),
                Some(HANDLE(removed as *mut _)),
            )
            .unwrap();
            assert_eq!(choice(f.panel.0, catalog::LINE), 0x4c);
        }
        crate::desktop::context_menu::execute(f.owner.0, &f.state, 0x45).unwrap();
        refresh(&f.state);
        assert_eq!(choice(f.panel.0, catalog::SHAPE), 0x45);
        crate::desktop::context_menu::execute(f.owner.0, &f.state, 0x56).unwrap();
        refresh(&f.state);
        assert_eq!(choice(f.panel.0, catalog::SHAPE), 0x45);
        SendMessageW(
            GetDlgItem(Some(f.panel.0), catalog::SHAPE as i32).unwrap(),
            BM_CLICK,
            None,
            None,
        );
        let mut message = MSG::default();
        assert!(PeekMessageW(
            &mut message,
            Some(f.owner.0),
            super::super::EXECUTE,
            super::super::EXECUTE,
            PM_REMOVE
        )
        .as_bool());
        DispatchMessageW(&message);
        assert_eq!(f.state.canvas.borrow().tool, crate::editor::Tool::Ellipse);
        assert!(stamp.matches(&f.state.canvas.borrow().editor));
    }
}
#[test]
fn undo_redo_reflect_actual_history_and_busy_stale_click_does_not_mutate() {
    unsafe {
        let f = Fixture::new();
        let undo = GetDlgItem(Some(f.panel.0), 0x1007).unwrap();
        let redo = GetDlgItem(Some(f.panel.0), 0x1008).unwrap();
        assert!(!IsWindowEnabled(undo).as_bool() && !IsWindowEnabled(redo).as_bool());
        {
            let mut c = f.state.canvas.borrow_mut();
            c.editor
                .begin(
                    crate::editor::Tool::Line,
                    crate::viewport::Point { x: 1.0, y: 1.0 },
                )
                .unwrap();
            c.editor
                .extend(crate::viewport::Point { x: 20.0, y: 20.0 })
                .unwrap();
            assert!(c.editor.commit());
        }
        refresh(&f.state);
        assert!(IsWindowEnabled(undo).as_bool());
        f.state.file_dialog.set(true);
        execute(f.owner.0, &f.state, 0x1007, false);
        assert_eq!(f.state.canvas.borrow().editor.elements().len(), 1);
        f.state.file_dialog.set(false);
        execute(f.owner.0, &f.state, 0x1007, false);
        assert!(f.state.canvas.borrow().editor.elements().is_empty());
        assert!(IsWindowEnabled(redo).as_bool());
        execute(f.owner.0, &f.state, 0x1008, false);
        assert_eq!(f.state.canvas.borrow().editor.elements().len(), 1);
    }
}
#[test]
fn all_existing_tools_remain_in_groups_and_all_icons_are_valid() {
    let commands: Vec<_> = [
        catalog::MODE,
        catalog::SHAPE,
        catalog::LINE,
        catalog::BRUSH,
        catalog::LABEL,
        catalog::EFFECT,
    ]
    .into_iter()
    .flat_map(catalog::choices)
    .copied()
    .chain([0x53, 0x57, 0x56])
    .collect();
    for entry in crate::desktop::tool_commands::TOOLS {
        assert!(commands.contains(&entry.id), "{}", entry.label);
        catalog::icon(entry.id).validate().unwrap();
    }
}
#[test]
fn anchors_prefer_above_then_below_and_respect_negative_work_area() {
    let work = RECT {
        left: -1920,
        top: -300,
        right: 0,
        bottom: 740,
    };
    let host = RECT {
        left: -1800,
        top: 0,
        right: -1500,
        bottom: 200,
    };
    let frame = anchored(host, work, 420, 38, 12);
    assert_eq!((frame.left, frame.bottom), (-1812, 0));
    let frame = anchored(
        RECT {
            top: -300,
            bottom: -100,
            ..host
        },
        work,
        420,
        38,
        12,
    );
    assert_eq!(frame.top, -100);
}

#[test]
fn dropdown_modal_refresh_keeps_all_buttons_and_properties_in_their_real_state() {
    unsafe {
        let f = Fixture::new();
        f.state
            .canvas
            .borrow_mut()
            .select_tool(crate::editor::Tool::Rectangle);
        refresh(&f.state);
        let ids = control_ids();
        let enabled = || {
            ids.iter()
                .map(|id| IsWindowEnabled(GetDlgItem(Some(f.panel.0), *id).unwrap()).as_bool())
                .collect::<Vec<_>>()
        };
        let before = enabled();
        let properties = neuro_mini_ui::property_window(f.panel.0);
        let number = GetDlgItem(
            properties,
            crate::desktop::edit_properties::WIDTH as i32 * 4,
        )
        .unwrap();
        assert!(IsWindowEnabled(number).as_bool());
        let result: Result<()> = super::super::popup::preserve(&f.state, || {
            let _modal = crate::desktop::file_dialog::Guard::begin(&f.state.file_dialog).unwrap();
            // refresh runs after every host message; the original implementation disabled the whole bar through its own menu gate.
            SendMessageW(f.owner.0, WM_NULL, None, None);
            assert_eq!(enabled(), before);
            execute(f.owner.0, &f.state, catalog::LINE as usize, false);
            assert_eq!(f.state.canvas.borrow().tool, crate::editor::Tool::Rectangle);
            assert_eq!(neuro_mini_ui::property_window(f.panel.0), properties);
            assert!(IsWindowEnabled(number).as_bool());
            Err(windows::core::Error::new(E_ABORT, "cancel menu"))
        });
        assert!(result.is_err());
        assert!(!f.state.file_dialog.get());
        assert!(!super::super::popup::active(&f.state));
        assert_eq!(enabled(), before);
    }
}

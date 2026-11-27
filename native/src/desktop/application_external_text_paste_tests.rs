//! A separate Win32 EDIT process publishes CF_UNICODETEXT to a fresh noninteractive
//! station. The parent never changes station, opens the clipboard, or synthesizes input.
use super::*;
use crate::desktop::{open_job, paste_input, workspace_window_fixture};
use std::{process::Command, time::Duration};
use windows::Win32::{
    System::{Com::CoCreateGuid, DataExchange::GetClipboardSequenceNumber, StationsAndDesktops::*},
    UI::Controls::EM_SETSEL,
};

const ENV: &str = "OTTER_TEST_EXTERNAL_TEXT_STATION";
const ROLE: &str = "OTTER_TEST_EXTERNAL_TEXT_ROLE";
const PREFIX: &str = "Otter.TextPaste.";
const SAMPLE: &str = "\tfn 你好() {\r\n\r\n    let value = \"🦦\";\r\n}";

#[test]
#[ignore = "requires permission to create a private window station; run explicitly on the isolated desktop acceptance host"]
fn external_editor_clipboard_creates_a_new_editable_text_pin() {
    struct Station(HWINSTA);
    impl Drop for Station {
        fn drop(&mut self) {
            unsafe { CloseWindowStation(self.0).unwrap() };
        }
    }
    let name = format!("{PREFIX}{:?}", unsafe { CoCreateGuid().unwrap() });
    let _station = Station(unsafe {
        CreateWindowStationW(
            &HSTRING::from(&name),
            CWF_CREATE_ONLY,
            WINSTA_ALL_ACCESS as u32,
            None,
        )
        .unwrap()
    });
    for role in ["writer", "reader"] {
        let out = crate::test_process::run(
            Command::new(std::env::current_exe().unwrap())
                .args(["private_text_station_child", "--ignored", "--nocapture"])
                .env(ENV, &name)
                .env(ROLE, role),
            Duration::from_secs(30),
        )
        .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success()
                && stdout.contains(&format!("private-text-clipboard: {role} complete")),
            "{role}: {stdout}\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

// Never attach a shared test harness to this station. Each short-lived helper
// owns its assigned station and desktop handles until its process exits.
#[test]
#[ignore = "private subprocess of external_editor_clipboard_creates_a_new_editable_text_pin"]
fn private_text_station_child() {
    let Ok(name) = std::env::var(ENV) else { return };
    let role = std::env::var(ROLE).unwrap();
    let suffix = name.strip_prefix(PREFIX).unwrap();
    assert!(suffix.len() == 36 && suffix.bytes().all(|c| c.is_ascii_hexdigit() || c == b'-'));
    assert!(matches!(role.as_str(), "writer" | "reader"));
    assert!(std::env::args().any(|arg| arg == "private_text_station_child"));
    unsafe {
        let station =
            OpenWindowStationW(&HSTRING::from(&name), false, WINSTA_ALL_ACCESS as u32).unwrap();
        SetProcessWindowStation(station).unwrap();
        let active = GetProcessWindowStation().unwrap();
        let mut flags = USEROBJECTFLAGS::default();
        let mut actual = [0_u16; 96];
        GetUserObjectInformationW(
            HANDLE(active.0),
            UOI_FLAGS,
            Some((&mut flags as *mut USEROBJECTFLAGS).cast()),
            std::mem::size_of_val(&flags) as u32,
            None,
        )
        .unwrap();
        GetUserObjectInformationW(
            HANDLE(active.0),
            UOI_NAME,
            Some(actual.as_mut_ptr().cast()),
            std::mem::size_of_val(&actual) as u32,
            None,
        )
        .unwrap();
        assert_eq!(
            flags.dwFlags & WSF_VISIBLE as u32,
            0,
            "refuse visible clipboard"
        );
        assert_eq!(
            String::from_utf16(&actual[..actual.iter().position(|&u| u == 0).unwrap()]).unwrap(),
            name
        );
        let desktop = CreateDesktopW(
            &HSTRING::from(&role),
            None,
            None,
            DESKTOP_CONTROL_FLAGS(0),
            DESKTOP_CREATEWINDOW.0
                | DESKTOP_CREATEMENU.0
                | DESKTOP_READOBJECTS.0
                | DESKTOP_WRITEOBJECTS.0,
            None,
        )
        .unwrap();
        SetThreadDesktop(desktop).unwrap();
        if role == "writer" {
            let edit = crate::desktop::Window(
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("EDIT"),
                    &HSTRING::from(SAMPLE),
                    WS_POPUP | WINDOW_STYLE(ES_MULTILINE as u32),
                    0,
                    0,
                    200,
                    100,
                    None,
                    None,
                    None,
                    None,
                )
                .unwrap(),
            );
            SendMessageW(edit.0, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
            let before = GetClipboardSequenceNumber();
            SendMessageW(edit.0, WM_COPY, None, None);
            assert_ne!(
                GetClipboardSequenceNumber(),
                before,
                "native EDIT did not copy"
            );
        } else {
            read_pin();
        }
    }
    println!("private-text-clipboard: {role} complete");
}

unsafe fn read_pin() {
    let _apt = crate::desktop::Apartment::new().unwrap();
    let instance = workspace_window_fixture::register();
    let first = DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
    let mut app = Application::new(instance, first);
    let doc = &app.documents[0];
    assert!(paste_input::read_image(doc.window.0, &doc.state, false).is_err());
    let snapshot = paste_input::read_image(doc.window.0, &doc.state, true).unwrap();
    assert!(matches!(snapshot, paste_input::Snapshot::Text(_)));
    paste_input::deliver_at(doc.window.0, &doc.state, snapshot, true, POINT::default()).unwrap();
    open_job::finish(doc.window.0, &doc.state, true);
    app.pending_windows(false).unwrap();
    assert_eq!(app.documents.len(), 2);
    let canvas = app.documents[1].state.canvas.borrow();
    let stroke = &canvas.editor.elements()[0];
    assert!(canvas.pinned && canvas.image.is_some());
    assert_eq!(stroke.tool, crate::editor::Tool::Text);
    assert_eq!(
        stroke.text.as_ref().unwrap().content(),
        SAMPLE.replace("\r\n", "\n")
    );
}

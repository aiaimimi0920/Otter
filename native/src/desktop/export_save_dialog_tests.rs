//! Private-desktop regression for the one-stop Neuro-themed export dialog.
use super::*;
use crate::desktop::image::Image;
use windows::core::w;

#[test]
fn export_save_dialog_initializes_switches_formats_and_confirms() {
    crate::desktop::workspace_window_fixture::with_private_input_desktop(|| unsafe {
        let _apt = crate::desktop::Apartment::new().unwrap();
        let temp_dir =
            std::env::temp_dir().join(format!("otter-export-dialog-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let owner = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("export host"),
            WS_POPUP,
            0,
            0,
            16,
            16,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let tid = windows::Win32::System::Threading::GetCurrentThreadId();
        let owner_raw = owner.0 as isize;
        let worker = std::thread::spawn(move || {
            unsafe extern "system" fn find(hwnd: HWND, lp: LPARAM) -> windows::core::BOOL {
                let slot = &mut *(lp.0 as *mut (isize, HWND));
                if hwnd.0 as isize != slot.0 && IsWindowVisible(hwnd).as_bool() {
                    let mut class = [0u16; 16];
                    let len = GetClassNameW(hwnd, &mut class) as usize;
                    if len == 6 && String::from_utf16_lossy(&class[..len]) == "#32770" {
                        slot.1 = hwnd;
                        return false.into();
                    }
                }
                true.into()
            }
            let mut dialog = HWND::default();
            for _ in 0..300 {
                let mut state = (owner_raw, HWND::default());
                let _ = EnumThreadWindows(tid, Some(find), LPARAM(&mut state as *mut _ as isize));
                if !state.1.is_invalid() {
                    dialog = state.1;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            assert!(!dialog.is_invalid(), "export dialog did not open");
            struct CloseOnDrop(HWND);
            impl Drop for CloseOnDrop {
                fn drop(&mut self) {
                    unsafe {
                        let _ = PostMessageW(
                            Some(self.0),
                            WM_COMMAND,
                            WPARAM(IDCANCEL.0 as usize),
                            LPARAM(0),
                        );
                    }
                }
            }
            let guard = CloseOnDrop(dialog);
            // Wait for initial PNG size to settle so label 201 shows exact encoded size.
            let pending_text = crate::ui_text::text("export-size-pending");
            for _ in 0..300 {
                let status = read_field(dialog, 201);
                if !status.is_empty() && status != pending_text {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            for cmd in [103usize, 104, 102] {
                let _ = SendMessageW(dialog, WM_COMMAND, Some(WPARAM(cmd)), None);
                let ok_enabled = GetDlgItem(Some(dialog), IDOK.0).is_ok_and(|btn| {
                    windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(btn).as_bool()
                });
                assert!(
                    ok_enabled,
                    "IDOK must stay enabled without flicker across format switch"
                );
                let status = read_field(dialog, 201);
                assert_ne!(
                    status, pending_text,
                    "size label must not flash back to pending during format switch"
                );
            }
            let file_name = read_field(dialog, layout::FILE_EDIT_ID);
            assert!(
                file_name.ends_with(".jpg"),
                "expected .jpg filename, got {file_name}"
            );
            let ok_enabled = GetDlgItem(Some(dialog), IDOK.0).is_ok_and(|btn| {
                windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(btn).as_bool()
            });
            assert!(ok_enabled, "export IDOK remained disabled");
            std::mem::forget(guard);
            let _ = PostMessageW(Some(dialog), WM_COMMAND, WPARAM(IDOK.0 as usize), LPARAM(0));
        });
        let source = Source {
            image: Image {
                width: 8,
                height: 8,
                pixels: vec![255; 8 * 8 * 4].into(),
            },
            editor: crate::editor::Editor::default(),
            style: Default::default(),
            opacity: 1.0,
        };
        let picked = choose(owner, source, "Otter_unit".into(), temp_dir.clone());
        worker.join().unwrap();
        let _ = DestroyWindow(owner);
        let _ = std::fs::remove_dir_all(&temp_dir);
        let choice = picked.unwrap().unwrap();
        assert_eq!(choice.format, Format::Jpeg(85));
        assert!(!choice.bytes.is_empty());
        assert_eq!(choice.path.file_name().unwrap(), "Otter_unit.jpg");
    });
}

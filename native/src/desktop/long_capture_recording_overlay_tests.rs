//! 真实非模态消息循环：保持源窗启用、箭头在预览之上、中心穿透、完成后成对释放窗口。
use super::*;
use windows::{core::w, Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled};

#[test]
fn modeless_preview_has_only_arrow_hits_and_finishes_without_disabling_owner() {
    unsafe {
        let owner = crate::desktop::Window(
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("BUTTON"),
                w!("recording source fixture"),
                WS_POPUP | WS_VISIBLE,
                0,
                0,
                900,
                700,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let before = GetForegroundWindow();
        let (mut input, _) = setup(vec![], None);
        input.movement = movement::State::new(Region {
            left: 100,
            top: 100,
            width: 32,
            height: 48,
        });
        let mut handles = Vec::new();
        let result = super::super::super::overlay::run_with(owner.0, &input, |hwnd| {
            assert!(IsWindowEnabled(owner.0).as_bool());
            assert_eq!(GetForegroundWindow(), before);
            let backing = HWND(GetPropW(hwnd, w!("Otter.LongCapture.Preview")).0);
            handles.extend([hwnd, backing]);
            assert_eq!(GetWindow(backing, GW_OWNER).unwrap(), owner.0);
            let mut current = GetWindow(hwnd, GW_HWNDNEXT).unwrap_or_default();
            while !current.is_invalid() && current != backing {
                current = GetWindow(current, GW_HWNDNEXT).unwrap_or_default();
            }
            assert_eq!(
                current, backing,
                "arrow window must remain above preview backing"
            );
            let mut outer = RECT::default();
            GetWindowRect(hwnd, &mut outer).unwrap();
            let mut origin = POINT::default();
            windows::Win32::Graphics::Gdi::ClientToScreen(hwnd, &mut origin)
                .ok()
                .unwrap();
            assert_eq!((origin.x, origin.y), (outer.left, outer.top));
            for id in 108..=111 {
                let child = GetDlgItem(Some(hwnd), id).unwrap();
                let mut rect = RECT::default();
                GetWindowRect(child, &mut rect).unwrap();
                let hit = WindowFromPoint(POINT {
                    x: (rect.left + rect.right) / 2,
                    y: (rect.top + rect.bottom) / 2,
                });
                assert_eq!(hit, child, "arrow must be visible and clickable");
            }
            let center = WindowFromPoint(POINT {
                x: (outer.left + outer.right) / 2,
                y: (outer.top + outer.bottom) / 2,
            });
            assert_eq!(center, owner.0, "preview pixels must not block the source");
            PostMessageW(Some(hwnd), WM_COMMAND, WPARAM(1), LPARAM(0))?;
            Ok(())
        })
        .unwrap();
        assert_eq!(
            result.map(|image| (image.width, image.height)),
            Some((32, 48))
        );
        assert!(IsWindowEnabled(owner.0).as_bool());
        assert!(handles.iter().all(|hwnd| !IsWindow(Some(*hwnd)).as_bool()));
    }
}

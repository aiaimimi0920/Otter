//! 真实非模态消息循环：保持源窗启用、箭头在预览之上、中心穿透、完成后成对释放窗口。
use super::super::super as dialog;
use super::*;
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::{core::w, Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled};

#[derive(Default)]
struct Positions {
    arrows: Cell<usize>,
    backing: Cell<usize>,
}
unsafe extern "system" fn observe_positions(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    id: usize,
    data: usize,
) -> LRESULT {
    if message == WM_WINDOWPOSCHANGING {
        let counts = &*(data as *const Positions);
        let count = if id == 1 {
            &counts.arrows
        } else {
            &counts.backing
        };
        count.set(count.get() + 1);
    }
    DefSubclassProc(hwnd, message, wp, lp)
}

#[test]
fn modeless_preview_has_only_arrow_hits_and_finishes_without_disabling_owner() {
    unsafe {
        let owner = crate::desktop::Window(
            CreateWindowExW(
                // Let only the owned fixtures decide the global hit-test result; unrelated desktop
                // windows can otherwise sit between this source and the topmost recording overlay.
                WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
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
        let positions = Positions::default();
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
            let arrows: Vec<_> = (108..=111)
                .map(|id| GetDlgItem(Some(hwnd), id).unwrap())
                .collect();
            let data = &positions as *const Positions as usize;
            for arrow in &arrows {
                assert!(SetWindowSubclass(*arrow, Some(observe_positions), 1, data).as_bool());
            }
            assert!(SetWindowSubclass(backing, Some(observe_positions), 2, data).as_bool());
            for _ in 0..3 {
                status(hwnd, "录制中");
            }
            assert_eq!(
                positions.arrows.get(),
                0,
                "status redraw must reuse arrow geometry"
            );
            assert!(
                positions.backing.replace(0) > 0,
                "status must still reach the backing"
            );
            input.job.borrow_mut().append(frame(0, 4)).unwrap();
            // 只领取采集回复；settle 会连新预览回复一起领取，无法观察旧预览阶段。
            let deadline = Instant::now() + Duration::from_secs(5);
            while input.job.borrow().snapshot().frame_count < 2 {
                dialog::handle(hwnd, WM_TIMER, WPARAM(TIMER), LPARAM(0));
                assert!(Instant::now() < deadline, "append reply did not arrive");
                std::thread::sleep(Duration::from_millis(1));
            }
            assert_eq!(input.job.borrow().snapshot().frame_count, 2);
            assert!(input.preview.borrow().is_none());
            assert_eq!(
                positions.backing.get(),
                0,
                "accepted frame must not redraw the old preview"
            );
            dialog::tests::settle(hwnd, &input);
            assert!(positions.backing.get() > 0, "new preview must be presented");
            input.job.borrow_mut().append(frame(0, 8)).unwrap();
            dialog::tests::settle(hwnd, &input);
            assert_eq!(
                input.preview.borrow().as_ref().map(|i| (i.width, i.height)),
                Some((32, 56))
            );
            assert!(positions.backing.get() > 0, "new preview must be presented");
            assert!(
                positions.arrows.get() > 0,
                "changed preview size must relayout arrows"
            );
            let layouts = positions.arrows.get();
            status(hwnd, "录制已停止");
            assert_eq!(positions.arrows.get(), layouts);
            for arrow in arrows {
                assert!(RemoveWindowSubclass(arrow, Some(observe_positions), 1).as_bool());
            }
            assert!(RemoveWindowSubclass(backing, Some(observe_positions), 2).as_bool());
            PostMessageW(Some(hwnd), WM_COMMAND, WPARAM(1), LPARAM(0))?;
            Ok(())
        })
        .unwrap();
        assert_eq!(
            result.map(|image| (image.width, image.height)),
            Some((32, 56))
        );
        assert!(IsWindowEnabled(owner.0).as_bool());
        assert!(handles.iter().all(|hwnd| !IsWindow(Some(*hwnd)).as_bool()));
    }
}

use super::*;
#[path = "composition_live_session_end_tests.rs"]
mod session_end;
use crate::{desktop::Apartment, live_frames::Source};
use std::sync::Arc;
use windows::{core::Interface, Win32::Graphics::Direct3D11::ID3D11Multithread};

pub(super) unsafe fn publish(source: &Source<Texture>) {
    let device = super::super::super::create_device(
        windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_WARP,
    )
    .unwrap();
    let protection: ID3D11Multithread = device.GetImmediateContext().unwrap().cast().unwrap();
    let _ = protection.SetMultithreadProtected(true);
    let frame = super::super::tests::frame(&device, [0, 255, 0, 255]);
    source
        .publish(Arc::try_unwrap(frame).ok().unwrap().resource)
        .unwrap();
}

#[test]
fn input_menu_requires_window_capability_ack_and_focus_loss_clears_queue() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let source = Source::new();
        let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
        SendMessageW(
            window.hwnd,
            WM_SYSCOMMAND,
            Some(WPARAM(input::TOGGLE)),
            None,
        );
        assert_eq!(window.take_input_request(), None);
        window.allow_input().unwrap();
        SendMessageW(
            window.hwnd,
            WM_SYSCOMMAND,
            Some(WPARAM(input::TOGGLE)),
            None,
        );
        assert_eq!(window.take_input_request(), Some(true));
        publish(&source);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        window.input_status(false, true, None);
        SendMessageW(window.hwnd, WM_KEYDOWN, Some(WPARAM(65)), None);
        assert!(window.pop_input().is_none());
        window.input_status(true, false, None);
        SendMessageW(window.hwnd, WM_KEYDOWN, Some(WPARAM(65)), None);
        assert_eq!(
            window.pop_input(),
            Some(crate::live_input::Event::Key {
                code: 65,
                down: true
            })
        );
        SendMessageW(window.hwnd, WM_KEYDOWN, Some(WPARAM(66)), None);
        SendMessageW(window.hwnd, WM_KILLFOCUS, None, None);
        assert_eq!(window.take_input_request(), Some(false));
        assert!(window.pop_input().is_none());
        assert_eq!(
            GetMenuState(
                GetSystemMenu(window.hwnd, false),
                input::TOGGLE as u32,
                MF_BYCOMMAND
            ) & MF_CHECKED.0,
            0
        );
        assert!(window.is_alive());
    }
}

#[test]
#[ignore = "shows owned Live fixture and uses mouse capture, without synthesizing physical input"]
fn input_mouse_letterbox_capture_release_and_capture_loss_are_safe() {
    use crate::live_input::{Button, Event};
    use windows::Win32::UI::Input::KeyboardAndMouse::GetCapture;
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let source = Source::new();
        let window = LiveWindow::new(source.subscribe().unwrap(), 160, 160).unwrap();
        window.allow_input().unwrap();
        window.show().unwrap();
        publish(&source);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        let _ = window.take_input_request();
        window.input_status(true, false, None);
        let mut client = RECT::default();
        GetClientRect(window.hwnd, &mut client).unwrap();
        let view = window
            .state
            .host
            .borrow()
            .as_ref()
            .unwrap()
            .presented_viewport([client.right as u32, client.bottom as u32])
            .unwrap()
            .1;
        let [left, top, right, bottom] = view.destination();
        let center = [((left + right) * 0.5) as i32, ((top + bottom) * 0.5) as i32];
        let lp = |x: i32, y: i32| {
            LPARAM((u32::from(x as i16 as u16) | (u32::from(y as i16 as u16) << 16)) as isize)
        };
        SendMessageW(window.hwnd, WM_LBUTTONDOWN, None, Some(lp(-1, -1)));
        assert!(window.pop_input().is_none());
        SendMessageW(
            window.hwnd,
            WM_LBUTTONDOWN,
            None,
            Some(lp(center[0], center[1])),
        );
        assert_eq!(GetCapture(), window.hwnd);
        assert!(matches!(
            window.pop_input(),
            Some(Event::Button {
                button: Button::Left,
                down: true,
                ..
            })
        ));
        SendMessageW(window.hwnd, WM_MOUSEMOVE, None, Some(lp(-10, -10)));
        assert_eq!(window.pop_input(), Some(Event::Move { point: [0.0, 0.0] }));
        SendMessageW(window.hwnd, WM_LBUTTONUP, None, Some(lp(-10, -10)));
        assert!(matches!(
            window.pop_input(),
            Some(Event::Button {
                down: false,
                point: [0.0, 0.0],
                ..
            })
        ));
        assert_ne!(GetCapture(), window.hwnd);
        assert_eq!(window.take_input_request(), None);
        SendMessageW(
            window.hwnd,
            WM_LBUTTONDBLCLK,
            None,
            Some(lp(center[0], center[1])),
        );
        assert!(matches!(
            window.pop_input(),
            Some(Event::Button { double: true, .. })
        ));
        SendMessageW(window.hwnd, WM_CAPTURECHANGED, None, None);
        assert_eq!(window.take_input_request(), Some(false));
        assert_ne!(GetCapture(), window.hwnd);
    }
}
#[test]
fn owned_window_dispatches_live_notifications_paints_and_closes_without_quitting_app() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let source = Source::new();
        let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
        assert!(window.is_alive());
        assert!(!IsWindowVisible(window.hwnd).as_bool());
        let mut affinity = 0;
        GetWindowDisplayAffinity(window.hwnd, &mut affinity).unwrap();
        assert_eq!(affinity, WDA_EXCLUDEFROMCAPTURE.0);
        publish(&source);
        let mut message = MSG::default();
        assert!(PeekMessageW(
            &mut message,
            Some(window.hwnd),
            notification::MESSAGE,
            notification::MESSAGE,
            PM_REMOVE
        )
        .as_bool());
        DispatchMessageW(&message);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        assert!(window.take_error().is_none());
        assert!(window.is_alive());
        source.close();
        // 不分发关闭通知，实际 WM_PAINT 的补偿读取也必须销毁窗口并释放订阅。
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        assert!(!window.is_alive());
        assert!(!source.has_consumers());
        assert!(window.take_error().is_none());
        assert!(window.show().is_err());
        assert!(!PeekMessageW(&mut message, None, WM_QUIT, WM_QUIT, PM_REMOVE).as_bool());
    }
}
#[test]
fn reentrant_notification_is_redrawn_after_outer_host_operation() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let source = Source::new();
        let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
        publish(&source);
        let mut message = MSG::default();
        assert!(PeekMessageW(
            &mut message,
            Some(window.hwnd),
            notification::MESSAGE,
            notification::MESSAGE,
            PM_REMOVE
        )
        .as_bool());
        window
            .state
            .with_host(|_| {
                SendMessageW(
                    window.hwnd,
                    message.message,
                    Some(message.wParam),
                    Some(message.lParam),
                );
                Ok(())
            })
            .unwrap();
        assert!(window.state.deferred.get());
        assert!(reschedule(window.hwnd, &window.state));
        assert!(!window.state.deferred.get());
        // 隐藏 HWND 不保证维护可见更新区；检查调度成功及单次性，再显式派发绘制。
        assert!(!reschedule(window.hwnd, &window.state));
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        assert_eq!(
            window
                .state
                .with_host(|host| host.paint(D2D_SIZE_U {
                    width: 64,
                    height: 64
                }))
                .unwrap(),
            Some(true)
        );
        assert!(window.is_alive());
        assert!(window.take_error().is_none());
    }
}
#[test]
#[ignore = "shows only an owned synthetic window; requires an interactive Windows desktop"]
fn actual_minimize_releases_surface_and_restore_reuses_static_frame() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let source = Source::new();
        let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
        publish(&source);
        window.show().unwrap();
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        let has_surface = || {
            window
                .state
                .with_host(|host| Ok(host.has_surface()))
                .unwrap()
        };
        assert_eq!(has_surface(), Some(true));
        for _ in 0..3 {
            let _ = ShowWindow(window.hwnd, SW_SHOWMINNOACTIVE);
            assert!(IsIconic(window.hwnd).as_bool());
            assert_eq!(has_surface(), Some(false));
            // 即使显式投递 paint，最小化状态也不能重新分配呈现链。
            SendMessageW(window.hwnd, WM_PAINT, None, None);
            assert_eq!(has_surface(), Some(false));
            assert!(source.has_consumers());
            window.show().unwrap();
            assert!(!IsIconic(window.hwnd).as_bool());
            SendMessageW(window.hwnd, WM_PAINT, None, None);
            assert_eq!(has_surface(), Some(true));
            assert!(window.take_error().is_none());
        }
        // 全程只发布一次；恢复不能依赖新帧。关闭仍由已有源通知/补读处理。
        let _ = ShowWindow(window.hwnd, SW_SHOWMINNOACTIVE);
        source.close();
        window.refresh().unwrap();
        assert!(!window.is_alive());
        assert!(!source.has_consumers());
        assert!(window.take_error().is_none());
        println!("native-live-output-runtime: minimize=true no-hidden-surface=true static-restore=true cycles=3 close=true");
    }
}
#[test]
fn minimized_size_message_releases_surface_before_next_paint() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let source = Source::new();
        let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
        publish(&source);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        assert!(window.is_alive());
        SendMessageW(
            window.hwnd,
            WM_SIZE,
            Some(WPARAM(SIZE_MINIMIZED as usize)),
            None,
        );
        // 合成消息不真的最小化窗口。新设备可重建而非触发旧 surface 的跨设备拒绝，
        // 证明 WndProc 已释放呈现 owner；不改变任何用户窗口状态。
        publish(&source);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        assert!(window.is_alive());
        assert!(window.take_error().is_none());
        assert!(source.has_consumers());
    }
}
#[test]
fn incompatible_device_closes_window_and_preserves_one_shot_error() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let source = Source::new();
        let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
        publish(&source);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        assert!(window.is_alive());
        assert!(window.take_error().is_none());
        publish(&source);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        assert!(!window.is_alive());
        assert!(!source.has_consumers());
        assert!(window.take_error().is_some());
        assert!(window.take_error().is_none());
        assert!(!IsWindow(Some(window.hwnd)).as_bool());
    }
}
#[test]
fn owner_relocation_and_user_close_keep_state_safe_and_release_subscription() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let source = Source::<Texture>::new();
        assert!(LiveWindow::new(source.subscribe().unwrap(), 0, 128).is_err());
        assert!(!source.has_consumers());
        let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
        let pointer = GetWindowLongPtrW(window.hwnd, GWLP_USERDATA);
        let mut moved = Vec::with_capacity(1);
        moved.push(window);
        moved.reserve(10);
        let window = moved.pop().unwrap();
        assert_eq!(GetWindowLongPtrW(window.hwnd, GWLP_USERDATA), pointer);
        SendMessageW(window.hwnd, WM_CLOSE, None, None);
        assert!(!window.is_alive());
        assert!(!source.has_consumers());
        let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
        let hwnd = window.hwnd;
        drop(window);
        assert!(!IsWindow(Some(hwnd)).as_bool());
        assert!(!source.has_consumers());
    }
}

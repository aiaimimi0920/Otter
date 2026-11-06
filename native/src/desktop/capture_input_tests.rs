//! 普通截图显隐回归；只用自有窗口和合成像素，不采集桌面、不注入全局输入。
use super::*;

unsafe fn window(visible: bool) -> crate::desktop::Window {
    let window = crate::desktop::Window(
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            w!("STATIC"),
            w!("capture visibility fixture"),
            WS_POPUP,
            0,
            0,
            10,
            10,
            None,
            None,
            None,
            None,
        )
        .unwrap(),
    );
    if visible {
        let _ = ShowWindow(window.0, SW_SHOWNOACTIVATE);
    }
    window
}

#[test]
fn normal_capture_keeps_existing_windows_visible_on_cancel_failure_and_delivery() {
    unsafe {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        let state = State::new(None, true).unwrap();
        let first = window(true);
        let second = window(true);
        let hidden = window(false);
        let windows = [first.0, second.0, hidden.0];
        *state.capture_windows.borrow_mut() = windows.to_vec();
        let assert_visibility = || {
            assert!(IsWindowVisible(first.0).as_bool());
            assert!(IsWindowVisible(second.0).as_bool());
            assert!(!IsWindowVisible(hidden.0).as_bool());
        };
        let stamp = state.canvas.borrow().editor.document_stamp();
        run_positioned(first.0, &state, true, || {
            assert_visibility();
            Ok(None)
        })
        .unwrap();
        assert_visibility();
        assert!(state.open_job.borrow().is_none());
        let failure = run_positioned(first.0, &state, true, || {
            assert_visibility();
            Err(Error::new(E_FAIL, "Injected visible capture failure"))
        });
        assert_eq!(failure.unwrap_err().code(), E_FAIL);
        assert_visibility();
        assert!(state.open_job.borrow().is_none());
        run_positioned(first.0, &state, true, || {
            assert_visibility();
            Ok(Some((
                Image {
                    width: 1,
                    height: 1,
                    pixels: vec![0, 0, 255, 255].into(),
                },
                Some(POINT { x: -30, y: 10 }),
            )))
        })
        .unwrap();
        super::super::open_job::finish(first.0, &state, true);
        assert_visibility();
        assert!(stamp.matches(&state.canvas.borrow().editor));
        let documents = state.new_pin.borrow_mut().take().unwrap();
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].image.pixels.as_ref(), &[0, 0, 255, 255]);
    }
}

//! 本地 B 到后台 A 的 hover/leave/字符回归；只操作自有源，绝不注入全局输入。
use super::*;

#[test]
#[ignore = "interactive desktop; owned WGC source and local pointer/text messages"]
fn hovered_and_committed_text_reach_occluded_source() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let old = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        assert!(!old.0.is_null());
        let _dpi = Dpi(old);
        let fixture = Fixture::new();
        let target = Target::bind(fixture.root).unwrap();
        let mut owner = Owner::fixture();
        let protection = owner.acquire(target.clone()).unwrap();
        let frame = target.frame_bounds().unwrap();
        let crop = crate::live_window_target::region::Crop::from_screen(
            frame,
            RECT {
                right: frame.left + 120,
                bottom: frame.top + 80,
                ..frame
            },
        )
        .unwrap();
        let mut session =
            Session::start_window_region(target.clone(), Some(crop), crop.screen(frame).unwrap())
                .unwrap();
        session.protect(protection);
        session.show().unwrap();
        until(&mut session, &mut owner, "pointer/text startup", |s| {
            s.input_enabled() && s.window.as_ref().unwrap().frame_for_test().is_some()
        });
        let window = session.window.as_ref().unwrap().hwnd_for_test();
        let point = LPARAM(50 | (40 << 16));
        SendMessageW(window, WM_MOUSEMOVE, None, Some(point));
        SendMessageW(window, WM_MOUSEHOVER, None, Some(point));
        until(&mut session, &mut owner, "hover forwarded", |_| {
            fixture.received(WM_MOUSEHOVER, None)
        });
        SendMessageW(window, WM_MOUSELEAVE, None, None);
        until(&mut session, &mut owner, "leave forwarded", |_| {
            fixture.received(WM_MOUSELEAVE, None)
        });
        SendMessageW(window, WM_LBUTTONDOWN, None, Some(point));
        SendMessageW(window, WM_LBUTTONUP, None, Some(point));
        until(&mut session, &mut owner, "click before composition", |_| {
            fixture.received(WM_LBUTTONUP, None)
        });
        {
            use windows::Win32::UI::Input::Ime::*;
            let context = ImmCreateContext();
            assert!(!context.0.is_null());
            let previous = ImmAssociateContext(window, context);
            struct Ime(HWND, HIMC, HIMC);
            impl Drop for Ime {
                fn drop(&mut self) {
                    unsafe {
                        ImmAssociateContext(self.0, self.2);
                        let _ = ImmDestroyContext(self.1);
                    }
                }
            }
            let _ime = Ime(window, context, previous);
            SendMessageW(window, WM_IME_STARTCOMPOSITION, None, None);
            let mut composition = COMPOSITIONFORM::default();
            assert!(ImmGetCompositionWindow(context, &mut composition).as_bool());
            assert_eq!(
                [composition.ptCurrentPos.x, composition.ptCurrentPos.y],
                [50, 40]
            );
            let mut candidate = CANDIDATEFORM::default();
            assert!(ImmGetCandidateWindow(context, 0, &mut candidate).as_bool());
            assert_eq!(
                [candidate.ptCurrentPos.x, candidate.ptCurrentPos.y],
                [50, 40]
            );
            SendMessageW(window, WM_IME_ENDCOMPOSITION, None, None);
        }
        SendMessageW(window, WM_CHAR, Some(WPARAM(0x4e2d)), None);
        until(&mut session, &mut owner, "committed text forwarded", |_| {
            fixture.received(WM_CHAR, Some(0x4e2d))
        });
        assert!(session.take_input_error().is_none());
        assert_eq!(Snapshot::read(&target).unwrap().layered.unwrap().alpha, 1);
        eprintln!("native-live-pointer-text: hidden-source=true hover=true leave=true committed-text=true");
        // 用户召回 A 后再次最小化；B 的新输入重新登记透明后台渲染，不能要求用户手动还原。
        owner.observe_reclaim(2, None, 10, |_| false).unwrap();
        let mut intent = crate::live_reclaim::IntentTracker::default();
        intent.record(30, 2);
        owner
            .observe_reclaim(fixture.root.0 as usize, intent.snapshot(31), 31, |_| false)
            .unwrap();
        until(&mut session, &mut owner, "source recalled", |_| {
            Snapshot::read(&target).unwrap().layered.is_none()
        });
        let _ = ShowWindow(fixture.root, SW_SHOWMINNOACTIVE);
        assert!(IsIconic(fixture.root).as_bool());
        INPUT.with(|input| input.borrow_mut().clear());
        SendMessageW(window, WM_MOUSEMOVE, None, Some(point));
        until(&mut session, &mut owner, "minimized source rearmed", |s| {
            !IsIconic(fixture.root).as_bool() && s.input_enabled()
        });
        SendMessageW(window, WM_LBUTTONDOWN, None, Some(point));
        SendMessageW(window, WM_LBUTTONUP, None, Some(point));
        until(&mut session, &mut owner, "click after rearm", |_| {
            fixture.received(WM_LBUTTONUP, None)
        });
        assert!(session.take_input_error().is_none());
        super::cycles::close(&mut session, &mut owner);
        assert!(
            !IsIconic(fixture.root).as_bool(),
            "close retains existing source recall semantics"
        );
    }
}

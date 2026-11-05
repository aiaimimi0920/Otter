//! 仅本测试进程的被动前台事件证明：允许用户切换其他程序，不允许receiver曾经激活。
use super::*;
use std::{cell::Cell, marker::PhantomData, rc::Rc};
use windows::Win32::{System::Threading::GetCurrentProcessId, UI::Accessibility::*};
thread_local! {
    static OBSERVED: Cell<(isize, bool)> = const { Cell::new((0, false)) };
}
pub(super) struct Watch {
    hook: HWINEVENTHOOK,
    _thread: PhantomData<Rc<()>>,
}
impl Watch {
    pub(super) unsafe fn new() -> Self {
        OBSERVED.with(|v| assert_eq!(v.get().0, 0));
        let hook = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            None,
            Some(event),
            GetCurrentProcessId(),
            0,
            WINEVENT_OUTOFCONTEXT,
        );
        assert!(!hook.is_invalid());
        OBSERVED.with(|v| v.set((hook.0 as isize, false)));
        let watch = Self {
            hook,
            _thread: PhantomData,
        };
        watch.assert_passive();
        watch
    }
    pub(super) unsafe fn assert_passive(&self) {
        drain_messages();
        let mut process = 0;
        GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut process));
        assert_ne!(
            process,
            GetCurrentProcessId(),
            "receiver must not own current foreground"
        );
        OBSERVED.with(|v| {
            assert_eq!(
                v.get(),
                (self.hook.0 as isize, false),
                "receiver was activated during the observed interval"
            )
        });
    }
}
impl Drop for Watch {
    fn drop(&mut self) {
        OBSERVED.with(|v| v.set((0, false))); // 先撤登记，迟到事件不能污染后续会话。
        assert!(unsafe { UnhookWinEvent(self.hook) }.as_bool());
    }
}
unsafe extern "system" fn event(
    hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    _: i32,
    _: i32,
    _: u32,
    _: u32,
) {
    if event != EVENT_SYSTEM_FOREGROUND {
        return;
    }
    let mut process = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut process));
    if process == GetCurrentProcessId() {
        OBSERVED.with(|v| {
            if v.get().0 == hook.0 as isize {
                v.set((hook.0 as isize, true));
            }
        });
    }
}
#[test]
fn passive_foreground_monitor_detects_own_event_and_unregisters_without_activating_window() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (files, _, _) = Files::new();
    unsafe {
        let (_class, mut app) = application(files.root());
        let watch = Watch::new();
        event(
            watch.hook,
            EVENT_OBJECT_SHOW,
            app.documents[0].window.0,
            0,
            0,
            0,
            0,
        );
        watch.assert_passive();
        // 仅调用检测回调证明正向断言，不真实SetForegroundWindow，不抢用户焦点。
        event(
            watch.hook,
            EVENT_SYSTEM_FOREGROUND,
            app.documents[0].window.0,
            0,
            0,
            0,
            0,
        );
        OBSERVED.with(|v| assert!(v.get().1));
        drop(watch);
        OBSERVED.with(|v| assert_eq!(v.get(), (0, false)));
        app.shutdown();
    }
}

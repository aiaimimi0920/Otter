//! 显式恢复的只读 WM_PAINT 诊断；只观察当前夹具 HWND，不改变消息、期限或呈现条件。
use super::*;
use std::cell::Cell;
use windows::Win32::{
    Graphics::Gdi::{GetUpdateRect, GetWindowRgnBox},
    System::Threading::GetCurrentThreadId,
    UI::Shell::{DefSubclassProc, GetWindowSubclass, RemoveWindowSubclass, SetWindowSubclass},
};

const ID: usize = 0x4f545054;
#[derive(Default)]
struct Counts {
    entered: Cell<u64>,
    returned: Cell<u64>,
    client: Cell<Option<(i32, i32)>>,
}
pub(super) struct Trace {
    hwnd: HWND,
    counts: Option<Box<Counts>>,
    procedure: isize,
}
impl Trace {
    pub(super) unsafe fn new(hwnd: HWND) -> Self {
        let counts = Box::<Counts>::default();
        assert!(SetWindowSubclass(
            hwnd,
            Some(observe),
            ID,
            (&*counts as *const Counts) as usize
        )
        .as_bool());
        Self {
            hwnd,
            counts: Some(counts),
            procedure: GetWindowLongPtrW(hwnd, GWLP_WNDPROC),
        }
    }
    pub(super) unsafe fn describe(&self) -> String {
        let counts = self.counts.as_ref().unwrap();
        let mut region = RECT::default();
        let region_type = GetWindowRgnBox(self.hwnd, &mut region);
        let mut data = 0;
        let same_subclass = GetWindowSubclass(self.hwnd, Some(observe), ID, Some(&mut data))
            .as_bool()
            && data == (&**counts as *const Counts) as usize;
        format!(
            "wm_paint={}/{}; last_client={:?}; update_pending={}; region_type={}; region_size={:?}; same_subclass={same_subclass}; same_procedure={}; owner_thread={}",
            counts.entered.get(), counts.returned.get(), counts.client.get(),
            GetUpdateRect(self.hwnd, None, false).as_bool(), region_type.0,
            (region.right - region.left, region.bottom - region.top),
            GetWindowLongPtrW(self.hwnd, GWLP_WNDPROC) == self.procedure,
            GetWindowThreadProcessId(self.hwnd, None) == GetCurrentThreadId(),
        )
    }
}
impl Drop for Trace {
    fn drop(&mut self) {
        unsafe {
            if !RemoveWindowSubclass(self.hwnd, Some(observe), ID).as_bool()
                && IsWindow(Some(self.hwnd)).as_bool()
            {
                // 异常移除失败时宁可保留这一个夹具计数器，也不把悬垂地址留给活窗口回调。
                let _ = Box::leak(self.counts.take().unwrap());
                eprintln!("projection-paint-trace: subclass removal failed; counter retained");
            }
        }
    }
}
unsafe extern "system" fn observe(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    let counts = &*(data as *const Counts);
    if message == WM_PAINT {
        counts.entered.set(counts.entered.get().saturating_add(1));
        let mut rect = RECT::default();
        counts.client.set(
            GetClientRect(hwnd, &mut rect)
                .ok()
                .map(|_| (rect.right - rect.left, rect.bottom - rect.top)),
        );
    }
    // 完整转发原过程，计数器不触碰 BeginPaint、update region 或 Canvas。
    let result = DefSubclassProc(hwnd, message, wp, lp);
    if message == WM_PAINT {
        counts.returned.set(counts.returned.get().saturating_add(1));
    }
    result
}

#[test]
fn projection_display_recovery_paint_trace_forwards_and_removes_its_own_subclass() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let (_class, mut app) = application();
        let doc = &app.documents[0];
        let before = doc.state.canvas.borrow().paints;
        let trace = Trace::new(doc.window.0);
        SendMessageW(doc.window.0, WM_PAINT, None, None);
        let counts = trace.counts.as_ref().unwrap();
        assert_eq!((counts.entered.get(), counts.returned.get()), (1, 1));
        assert!(counts.client.get().is_some_and(|(w, h)| w > 0 && h > 0));
        assert!(doc.state.canvas.borrow().paints > before);
        assert!(trace
            .describe()
            .contains("same_subclass=true; same_procedure=true; owner_thread=true"));
        drop(trace);
        let mut data = 0;
        assert!(!GetWindowSubclass(doc.window.0, Some(observe), ID, Some(&mut data)).as_bool());
        app.shutdown();
    }
}

//! 实际 SendMessageCallback/生产 Backend 的迟到拒绝，不安装源 hook、不伪造完成属性。
use super::*;
use crate::live_input_worker::{Command, Reason, Reply, Worker};

const GATE: windows::core::PCWSTR = w!("Otter.Test.AsyncRejectGate");
pub(super) unsafe fn handle(window: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> Option<LRESULT> {
    if message != otter_live_hover_hook::native_dispatch::message_id()
        || unsafe { GetPropW(window, GATE) }.0.is_null()
    {
        return None;
    }
    TRACE.with(|trace| {
        trace
            .borrow_mut()
            .push((window.0 as usize, message, wp.0, lp.0))
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    while !unsafe { GetPropW(window, GATE) }.0.is_null() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    // 私有入口由自有窗口过程直接返回，不领取请求、不写 FINISHED，宿主必须识别拒绝。
    Some(LRESULT(0))
}
struct Gate(HWND);
impl Drop for Gate {
    fn drop(&mut self) {
        let _ = unsafe { RemovePropW(self.0, GATE) };
    }
}
struct Window(HWND);
impl Drop for Window {
    fn drop(&mut self) {
        unsafe { DestroyWindow(self.0).unwrap() };
    }
}
fn reply(worker: &mut Worker<Error>) -> Reply<Error> {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(reply) = worker.poll_reply().unwrap() {
            return reply;
        }
        assert!(
            Instant::now() < deadline,
            "asynchronous backend result did not arrive"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
#[ignore = "interactive desktop; actual callback from owned alpha=1 source, no product DLL"]
fn late_unconfirmed_native_callback_reports_without_a_second_command() {
    let (mut source, target, child) = fixture();
    let owner_window = Window(
        unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!(""),
                WS_POPUP,
                -30000,
                -30000,
                30,
                30,
                None,
                None,
                None,
                None,
            )
        }
        .unwrap(),
    );
    let owner = crate::live_hover_owner::Owner::new(owner_window.0).unwrap();
    owner.control(true).unwrap();
    let control = owner.identity().capture_control().unwrap();
    let hwnd = HWND(child as *mut _);
    unsafe {
        SetPropW(
            hwnd,
            GATE,
            Some(HANDLE(std::ptr::without_provenance_mut(1))),
        )
        .unwrap()
    };
    let mut worker = Worker::start(move || {
        let setup = || -> Result<Backend, Error> {
            let destination =
                Destination::bind(&target, HWND(child as *mut _)).map_err(Error::Target)?;
            let mut backend = Backend::new_local(target, None)?.with_hover(Some(control));
            // 只建立精确请求和真实异步回调；不经过 prepare_native，不加载 DLL。
            backend.modal.set_owner(control);
            message::button_modal(
                &destination,
                Message::button([5, 5], 1, Button::Left, true, false).map_err(Error::Message)?,
                &mut backend.modal,
            )
            .map_err(Error::Message)?;
            if !backend.modal.pending() {
                return Err(Error::InvalidDelivery);
            }
            Ok(backend)
        };
        setup().map_err(|error| format!("{error:?}"))
    })
    .unwrap();
    // 先于 Worker 析构打开源线程闸门；源处理本身也有绝对期限。
    let gate = Gate(hwnd);
    worker.submit(Command::Enable).unwrap();
    let enabled = reply(&mut worker);
    assert!(matches!(enabled.command, Some(Command::Enable)));
    assert_eq!(enabled.result, Ok(live_input::Delivery::Completed));
    assert_eq!(worker.outstanding(), 0);
    drop(gate);
    let terminal = reply(&mut worker);
    assert!(terminal.command.is_none());
    assert_eq!(
        terminal.result,
        Err(live_input::Error::Backend(Error::Message(
            message::Error::DeliveryFailed
        )))
    );
    let deadline = Instant::now() + Duration::from_secs(3);
    let exit = loop {
        if let Some(exit) = worker.take_exit() {
            break exit;
        }
        assert!(Instant::now() < deadline, "failed backend did not retire");
        std::thread::sleep(Duration::from_millis(1));
    };
    assert_eq!(exit.reason, Reason::InputFailed);
    assert_eq!(exit.cleanup, Some(live_input::Cleanup::default()));
    let records = source.finish();
    assert_eq!(
        records
            .iter()
            .filter(|record| record.1 == otter_live_hover_hook::native_dispatch::message_id())
            .count(),
        1
    );
    assert!(!records.iter().any(|record| record.1 == WM_LBUTTONDOWN));
}

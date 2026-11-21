//! 带值身份的本地按钮/悬浮入口；不猜测原始鼠标报文来源，不重放未确认的输入。
use crate::native_protocol::{self as protocol, Request};
use std::{cell::Cell, sync::OnceLock};
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};

thread_local! {
    // 不持有 RefCell 借用或锁进入应用的任意/模态窗口过程。
    static CURRENT: Cell<Option<Request>> = const { Cell::new(None) };
    static COLLECTING: Cell<bool> = const { Cell::new(false) };
    static ANCHOR: Cell<Option<crate::native_popup_placement::Anchor>> = const { Cell::new(None) };
    static DEPTH: Cell<usize> = const { Cell::new(0) };
    static PUMP: Cell<usize> = const { Cell::new(0) };
}

pub(crate) fn place_popup(popup: HWND) -> windows::core::Result<()> {
    match ANCHOR.with(Cell::get) {
        Some(anchor) => anchor.place(popup),
        None => Ok(()),
    }
}

pub fn message_id() -> u32 {
    static MESSAGE: OnceLock<u32> = OnceLock::new();
    *MESSAGE.get_or_init(|| unsafe { RegisterWindowMessageW(protocol::MESSAGE_NAME) })
}

/// 首次窥视/取得消息即停止新弹窗收集，但不表示应用按钮已经返回。
pub fn message_pump_entered() {
    COLLECTING.with(|slot| slot.set(false));
    PUMP.with(|slot| slot.set(slot.get().wrapping_add(1)));
}

pub fn collecting() -> Option<Request> {
    if !COLLECTING.with(Cell::get) {
        return None;
    }
    CURRENT.with(Cell::get).filter(|request| {
        Request::read(request.owner, request.nonce, request.destination) == Some(*request)
    })
}

struct Running {
    request: Request,
    parent: Option<Request>,
    collecting: bool,
    anchor: Option<crate::native_popup_placement::Anchor>,
    pump: usize,
}
impl Drop for Running {
    fn drop(&mut self) {
        COLLECTING.with(|slot| slot.set(false));
        crate::native_popup_layer::finish(self.request);
        CURRENT.with(|slot| slot.set(self.parent));
        ANCHOR.with(|slot| slot.set(self.anchor));
        // 子事务若进入消息泵，父事务也已经越过同步 SHOW 的来源边界。
        COLLECTING.with(|slot| slot.set(self.collecting && PUMP.with(Cell::get) == self.pump));
        DEPTH.with(|slot| slot.set(slot.get() - 1));
    }
}

/// 只在接收目标的源线程执行。仅准已验证菜单后代嵌套，独立保存并恢复原事务。
/// Windows 管理远端 hook DLL 的在途执行引用；宿主取消不等于此函数已返回。
pub fn dispatch(owner: HWND, nonce: usize, receiving: HWND) -> windows::core::Result<bool> {
    let Some(request) = Request::read(owner, nonce, receiving) else {
        return Ok(false);
    };
    let parent = CURRENT.with(Cell::get);
    if DEPTH.with(Cell::get) >= 4
        || parent.is_some_and(|parent| !crate::native_nested::allowed(parent, request))
    {
        return Ok(false);
    }
    if !request.claim()? {
        return Ok(false);
    }
    let _running = Running {
        request,
        parent,
        collecting: COLLECTING.with(Cell::get),
        anchor: ANCHOR.with(Cell::get),
        pump: PUMP.with(Cell::get),
    };
    DEPTH.with(|slot| slot.set(slot.get() + 1));
    CURRENT.with(|slot| slot.set(Some(request)));
    COLLECTING.with(|slot| slot.set(true));
    ANCHOR.with(|slot| slot.set(crate::native_popup_placement::Anchor::read(request)));
    crate::native_activation::prepare(request)?;
    // 锚点查询/源队列激活可重入应用；仍只认原来源。claimed 不等于执行，拒绝不重放。
    if Request::read(owner, nonce, receiving) != Some(request) {
        return Ok(false);
    }
    // request 只包含白名单按钮与整数参数；这里不接受宿主地址或任意消息号。
    unsafe {
        SendMessageW(
            receiving,
            request.message,
            Some(WPARAM(request.wp)),
            Some(LPARAM(request.lp)),
        );
    }
    // 原窗口过程已返回；后续授权可撤销，但不能把已执行结果误报为没有完成。
    // finish 仍匹配原 cookie/完整报文/nonce/claim，不会确认已撤销或被替换的 publication。
    let _ = request.finish_returned()?;
    Ok(true)
}

/// # Safety
/// Windows WH_CALLWNDPROC 的非负 code 对应有效 CWPSTRUCT；始终继续原 hook 链。
#[no_mangle]
pub unsafe extern "system" fn otter_live_native_call(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code >= 0 && lp.0 != 0 {
        let call = &*(lp.0 as *const CWPSTRUCT);
        crate::native_popup::before(call);
        let id = message_id();
        if id != 0 && call.message == id {
            // 失败由宿主检查 CLAIMED/FINISHED 判定；不能重试已进入应用的按钮。
            let _ = dispatch(
                HWND(call.wParam.0 as *mut _),
                call.lParam.0 as usize,
                call.hwnd,
            );
        }
    }
    CallNextHookEx(None, code, wp, lp)
}

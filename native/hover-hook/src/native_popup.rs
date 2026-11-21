//! 同步 SHOW 的实例租约；不改应用 owner，不以同进程/时间接近追认已有窗口。
use crate::{native_dispatch, native_protocol as protocol, scope};
use std::sync::atomic::{AtomicUsize, Ordering};
use windows::{
    core::{w, PCWSTR},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

const COMMIT: PCWSTR = w!("Otter.Live.Popup.Commit.v1");
const OWNER: PCWSTR = w!("Otter.Live.Popup.Owner.v1");
const COOKIE: PCWSTR = w!("Otter.Live.Popup.Cookie.v1");
const EPOCH: PCWSTR = w!("Otter.Live.Popup.Epoch.v1");
const ROOT: PCWSTR = w!("Otter.Live.Popup.Root.v1");
const REQUEST: PCWSTR = w!("Otter.Live.Popup.Request.v1");
const PENDING: PCWSTR = w!("Otter.Live.Popup.PendingShow.v1");
const REVOKED: PCWSTR = w!("Otter.Live.Popup.Revoked.v1");
static NEXT: AtomicUsize = AtomicUsize::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lease {
    pub popup: HWND,
    pub owner: HWND,
    pub cookie: usize,
    pub epoch: usize,
    pub root: HWND,
    pub request: usize,
    pub instance: usize,
}
impl Lease {
    pub(crate) fn snapshot(popup: HWND) -> Self {
        Self {
            popup,
            owner: HWND(protocol::value(popup, OWNER) as *mut _),
            cookie: protocol::value(popup, COOKIE),
            epoch: protocol::value(popup, EPOCH),
            root: HWND(protocol::value(popup, ROOT) as *mut _),
            request: protocol::value(popup, REQUEST),
            instance: protocol::value(popup, COMMIT),
        }
    }

    pub fn read(popup: HWND) -> Option<Self> {
        let lease = Self::snapshot(popup);
        if lease.instance == 0
            || lease.request == 0
            || lease.cookie == 0
            || lease.epoch == 0
            || protocol::value(popup, REVOKED) == lease.instance
        {
            return None;
        }
        unsafe {
            let mut root_process = 0;
            let mut popup_process = 0;
            let root_thread = GetWindowThreadProcessId(lease.root, Some(&mut root_process));
            let popup_thread = GetWindowThreadProcessId(popup, Some(&mut popup_process));
            if root_thread == 0
                || popup_thread != root_thread
                || popup_process != root_process
                || GetAncestor(lease.root, GA_ROOT) != lease.root
                || GetAncestor(popup, GA_ROOT) != popup
                || popup == lease.root
                || GetWindowLongPtrW(popup, GWLP_HWNDPARENT) != 0
                || !IsWindowVisible(popup).as_bool()
                || !IsWindowVisible(lease.owner).as_bool()
                || IsIconic(lease.owner).as_bool()
                || IsIconic(lease.root).as_bool()
            {
                return None;
            }
        }
        (protocol::value(lease.owner, scope::COOKIE) == lease.cookie
            && protocol::value(lease.owner, protocol::EPOCH) == lease.epoch
            && protocol::value(popup, REVOKED) != lease.instance
            && Self::snapshot(popup) == lease)
            .then_some(lease)
    }

    pub fn valid(self) -> bool {
        Self::read(self.popup) == Some(self)
    }

    /// 仅供撤销后的状态恢复；此检查不授予输入权限，不要求 B 的控制 epoch 仍有效。
    pub fn same_instance(self) -> bool {
        self.instance != 0 && Self::snapshot(self.popup) == self
    }
}

fn revoke(popup: HWND) -> bool {
    // 先撤销输入权限，再恢复层级；恢复失败仍保留实例供重试，但不能在下次 SHOW 复活旧授权。
    let previous = Lease::snapshot(popup);
    if previous.instance != 0 {
        unsafe {
            if let Err(error) = SetPropW(popup, REVOKED, Some(HANDLE(previous.instance as *mut _)))
            {
                eprintln!("native-live-popup-revoke: {error}");
                return false;
            }
        }
    }
    if !crate::native_popup_layer::release(popup, false) {
        return false;
    }
    if Lease::snapshot(popup) != previous {
        return true; // 同步恢复重入了销毁/另一显示实例，不能清掉新实例的属性。
    }
    unsafe {
        // 只写本协议属性；不恢复/修改用户应用的窗口样式或 owner。
        for key in [
            COMMIT, OWNER, COOKIE, EPOCH, ROOT, REQUEST, PENDING, REVOKED,
        ] {
            let _ = RemovePropW(popup, key);
        }
    }
    true
}

fn shown(popup: HWND) -> windows::core::Result<()> {
    // 外层定位/层级调用尚未返回，内层 SHOW 不能提交另一代身份。这是预期的
    // 延后结算，不应将其记录成恢复失败；hide 已负责撤销旧输入资格。
    if crate::native_popup_layer::mutating(popup) {
        return Ok(());
    }
    let Some(request) = native_dispatch::collecting() else {
        return Ok(());
    };
    unsafe {
        // WINDOWPOSCHANGING 时必须尚不可见；已有可见窗口的普通移动不建立租约。
        if protocol::value(popup, PENDING) != request.nonce {
            return Ok(());
        }
        let mut process = 0;
        if popup == request.root
            || GetAncestor(popup, GA_ROOT) != popup
            || GetWindowThreadProcessId(popup, Some(&mut process))
                != windows::Win32::System::Threading::GetCurrentThreadId()
            || process != windows::Win32::System::Threading::GetCurrentProcessId()
            || !IsWindowVisible(popup).as_bool()
        {
            return Ok(());
        }
        if GetWindowLongPtrW(popup, GWLP_HWNDPARENT) != 0 {
            // Owned 菜单无需 lease，但也会按真实鼠标（B）创建。只处理本次
            // 同步 SHOW 且真实 owner 链回到原 root 的菜单，防止映射时再次叠加 A-B 偏移。
            // 先消费 PENDING，定位的同步重入不能对同一次 SHOW 再移动一次。
            let _ = RemovePropW(popup, PENDING);
            if crate::native_input_scope::owned_by(
                request.root.0 as usize,
                popup.0 as usize,
                process,
            ) && native_dispatch::collecting() == Some(request)
            {
                native_dispatch::place_popup(popup)?;
            }
            return Ok(());
        }
        if !revoke(popup) {
            return Err(windows::core::Error::new(
                E_FAIL,
                "Previous popup layer restore failed",
            ));
        }
        let Ok(instance) =
            NEXT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        else {
            return Ok(());
        };
        for (key, value) in [
            (OWNER, request.owner.0 as usize),
            (COOKIE, request.cookie),
            (EPOCH, request.epoch),
            (ROOT, request.root.0 as usize),
            (REQUEST, request.nonce),
        ] {
            SetPropW(popup, key, Some(HANDLE(value as *mut _)))?;
        }
        // 提交前再验，取消/新请求不得借迟到 SHOW 恢复租约。
        if native_dispatch::collecting() == Some(request) {
            SetPropW(popup, COMMIT, Some(HANDLE(instance as *mut _)))?;
            if let Some(lease) = Lease::read(popup) {
                crate::native_popup_layer::prepare(lease)?;
            }
        }
    }
    Ok(())
}

/// # Safety
/// call 来自有效 WH_CALLWNDPROC，WINDOWPOSCHANGING 的 LPARAM 在回调期间有效。
pub unsafe fn before(call: &CWPSTRUCT) {
    if call.message != WM_WINDOWPOSCHANGING || call.lParam.0 == 0 {
        return;
    }
    let position = &*(call.lParam.0 as *const WINDOWPOS);
    if position.flags.contains(SWP_HIDEWINDOW) {
        revoke(call.hwnd);
    } else if position.flags.contains(SWP_SHOWWINDOW)
        && !IsWindowVisible(call.hwnd).as_bool()
        && !crate::native_popup_layer::mutating(call.hwnd)
    {
        if let Some(request) = native_dispatch::collecting() {
            let _ = SetPropW(call.hwnd, PENDING, Some(HANDLE(request.nonce as *mut _)));
        }
    }
}

/// # Safety
/// 非负 code 的 lp 来自 WH_CALLWNDPROCRET，窗口位置消息的 LPARAM 由 Windows 保活。
#[no_mangle]
pub unsafe extern "system" fn otter_live_native_return(
    code: i32,
    wp: WPARAM,
    lp: LPARAM,
) -> LRESULT {
    if code >= 0 && lp.0 != 0 {
        let call = &*(lp.0 as *const CWPRETSTRUCT);
        if call.message == WM_NCDESTROY {
            crate::native_popup_layer::release(call.hwnd, true);
            revoke(call.hwnd);
        } else if call.message == WM_WINDOWPOSCHANGED && call.lParam.0 != 0 {
            let position = &*(call.lParam.0 as *const WINDOWPOS);
            if position.flags.contains(SWP_HIDEWINDOW) {
                revoke(call.hwnd);
            } else if position.flags.contains(SWP_SHOWWINDOW) {
                if let Err(error) = shown(call.hwnd) {
                    eprintln!("native-live-popup-show: {error}");
                }
            }
        }
    }
    CallNextHookEx(None, code, wp, lp)
}

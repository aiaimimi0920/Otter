//! 源线程菜单定位/层级变更与归还；同 HWND 重入不能丢恢复身份或重授显示租约。
use crate::native_popup::Lease;
use std::cell::RefCell;
use windows::{
    core::{w, PCWSTR},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

pub const TEMPORARY: PCWSTR = w!("Otter.Live.Popup.TemporaryTopmost.v1");
pub const MANAGED: PCWSTR = w!("Otter.Live.Popup.ManagedTopmost.v1");

thread_local! {
    static PROMOTED: RefCell<Vec<Lease>> = const { RefCell::new(Vec::new()) };
    static MUTATING: RefCell<Vec<Lease>> = const { RefCell::new(Vec::new()) };
}

struct Mutation(Lease);
impl Mutation {
    fn for_request(lease: Lease) -> windows::core::Result<Self> {
        // 两种拒绝（同 HWND 重入或有界槽已满）均发生在源窗口修改前。
        // 固定请求用明确 Pending 区分已结算的忙状态与未知的在途结果。
        Self::enter(lease).map_err(|_| {
            windows::core::Error::new(
                crate::native_layer_protocol::PENDING,
                "Popup window mutation is pending",
            )
        })
    }
    fn enter(lease: Lease) -> windows::core::Result<Self> {
        MUTATING.with(|entries| {
            let mut entries = entries.borrow_mut();
            if entries.len() >= 16 || entries.iter().any(|entry| entry.popup == lease.popup) {
                return Err(windows::core::Error::new(
                    E_ABORT,
                    "Popup window mutation reentered",
                ));
            }
            entries.push(lease);
            Ok(Self(lease))
        })
    }
}
impl Drop for Mutation {
    fn drop(&mut self) {
        MUTATING.with(|entries| entries.borrow_mut().retain(|entry| *entry != self.0));
    }
}

pub(crate) fn mutating(popup: HWND) -> bool {
    MUTATING.with(|entries| entries.borrow().iter().any(|entry| entry.popup == popup))
}

fn set_topmost(popup: HWND, enabled: bool) -> windows::core::Result<()> {
    unsafe {
        SetWindowPos(
            popup,
            Some(if enabled {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            }),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
        )?;
        // 桌面策略或源窗口过程可能忽略 Z-order，Win32 返回成功仍须核对实际结果。
        if (GetWindowLongPtrW(popup, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0) != enabled {
            return Err(windows::core::Error::new(
                E_FAIL,
                "Popup topmost change was not applied",
            ));
        }
    }
    Ok(())
}

pub(crate) fn prepare(lease: Lease) -> windows::core::Result<()> {
    if !lease.valid() {
        return Ok(());
    }
    let result = {
        // 定位同样会同步进入应用；必须在定位之前保护这一代显示身份，而不是
        // 等定位返回后重新读取一个可能已被 hide/show 替换的新 lease。
        let _mutation = Mutation::enter(lease)?;
        (|| {
            crate::native_dispatch::place_popup(lease.popup)?;
            promote(lease)
        })()
    };
    if result.is_err() || !lease.valid() {
        // 重入的 hide 只撤输入、不清身份。外层变更返回后再归还已实际借用的
        // 层级；原本置顶或只定位过的窗口没有本协议标志，不能擅自降级。
        let tracked = PROMOTED.with(|entries| {
            let mut entries = entries.borrow_mut();
            let before = entries.len();
            entries.retain(|entry| *entry != lease);
            before != entries.len()
        });
        if tracked
            || [TEMPORARY, MANAGED]
                .iter()
                .any(|key| crate::native_protocol::value(lease.popup, *key) == lease.instance)
        {
            restore(lease)?;
        }
    }
    result
}

fn promote(lease: Lease) -> windows::core::Result<()> {
    unsafe {
        if !lease.valid()
            || GetWindowLongPtrW(lease.popup, GWL_EXSTYLE) as u32 & WS_EX_NOACTIVATE.0 == 0
            || GetWindowLongPtrW(lease.popup, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0
            || GetWindowLongPtrW(lease.root, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 == 0
        {
            return Ok(());
        }
        let retained = PROMOTED.with(|entries| {
            let mut entries = entries.borrow_mut();
            if entries.len() >= 4 {
                return false;
            }
            entries.push(lease);
            true
        });
        if !retained {
            return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                0x800700AAu32 as i32,
            )));
        }
        // 状态先入槽且不跨 Win32 持借用：SetWindowPos 可重入 hide/destroy。
        SetPropW(
            lease.popup,
            TEMPORARY,
            Some(HANDLE(lease.instance as *mut _)),
        )?;
        set_topmost(lease.popup, true)?;
    }
    Ok(())
}

fn restore(lease: Lease) -> windows::core::Result<()> {
    // 取消可先撤销 B epoch，所以这里只用源侧显示实例防 HWND 复用，不要求控制仍有效。
    if !lease.same_instance() {
        return Ok(());
    }
    // SetWindowPos 会同步重入应用。内层 hide/show 必须保留身份且拒绝重获授权，
    // 不能在外层确认恢复之前清空恢复标志或递归恢复同一 HWND。
    let _mutation = Mutation::enter(lease)?;
    restore_inner(lease)
}

// 调用方已持原显示实例的 Mutation；不能递归进入 guard 或跨 Win32 持 Vec 借用。
fn restore_inner(lease: Lease) -> windows::core::Result<()> {
    unsafe {
        if GetWindowLongPtrW(lease.popup, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0 {
            set_topmost(lease.popup, false).or_else(|error| {
                if lease.same_instance() {
                    Err(error)
                } else {
                    Ok(())
                }
            })?;
        }
        if lease.same_instance() {
            let _ = RemovePropW(lease.popup, TEMPORARY);
            let _ = RemovePropW(lease.popup, MANAGED);
        }
    }
    Ok(())
}

pub(crate) fn acquire_managed(
    lease: Lease,
    active: impl Fn() -> bool,
) -> windows::core::Result<()> {
    let changed = || windows::core::Error::new(E_ABORT, "Popup layer request changed");
    if !lease.valid() {
        return Err(changed());
    }
    let _mutation = Mutation::for_request(lease)?;
    if !active() {
        return Err(changed());
    }
    unsafe {
        let style = GetWindowLongPtrW(lease.popup, GWL_EXSTYLE) as u32;
        let owned = [TEMPORARY, MANAGED]
            .iter()
            .any(|key| crate::native_protocol::value(lease.popup, *key) == lease.instance);
        if style & WS_EX_NOACTIVATE.0 == 0
            || GetWindowLongPtrW(lease.root, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 == 0
            || style & WS_EX_TOPMOST.0 != 0 && !owned
        {
            return Err(changed()); // 原应用自己的 TOPMOST 不能被登记借用。
        }
        SetPropW(lease.popup, MANAGED, Some(HANDLE(lease.instance as *mut _)))?;
        let result = if style & WS_EX_TOPMOST.0 == 0 {
            set_topmost(lease.popup, true)
        } else {
            Ok(())
        };
        if result.is_err() || !active() || !lease.valid() {
            if lease.same_instance()
                && crate::native_protocol::value(lease.popup, MANAGED) == lease.instance
            {
                restore_inner(lease)?;
            }
            return result.and_then(|()| Err(changed()));
        }
    }
    Ok(())
}

pub(crate) fn restore_managed(lease: Lease) -> windows::core::Result<()> {
    if !lease.same_instance() {
        return Ok(()); // 原对象已消失，不对后来显示的实例修改层级。
    }
    let _mutation = Mutation::for_request(lease)?;
    if crate::native_protocol::value(lease.popup, MANAGED) != lease.instance {
        return Ok(()); // 尚未接管时不能收回源在途事务的临时层级。
    }
    restore_inner(lease)
}

pub(crate) fn release(popup: HWND, destroying: bool) -> bool {
    if !destroying && mutating(popup) {
        return false;
    }
    let lease = PROMOTED.with(|entries| {
        let mut entries = entries.borrow_mut();
        let index = entries.iter().position(|lease| lease.popup == popup)?;
        Some(entries.swap_remove(index))
    });
    let lease = lease.or_else(|| {
        let lease = Lease::snapshot(popup);
        (lease.same_instance()
            && [MANAGED, TEMPORARY]
                .iter()
                .any(|key| crate::native_protocol::value(popup, *key) == lease.instance))
        .then_some(lease)
    });
    if let Some(lease) = lease {
        if destroying {
            unsafe {
                let _ = RemovePropW(popup, MANAGED);
                let _ = RemovePropW(popup, TEMPORARY);
            }
        } else if let Err(error) = restore(lease) {
            eprintln!("native-live-popup-layer-restore: {error}");
            return false; // 保留窗口上的显示身份和恢复标志，不能先 revoke 丢失责任。
        }
    }
    true
}

pub(crate) fn finish(request: crate::native_protocol::Request) {
    // 子菜单按钮返回不得恢复仍在运行的父菜单层级；不跨窗口调用持有借用。
    let leases = PROMOTED.with(|entries| {
        let mut entries = entries.borrow_mut();
        let (finished, remaining) = std::mem::take(&mut *entries)
            .into_iter()
            .partition::<Vec<_>, _>(|lease| {
                lease.owner == request.owner
                    && lease.cookie == request.cookie
                    && lease.epoch == request.epoch
                    && lease.request == request.nonce
            });
        *entries = remaining;
        finished
    });
    for lease in leases {
        if crate::native_protocol::value(lease.popup, MANAGED) != lease.instance {
            if let Err(error) = restore(lease) {
                eprintln!("native-live-popup-layer-finish: {error}");
            }
        } // 宿主仅在恢复 journal ACK 后写 MANAGED；hide/destroy 仍由 release 收尾。
    }
}

#[cfg(test)]
#[path = "native_popup_layer_result_tests.rs"]
mod result_tests;

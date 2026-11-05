//! 原生文件拖出的 OLE 边界；不写剪贴板、不移动或删除来源文件。
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use windows::{
    core::{implement, Error, Result, BOOL, HRESULT},
    Win32::{
        Foundation::*,
        System::{Ole::*, SystemServices::*},
    },
};

#[path = "drag_transport_data.rs"]
mod data;
#[path = "drag_transport_preview.rs"]
mod preview;

struct Apartment;
impl Apartment {
    fn new() -> Result<Self> {
        unsafe {
            OleInitialize(None)?;
        }
        Ok(Self)
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            OleUninitialize();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Copied,
    Cancelled,
}

#[implement(IDropSource)]
struct Source {
    cancel: Arc<AtomicBool>,
}
impl IDropSource_Impl for Source_Impl {
    fn QueryContinueDrag(&self, escape: BOOL, keys: MODIFIERKEYS_FLAGS) -> HRESULT {
        continuation(escape.as_bool(), keys, self.cancel.load(Ordering::Acquire))
    }
    fn GiveFeedback(&self, _: DROPEFFECT) -> HRESULT {
        DRAGDROP_S_USEDEFAULTCURSORS
    }
}

fn continuation(escape: bool, keys: MODIFIERKEYS_FLAGS, cancelled: bool) -> HRESULT {
    if escape || cancelled || keys.contains(MK_RBUTTON) {
        DRAGDROP_S_CANCEL
    } else if !keys.contains(MK_LBUTTON) {
        DRAGDROP_S_DROP
    } else {
        S_OK
    }
}

/// 在拥有手势的 STA UI 线程调用；调用方必须保证文件是自己的稳定导出快照。
/// OLE 会运行嵌套消息循环：不得跨本调用持有 UI RefCell 借用；窗口关闭须设置 cancel。
/// 仅允许 COPY。返回只说明 OLE 结果，不证明目标应用已持久化数据。
/// 本函数不管理文件寿命；调用方不得在目标仍可能读取时撤销文件。
pub fn drag_files(paths: &[PathBuf], cancel: Arc<AtomicBool>) -> Result<Outcome> {
    drag_files_with_preview(paths, cancel, None)
}
pub fn drag_files_with_preview(
    paths: &[PathBuf],
    cancel: Arc<AtomicBool>,
    image: Option<&super::drag_preview::Preview>,
) -> Result<Outcome> {
    if cancel.load(Ordering::Acquire) {
        return Ok(Outcome::Cancelled);
    }
    let _apartment = Apartment::new()?;
    let payload = data::prepare(paths)?;
    if let Some(image) = image {
        // 预览是增强显示；Shell helper 不可用不能阻止 CF_HDROP 文件复制。
        if let Err(error) = preview::attach(&payload.object, image) {
            eprintln!("native-drag-preview: {error}");
        }
    }
    let source: IDropSource = Source {
        cancel: cancel.clone(),
    }
    .into();
    if cancel.load(Ordering::Acquire) {
        return Ok(Outcome::Cancelled);
    }
    let mut effect = DROPEFFECT_NONE;
    let status = unsafe { DoDragDrop(&payload.object, &source, DROPEFFECT_COPY, &mut effect) };
    outcome(status, effect)
}

fn outcome(status: HRESULT, effect: DROPEFFECT) -> Result<Outcome> {
    if status == DRAGDROP_S_CANCEL {
        return Ok(Outcome::Cancelled);
    }
    status.ok()?;
    if status == DRAGDROP_S_DROP && effect == DROPEFFECT_COPY {
        Ok(Outcome::Copied)
    } else if status == DRAGDROP_S_DROP && effect == DROPEFFECT_NONE {
        Ok(Outcome::Cancelled)
    } else {
        Err(Error::new(E_FAIL, "Unexpected native drag completion"))
    }
}

#[cfg(test)]
#[path = "drag_transport_tests.rs"]
mod tests;

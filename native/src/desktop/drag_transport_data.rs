//! 只使用标准 Shell IDataObject 承载有界 CF_HDROP；不访问路径指向的磁盘内容。
use std::path::PathBuf;
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*,
        System::{Com::*, Memory::*},
        UI::Shell::SHCreateDataObject,
    },
};

pub(super) struct Payload {
    pub object: IDataObject,
    // 先释放本地 COM 引用，再归还本次数据准备预算；接收方保留的 COM 数据由系统管理。
    _lease: crate::byte_budget::Lease,
}
struct Memory(HGLOBAL);
impl Drop for Memory {
    fn drop(&mut self) {
        unsafe {
            let _ = GlobalFree(Some(self.0));
        }
    }
}
struct Locked(HGLOBAL);
impl Drop for Locked {
    fn drop(&mut self) {
        unsafe {
            let _ = GlobalUnlock(self.0);
        }
    }
}

pub(super) fn format() -> FORMATETC {
    FORMATETC {
        cfFormat: 15, // CF_HDROP 是预定义格式，不调用 RegisterClipboardFormat。
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
        ..Default::default()
    }
}

pub(super) fn prepare(paths: &[PathBuf]) -> Result<Payload> {
    let bytes = super::super::paste_input::file::encode(paths)?;
    // 编码 Vec 与交付给 Shell 的 HGLOBAL 均有上限；编码器限制总包长 128 KiB。
    let mut lease = crate::pixels::reserve(bytes.len() * 2)
        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Native drag data exceeds budget"))?;
    unsafe {
        // 接收方能查看 GlobalSize，连分配器对齐余量也清零，避免泄露未初始化字节。
        let memory = Memory(GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, bytes.len())?);
        if !lease.grow_to(bytes.len() + GlobalSize(memory.0)) {
            return Err(Error::new(
                E_OUTOFMEMORY,
                "Native drag allocation exceeds budget",
            ));
        }
        let pointer = GlobalLock(memory.0);
        if pointer.is_null() {
            return Err(Error::from_win32());
        }
        let locked = Locked(memory.0);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer.cast(), bytes.len());
        drop(locked);
        let object: IDataObject = SHCreateDataObject(None, None, None)?;
        let medium = STGMEDIUM {
            tymed: TYMED_HGLOBAL.0 as u32,
            u: STGMEDIUM_0 { hGlobal: memory.0 },
            ..Default::default()
        };
        // 标准 Shell 对象不支持 fRelease=false；成功后只由 IDataObject 释放 HGLOBAL。
        // 失败仍由本地 RAII 回收，禁止成功后再次 GlobalFree。
        object.SetData(&format(), &medium, true)?;
        std::mem::forget(memory);
        Ok(Payload {
            object,
            _lease: lease,
        })
    }
}

//! 标准 Shell IDataObject 的有界 CF_HDROP 与逻辑操作回执；不访问路径指向的磁盘内容。
use std::path::PathBuf;
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*,
        System::{
            Com::*,
            DataExchange::RegisterClipboardFormatW,
            Memory::*,
            Ole::{ReleaseStgMedium, DROPEFFECT},
        },
        UI::Shell::{SHCreateDataObject, CFSTR_LOGICALPERFORMEDDROPEFFECT},
    },
};

pub(super) struct Payload {
    pub object: IDataObject,
    // 先释放本地 COM 引用，再归还本次数据准备预算；接收方保留的 COM 数据由系统管理。
    _lease: crate::byte_budget::Lease,
}

struct Medium(STGMEDIUM);
impl Drop for Medium {
    fn drop(&mut self) {
        unsafe {
            ReleaseStgMedium(&mut self.0);
        }
    }
}
pub(super) fn logical_format() -> Result<FORMATETC> {
    let id = unsafe { RegisterClipboardFormatW(CFSTR_LOGICALPERFORMEDDROPEFFECT) };
    if id == 0 {
        return Err(Error::from_win32());
    }
    Ok(FORMATETC {
        cfFormat: id as u16,
        ..format()
    })
}
/// Shell 优化移动后原 pdwEffect 可以是 NONE；逻辑回执才表示用户实际完成的操作。
pub(super) fn logical_effect(object: &IDataObject) -> Result<Option<DROPEFFECT>> {
    unsafe {
        let medium = match object.GetData(&logical_format()?) {
            Ok(value) => Medium(value),
            Err(error) if error.code() == DV_E_FORMATETC => return Ok(None),
            Err(error) => return Err(error),
        };
        if medium.0.tymed != TYMED_HGLOBAL.0 as u32
            || GlobalSize(medium.0.u.hGlobal) < std::mem::size_of::<u32>()
        {
            return Err(Error::new(
                E_INVALIDARG,
                "Invalid logical drop effect medium",
            ));
        }
        let pointer = GlobalLock(medium.0.u.hGlobal);
        if pointer.is_null() {
            return Err(Error::from_win32());
        }
        let _locked = Locked(medium.0.u.hGlobal);
        Ok(Some(DROPEFFECT(std::ptr::read_unaligned(
            pointer.cast::<u32>(),
        ))))
    }
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

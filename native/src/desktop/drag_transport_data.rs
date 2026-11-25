//! Bounded CF_HDROP and logical-operation receipts for standard Shell IDataObject; no access to disk content referenced by paths.
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
    // Release local COM references before returning preparation budgets; the system manages COM data retained by receivers.
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
/// After Shell-optimized moves, original pdwEffect may be NONE; logical receipts describe the operation users actually completed.
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
        cfFormat: 15, // CF_HDROP is predefined; do not call RegisterClipboardFormat.
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
        ..Default::default()
    }
}

pub(super) fn prepare(paths: &[PathBuf]) -> Result<Payload> {
    let bytes = super::super::paste_input::file::encode(paths)?;
    // Both encoding Vec and HGLOBAL delivered to Shell are bounded; the encoder caps total packet size at 128 KiB.
    let mut lease = crate::pixels::reserve(bytes.len() * 2)
        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Native drag data exceeds budget"))?;
    unsafe {
        // Receivers can inspect GlobalSize; zero even allocator-alignment slack to avoid exposing uninitialized bytes.
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
        // Standard Shell objects do not support fRelease=false; after success only IDataObject frees HGLOBAL.
        // Local RAII still reclaims failures; never GlobalFree again after success.
        object.SetData(&format(), &medium, true)?;
        std::mem::forget(memory);
        Ok(Payload {
            object,
            _lease: lease,
        })
    }
}

//! Shared system ICU leases; fixed callers supply locale/attributes, with no working-directory DLL loading or silent fallback to different ordering.
use std::ffi::c_void;
use windows::{
    core::{s, w, Error, Result, PCSTR},
    Win32::{
        Foundation::{FreeLibrary, E_FAIL, HMODULE},
        Globalization::{UColAttribute, UColAttributeValue},
        System::LibraryLoader::{GetProcAddress, LoadLibraryExW, LOAD_LIBRARY_SEARCH_SYSTEM32},
    },
};
type Open = unsafe extern "C" fn(*const u8, *mut i32) -> *mut c_void;
type Close = unsafe extern "C" fn(*mut c_void);
type SetAttribute = unsafe extern "C" fn(*mut c_void, i32, i32, *mut i32);
type Compare = unsafe extern "C" fn(*const c_void, *const u16, i32, *const u16, i32) -> i32;
struct Library(HMODULE);
impl Drop for Library {
    fn drop(&mut self) {
        unsafe {
            let _ = FreeLibrary(self.0);
        }
    }
}
pub(super) struct Collator {
    pointer: *mut c_void,
    close: Close,
    compare: Compare,
    _library: Library,
}
impl Drop for Collator {
    fn drop(&mut self) {
        unsafe {
            (self.close)(self.pointer);
        }
    }
}
fn failure(status: i32) -> Error {
    Error::new(E_FAIL, format!("Native ICU collation failed: {status}"))
}
impl Collator {
    pub(super) fn new(
        locale: PCSTR,
        attributes: &[(UColAttribute, UColAttributeValue)],
    ) -> Result<Self> {
        unsafe {
            let library = Library(LoadLibraryExW(
                w!("icu.dll"),
                None,
                LOAD_LIBRARY_SEARCH_SYSTEM32,
            )?);
            let open: Open = std::mem::transmute(
                GetProcAddress(library.0, s!("ucol_open")).ok_or_else(Error::from_win32)?,
            );
            let close: Close = std::mem::transmute(
                GetProcAddress(library.0, s!("ucol_close")).ok_or_else(Error::from_win32)?,
            );
            let compare: Compare = std::mem::transmute(
                GetProcAddress(library.0, s!("ucol_strcoll")).ok_or_else(Error::from_win32)?,
            );
            let set: SetAttribute = std::mem::transmute(
                GetProcAddress(library.0, s!("ucol_setAttribute")).ok_or_else(Error::from_win32)?,
            );
            let mut status = 0;
            let pointer = open(locale.as_ptr(), &mut status);
            if pointer.is_null() {
                return Err(failure(status));
            }
            let result = Self {
                pointer,
                close,
                compare,
                _library: library,
            };
            if status > 0 {
                return Err(failure(status));
            }
            for (attribute, value) in attributes {
                set(pointer, attribute.0, value.0, &mut status);
                if status > 0 {
                    return Err(failure(status));
                }
            }
            Ok(result)
        }
    }
    pub(super) fn compare(&self, left: &[u16], right: &[u16]) -> std::cmp::Ordering {
        unsafe {
            (self.compare)(
                self.pointer,
                left.as_ptr(),
                left.len() as i32,
                right.as_ptr(),
                right.len() as i32,
            )
            .cmp(&0)
        }
    }
}

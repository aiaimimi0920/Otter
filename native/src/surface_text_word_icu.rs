//! 系统 ICU 的有界词边界查询；仅按需加载系统 DLL，不改变进程启动依赖。
use std::ffi::c_void;
use windows::{
    core::{s, w, Error, Result},
    Win32::{
        Foundation::{FreeLibrary, HMODULE},
        System::LibraryLoader::{GetProcAddress, LoadLibraryExW, LOAD_LIBRARY_SEARCH_SYSTEM32},
    },
};
type Open = unsafe extern "C" fn(i32, *const u8, *const u16, i32, *mut i32) -> *mut c_void;
type Advance = unsafe extern "C" fn(*mut c_void) -> i32;
type Close = unsafe extern "C" fn(*mut c_void);
struct Library(HMODULE);
impl Drop for Library {
    fn drop(&mut self) {
        unsafe {
            let _ = FreeLibrary(self.0);
        }
    }
}
struct IteratorOwner {
    pointer: *mut c_void,
    close: Close,
}
impl Drop for IteratorOwner {
    fn drop(&mut self) {
        unsafe {
            (self.close)(self.pointer);
        }
    }
}
pub(super) fn boundaries(text: &[u16]) -> Result<Vec<u32>> {
    if text.len() > 65536 {
        return Err(super::invalid());
    }
    if text.is_empty() {
        return Ok(vec![0]);
    }
    unsafe {
        // Never search the application directory or PATH. Missing ICU is an operation error,
        // not a static import failure during application startup. No silent ASCII fallback.
        let library = Library(LoadLibraryExW(
            w!("icu.dll"),
            None,
            LOAD_LIBRARY_SEARCH_SYSTEM32,
        )?);
        let open: Open = std::mem::transmute(
            GetProcAddress(library.0, s!("ubrk_open")).ok_or_else(Error::from_win32)?,
        );
        let first: Advance = std::mem::transmute(
            GetProcAddress(library.0, s!("ubrk_first")).ok_or_else(Error::from_win32)?,
        );
        let next: Advance = std::mem::transmute(
            GetProcAddress(library.0, s!("ubrk_next")).ok_or_else(Error::from_win32)?,
        );
        let close: Close = std::mem::transmute(
            GetProcAddress(library.0, s!("ubrk_close")).ok_or_else(Error::from_win32)?,
        );
        let mut status = 0;
        let pointer = open(
            1,
            s!("und").as_ptr(),
            text.as_ptr(),
            text.len() as i32,
            &mut status,
        );
        if pointer.is_null() {
            return Err(super::invalid());
        }
        // The iterator borrows text; its Drop runs before the DLL owner and caller's slice.
        let iterator = IteratorOwner { pointer, close };
        if status > 0 {
            return Err(super::invalid());
        }
        let mut result = Vec::new();
        let mut at = first(iterator.pointer);
        while at != -1 {
            if at < 0
                || at as usize > text.len()
                || result.len() > text.len()
                || result.last().is_some_and(|previous| *previous >= at as u32)
            {
                return Err(super::invalid());
            }
            result.push(at as u32);
            at = next(iterator.pointer);
        }
        if result.first() != Some(&0) || result.last() != Some(&(text.len() as u32)) {
            return Err(super::invalid());
        }
        Ok(result)
    }
}

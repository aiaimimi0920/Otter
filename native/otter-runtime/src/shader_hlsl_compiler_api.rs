//! 编译器仅在 child 按 SYSTEM32 加载，避免静态导入让父 UI 常驻 compiler 或搜索邻接 DLL。
use super::*;
use std::ffi::c_void;
use windows::{
    core::{w, GUID, HRESULT, PCSTR},
    Win32::{
        Foundation::{FreeLibrary, HMODULE},
        System::LibraryLoader::{GetProcAddress, LoadLibraryExW, LOAD_LIBRARY_SEARCH_SYSTEM32},
    },
};
type Compile = unsafe extern "system" fn(
    *const c_void,
    usize,
    PCSTR,
    *const windows::Win32::Graphics::Direct3D::D3D_SHADER_MACRO,
    *mut c_void,
    PCSTR,
    PCSTR,
    u32,
    u32,
    *mut *mut c_void,
    *mut *mut c_void,
) -> HRESULT;
pub(super) type Reflect =
    unsafe extern "system" fn(*const c_void, usize, *const GUID, *mut *mut c_void) -> HRESULT;
pub(super) struct Compiler {
    pub compile: Compile,
    pub reflect: Reflect,
    _library: Library,
}
struct Library(HMODULE);
impl Drop for Library {
    fn drop(&mut self) {
        unsafe {
            let _ = FreeLibrary(self.0);
        }
    }
}
impl Compiler {
    pub fn new() -> Result<Self> {
        unsafe {
            let library = Library(
                LoadLibraryExW(w!("d3dcompiler_47.dll"), None, LOAD_LIBRARY_SEARCH_SYSTEM32)
                    .map_err(|e| api(Stage::Pipeline, e))?,
            );
            let compile = GetProcAddress(library.0, windows::core::s!("D3DCompile"))
                .ok_or(Error::Unsupported)?;
            let reflect = GetProcAddress(library.0, windows::core::s!("D3DReflect"))
                .ok_or(Error::Unsupported)?;
            Ok(Self {
                compile: std::mem::transmute::<unsafe extern "system" fn() -> isize, Compile>(
                    compile,
                ),
                reflect: std::mem::transmute::<unsafe extern "system" fn() -> isize, Reflect>(
                    reflect,
                ),
                _library: library,
            })
        }
    }
}

//! ANGLE 可信 bundle 的唯一 DLL owner；发行指纹必须来自调用方可信清单，不能来自 Art 响应。
use super::{Error, Result};
#[cfg(test)]
use sha2::{Digest, Sha256};
use std::{
    ffi::{c_void, CStr},
    fs::File,
    path::Path,
};
#[cfg(test)]
use std::{fs::OpenOptions, io::Read, os::windows::fs::OpenOptionsExt};
#[cfg(test)]
use windows::{
    core::HSTRING,
    Win32::{
        Storage::FileSystem::FILE_SHARE_READ,
        System::LibraryLoader::{GetModuleFileNameW, LoadLibraryExW, LOAD_LIBRARY_SEARCH_SYSTEM32},
    },
};
use windows::{
    core::PCSTR,
    Win32::{
        Foundation::{FreeLibrary, HMODULE},
        System::LibraryLoader::GetProcAddress,
    },
};

#[derive(Clone, Copy, Debug)]
pub struct Fingerprint {
    pub bytes: u64,
    pub sha256: [u8; 32],
}
#[derive(Clone, Copy, Debug)]
pub struct Identity {
    pub egl: Fingerprint,
    pub gles: Fingerprint,
}
pub(super) struct Library {
    module: HMODULE,
    // 禁止 DLL 在核验与加载间或整个映射寿命内被改写/删除。
    _file: File,
}
impl Drop for Library {
    fn drop(&mut self) {
        unsafe {
            let _ = FreeLibrary(self.module);
        }
    }
}
impl Library {
    pub(super) unsafe fn symbol(&self, name: &CStr) -> Result<*const c_void> {
        GetProcAddress(self.module, PCSTR(name.as_ptr().cast()))
            .map(|function| function as *const c_void)
            .ok_or(Error::Library)
    }
}
pub(super) struct Libraries {
    pub egl: Library,
    pub gles: Library,
}
#[cfg(test)]
fn verified(path: &Path, identity: Fingerprint) -> Result<File> {
    if identity.bytes == 0 || identity.bytes > 32 * 1024 * 1024 {
        return Err(Error::Library);
    }
    if std::fs::symlink_metadata(path)
        .map_err(|_| Error::Library)?
        .file_type()
        .is_symlink()
    {
        return Err(Error::Library);
    }
    let mut file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ.0)
        .open(path)
        .map_err(|_| Error::Library)?;
    let metadata = file.metadata().map_err(|_| Error::Library)?;
    if !metadata.is_file() || metadata.len() != identity.bytes {
        return Err(Error::Library);
    }
    let mut digest = Sha256::new();
    let mut chunk = [0u8; 16384];
    loop {
        let count = file.read(&mut chunk).map_err(|_| Error::Library)?;
        if count == 0 {
            break;
        }
        digest.update(&chunk[..count]);
    }
    if digest.finalize().as_slice() != identity.sha256 {
        return Err(Error::Library);
    }
    Ok(file)
}
impl Libraries {
    // 历史验证保留在测试目标，生产库不编译 ANGLE 的文件核验及 DLL 加载路径。
    #[cfg(not(test))]
    pub fn load(_directory: &Path, _identity: Identity) -> Result<Self> {
        Err(Error::Library)
    }
    #[cfg(test)]
    pub fn load(directory: &Path, identity: Identity) -> Result<Self> {
        // 调用方必须提供可信 ACL 的固定目录；哈希不授权执行来自 Art 的 DLL。
        let directory = directory.canonicalize().map_err(|_| Error::Library)?;
        // 清单只允许两个 DLL；目录枚举不是防并发写入的安全边界。
        for entry in std::fs::read_dir(&directory).map_err(|_| Error::Library)? {
            let entry = entry.map_err(|_| Error::Library)?;
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if name.ends_with(".dll") && name != "libegl.dll" && name != "libglesv2.dll" {
                return Err(Error::Library);
            }
        }
        let egl_path = directory.join("libEGL.dll");
        let gles_path = directory.join("libGLESv2.dll");
        let egl_file = verified(&egl_path, identity.egl)?;
        let gles_file = verified(&gles_path, identity.gles)?;
        let load = |path: &Path, file| -> Result<Library> {
            let module =
                unsafe { LoadLibraryExW(&HSTRING::from(path), None, LOAD_LIBRARY_SEARCH_SYSTEM32) }
                    .map_err(|_| Error::Library)?;
            let library = Library {
                module,
                _file: file,
            };
            let mut actual = [0u16; 32768];
            let count = unsafe { GetModuleFileNameW(Some(module), &mut actual) } as usize;
            if count == 0
                || count >= actual.len()
                || Path::new(&String::from_utf16_lossy(&actual[..count]))
                    .canonicalize()
                    .ok()
                    .as_deref()
                    != Some(path)
            {
                return Err(Error::Library);
            }
            Ok(library)
        };
        let gles = load(&gles_path, gles_file)?;
        let egl = load(&egl_path, egl_file)?;
        Ok(Self { egl, gles })
    }
}

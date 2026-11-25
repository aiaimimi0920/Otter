//! 只读系统版本探测；动态解析 RtlGetVersion，避免应用 manifest 对版本助手的影响。
use windows::{
    core::{s, w},
    Win32::{
        Foundation::NTSTATUS,
        System::{
            LibraryLoader::{GetModuleHandleW, GetProcAddress},
            SystemInformation::OSVERSIONINFOW,
        },
    },
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub build: u32,
}
impl Version {
    pub fn windows_11_or_newer(self) -> bool {
        self.major > 10 || (self.major == 10 && (self.minor > 0 || self.build >= 22000))
    }
}
#[derive(Debug)]
pub enum Error {
    Api(windows::core::Error),
    MissingFunction,
    Status(i32),
}
pub fn query() -> Result<Version, Error> {
    unsafe {
        // ntdll 是进程加载的系统模块；这里只借用句柄，不加载外部 DLL，不调用 FreeLibrary。
        let module = GetModuleHandleW(w!("ntdll.dll")).map_err(Error::Api)?;
        let address = GetProcAddress(module, s!("RtlGetVersion")).ok_or(Error::MissingFunction)?;
        type RtlGetVersion = unsafe extern "system" fn(*mut OSVERSIONINFOW) -> NTSTATUS;
        let get_version: RtlGetVersion = std::mem::transmute(address);
        let mut info = OSVERSIONINFOW {
            dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOW>() as u32,
            ..Default::default()
        };
        let status = get_version(&mut info);
        if status.0 < 0 {
            return Err(Error::Status(status.0));
        }
        Ok(Version {
            major: info.dwMajorVersion,
            minor: info.dwMinorVersion,
            build: info.dwBuildNumber,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn version_boundary_is_numeric_and_live_query_is_read_only() {
        for (major, minor, build, expected) in [
            (6, 3, 22000, false),
            (10, 0, 21999, false),
            (10, 0, 22000, true),
            (10, 0, 26100, true),
            (10, 1, 1, true),
            (11, 0, 1, true),
        ] {
            assert_eq!(
                Version {
                    major,
                    minor,
                    build
                }
                .windows_11_or_newer(),
                expected
            );
        }
        let actual = query().expect("Windows must expose RtlGetVersion");
        assert!(actual.major >= 6 && actual.build > 0);
    }
}

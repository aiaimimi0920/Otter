//! 恢复子进程父端认证：直接父进程、同 executable、双向 pipe 创建者必须一致。
use std::{
    ffi::OsString,
    os::windows::{
        ffi::OsStringExt,
        io::{AsHandle, AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle},
    },
    path::PathBuf,
};
use windows::{
    core::{Error, Result, PWSTR},
    Win32::{
        Foundation::*,
        System::{Diagnostics::ToolHelp::*, Pipes::GetNamedPipeServerProcessId, Threading::*},
    },
};

pub struct Parent {
    process: OwnedHandle,
}
impl Parent {
    /// input/output 必须是实际协议管道；nonce/命令行 PID 本身不构成认证。
    pub fn authenticate(
        expected_pid: u32,
        input: BorrowedHandle<'_>,
        output: BorrowedHandle<'_>,
    ) -> Result<Self> {
        Self::authenticate_with_access(expected_pid, input, output, false)
    }
    /// 只有独立紧急退出通道申请终止权；普通恢复调用仍只读/等待原父进程。
    pub(crate) fn authenticate_emergency(
        expected_pid: u32,
        pipe: BorrowedHandle<'_>,
    ) -> Result<Self> {
        Self::authenticate_with_access(expected_pid, pipe, pipe, true)
    }
    fn authenticate_with_access(
        expected_pid: u32,
        input: BorrowedHandle<'_>,
        output: BorrowedHandle<'_>,
        emergency: bool,
    ) -> Result<Self> {
        if expected_pid == 0 || expected_pid == std::process::id() {
            return Err(Error::new(E_ACCESSDENIED, "Invalid recovery parent"));
        }
        unsafe {
            let raw = OpenProcess(
                PROCESS_SYNCHRONIZE
                    | PROCESS_QUERY_LIMITED_INFORMATION
                    | if emergency {
                        PROCESS_TERMINATE
                    } else {
                        PROCESS_ACCESS_RIGHTS(0)
                    },
                false,
                expected_pid,
            )?;
            let owner = Self {
                process: OwnedHandle::from_raw_handle(raw.0),
            };
            if direct_parent()? != expected_pid
                || owner.exited()?
                || created(HANDLE(owner.process.as_raw_handle()))? > created(GetCurrentProcess())?
            {
                return Err(Error::new(
                    E_ACCESSDENIED,
                    "Recovery parent identity mismatch",
                ));
            }
            let current = std::fs::canonicalize(std::env::current_exe().map_err(io_error)?)
                .map_err(io_error)?;
            let parent = std::fs::canonicalize(image(HANDLE(owner.process.as_raw_handle()))?)
                .map_err(io_error)?;
            if current != parent {
                return Err(Error::new(
                    E_ACCESSDENIED,
                    "Recovery parent executable mismatch",
                ));
            }
            for pipe in [input, output] {
                let mut creator = 0;
                GetNamedPipeServerProcessId(HANDLE(pipe.as_raw_handle()), &mut creator)?;
                if creator != expected_pid || owner.exited()? {
                    return Err(Error::new(
                        E_ACCESSDENIED,
                        "Recovery pipe is not parent-owned",
                    ));
                }
            }
            Ok(owner)
        }
    }
    pub fn handle(&self) -> BorrowedHandle<'_> {
        self.process.as_handle()
    }
    /// 仅对已经认证且保活的原句柄执行，不重新按PID打开或终止整个Job。
    pub(crate) fn terminate_emergency(&self) -> Result<()> {
        unsafe { TerminateProcess(HANDLE(self.process.as_raw_handle()), 0) }
    }
    /// 查询原 handle，不重新按 PID 打开；管道关闭不改变此结果。
    pub fn exited(&self) -> Result<bool> {
        match unsafe { WaitForSingleObject(HANDLE(self.process.as_raw_handle()), 0) } {
            WAIT_OBJECT_0 => Ok(true),
            WAIT_TIMEOUT => Ok(false),
            WAIT_FAILED => Err(Error::from_win32()),
            _ => Err(Error::new(E_FAIL, "Unexpected recovery parent wait status")),
        }
    }
}
fn io_error(error: std::io::Error) -> Error {
    Error::new(E_FAIL, error.to_string())
}
unsafe fn created(process: HANDLE) -> Result<u64> {
    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    GetProcessTimes(process, &mut creation, &mut exit, &mut kernel, &mut user)?;
    // 旧父 PID 若已复用，新进程不能晚于本子进程出生；不信任 ToolHelp 中的 PID 数值本身。
    Ok((u64::from(creation.dwHighDateTime) << 32) | u64::from(creation.dwLowDateTime))
}
unsafe fn image(process: HANDLE) -> Result<PathBuf> {
    let mut buffer = vec![0u16; 32768];
    let mut length = buffer.len() as u32;
    QueryFullProcessImageNameW(
        process,
        PROCESS_NAME_WIN32,
        PWSTR(buffer.as_mut_ptr()),
        &mut length,
    )?;
    if length == 0 || length as usize >= buffer.len() {
        return Err(Error::new(E_FAIL, "Invalid recovery parent image length"));
    }
    Ok(PathBuf::from(OsString::from_wide(
        &buffer[..length as usize],
    )))
}
fn direct_parent() -> Result<u32> {
    unsafe {
        let raw = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)?;
        let snapshot = OwnedHandle::from_raw_handle(raw.0);
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        Process32FirstW(HANDLE(snapshot.as_raw_handle()), &mut entry)?;
        // 不保留进程清单、不记录其他进程；有界搜索只提取自己的直接父 PID。
        for _ in 0..65536 {
            if entry.th32ProcessID == std::process::id() {
                return Ok(entry.th32ParentProcessID);
            }
            Process32NextW(HANDLE(snapshot.as_raw_handle()), &mut entry)?;
        }
        Err(Error::new(E_FAIL, "Recovery parent lookup exceeded limit"))
    }
}

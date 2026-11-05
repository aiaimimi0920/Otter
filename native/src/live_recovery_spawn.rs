//! 恢复/紧急退出私有child的CreateProcessW：只继承协议端点/NUL，不是通用launcher。
use std::{
    io,
    os::windows::{
        io::{AsHandle, AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle},
        process::ExitStatusExt,
    },
    process::{Command, ExitStatus},
};
use windows::{
    core::{w, PCWSTR, PWSTR},
    Win32::{
        Foundation::*, Security::SECURITY_ATTRIBUTES, Storage::FileSystem::*, System::Threading::*,
    },
};
#[path = "live_recovery_spawn_text.rs"]
mod text;

pub(crate) struct Process(OwnedHandle);
impl AsHandle for Process {
    fn as_handle(&self) -> BorrowedHandle<'_> {
        self.0.as_handle()
    }
}
impl AsRawHandle for Process {
    fn as_raw_handle(&self) -> std::os::windows::io::RawHandle {
        self.0.as_raw_handle()
    }
}
impl Process {
    pub fn kill(&mut self) -> io::Result<()> {
        unsafe { TerminateProcess(HANDLE(self.as_raw_handle()), 1) }.map_err(io::Error::other)
    }
    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        unsafe {
            if WaitForSingleObject(HANDLE(self.as_raw_handle()), INFINITE) != WAIT_OBJECT_0 {
                return Err(io::Error::last_os_error());
            }
            let mut code = 0;
            GetExitCodeProcess(HANDLE(self.as_raw_handle()), &mut code)
                .map_err(io::Error::other)?;
            Ok(ExitStatus::from_raw(code))
        }
    }
}
struct Attributes {
    pointer: LPPROC_THREAD_ATTRIBUTE_LIST,
    _buffer: Vec<usize>,
}
impl Drop for Attributes {
    fn drop(&mut self) {
        unsafe {
            DeleteProcThreadAttributeList(self.pointer);
        }
    }
}
impl Attributes {
    fn new() -> io::Result<Self> {
        unsafe {
            let mut size = 0;
            let _ = InitializeProcThreadAttributeList(None, 1, None, &mut size);
            if size == 0 || size > 65536 {
                return Err(io::Error::other("Invalid process attribute size"));
            }
            let mut buffer = vec![0usize; size.div_ceil(std::mem::size_of::<usize>())];
            let pointer = LPPROC_THREAD_ATTRIBUTE_LIST(buffer.as_mut_ptr().cast());
            InitializeProcThreadAttributeList(Some(pointer), 1, None, &mut size)
                .map_err(io::Error::other)?;
            Ok(Self {
                pointer,
                _buffer: buffer,
            })
        }
    }
}
pub(crate) fn spawn(command: &Command, endpoint: OwnedHandle) -> io::Result<Process> {
    let executable = text::executable(command)?;
    let mut arguments = text::arguments(command)?;
    let environment = text::environment(command)?;
    unsafe {
        let mut duplicate = HANDLE::default();
        DuplicateHandle(
            GetCurrentProcess(),
            HANDLE(endpoint.as_raw_handle()),
            GetCurrentProcess(),
            &mut duplicate,
            0,
            true,
            DUPLICATE_SAME_ACCESS,
        )
        .map_err(io::Error::other)?;
        let input = OwnedHandle::from_raw_handle(duplicate.0);
        let security = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            bInheritHandle: true.into(),
            ..Default::default()
        };
        let null = CreateFileW(
            w!("NUL"),
            (GENERIC_READ | GENERIC_WRITE).0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            Some(&security),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
        .map_err(io::Error::other)?;
        let null = OwnedHandle::from_raw_handle(null.0);
        // 值数组必须活到属性列表销毁；显式声明在 Attributes 之前。
        let handles = [HANDLE(input.as_raw_handle()), HANDLE(null.as_raw_handle())];
        let attributes = Attributes::new()?;
        UpdateProcThreadAttribute(
            attributes.pointer,
            0,
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
            Some(handles.as_ptr().cast()),
            std::mem::size_of_val(&handles),
            None,
            None,
        )
        .map_err(io::Error::other)?;
        let mut startup = STARTUPINFOEXW {
            lpAttributeList: attributes.pointer,
            ..Default::default()
        };
        startup.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
        startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
        startup.StartupInfo.hStdInput = handles[0];
        startup.StartupInfo.hStdOutput = handles[1];
        startup.StartupInfo.hStdError = handles[1];
        let mut process = PROCESS_INFORMATION::default();
        CreateProcessW(
            PCWSTR(executable.as_ptr()),
            Some(PWSTR(arguments.as_mut_ptr())),
            None,
            None,
            true,
            CREATE_NO_WINDOW | EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT,
            environment.as_ref().map(|value| value.as_ptr().cast()),
            None,
            &startup.StartupInfo,
            &mut process,
        )
        .map_err(io::Error::other)?;
        let _thread = OwnedHandle::from_raw_handle(process.hThread.0);
        Ok(Process(OwnedHandle::from_raw_handle(process.hProcess.0)))
    }
}

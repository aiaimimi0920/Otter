//! Recovery-child parent authentication: the direct parent, matching executable, and creators of both pipe directions must agree.
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
    /// input/output must be the actual protocol pipes; a nonce/command-line PID alone is not authentication.
    pub fn authenticate(
        expected_pid: u32,
        input: BorrowedHandle<'_>,
        output: BorrowedHandle<'_>,
    ) -> Result<Self> {
        Self::authenticate_with_access(expected_pid, input, output, false)
    }
    /// Only the separate emergency-exit channel requests termination rights; ordinary recovery calls still only read/wait on the original parent.
    pub fn authenticate_emergency(expected_pid: u32, pipe: BorrowedHandle<'_>) -> Result<Self> {
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
    /// Operate only on the authenticated original handle kept alive; do not reopen by PID or terminate the entire Job.
    pub fn terminate_emergency(&self) -> Result<()> {
        unsafe { TerminateProcess(HANDLE(self.process.as_raw_handle()), 0) }
    }
    /// Query the original handle without reopening by PID; closing the pipe does not change this result.
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
    // If the old parent PID has been reused, the new process cannot have been born after this child; do not trust the ToolHelp PID value alone.
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
        // Retain no process list and record no other processes; the bounded search extracts only this process's direct parent PID.
        for _ in 0..65536 {
            if entry.th32ProcessID == std::process::id() {
                return Ok(entry.th32ParentProcessID);
            }
            Process32NextW(HANDLE(snapshot.as_raw_handle()), &mut entry)?;
        }
        Err(Error::new(E_FAIL, "Recovery parent lookup exceeded limit"))
    }
}

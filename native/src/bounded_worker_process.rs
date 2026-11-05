//! 受限计算 child 的唯一进程/Job owner；领域层只提供固定入口、内存策略和容量池。
use std::{
    io,
    os::windows::{
        ffi::OsStrExt,
        io::{AsHandle, AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle},
    },
    path::Path,
    sync::Arc,
    time::Instant,
};
use windows::{
    core::{w, PCWSTR, PWSTR},
    Win32::{
        Foundation::*,
        Security::SECURITY_ATTRIBUTES,
        Storage::FileSystem::*,
        System::{JobObjects::*, Threading::*},
    },
};
#[path = "bounded_worker_environment.rs"]
mod environment;
pub(crate) use environment::Environment;
pub(crate) struct Policy<'a> {
    pub suffix: &'a str,
    pub maximum: usize,
    pub pool: &'a Arc<crate::byte_budget::Budget>,
    pub environment: Environment,
}
pub(crate) struct Child {
    process: OwnedHandle,
    job: OwnedHandle,
    _capacity: crate::byte_budget::Lease,
}
impl Child {
    #[cfg(test)]
    pub(crate) fn peak_job_bytes(&self) -> io::Result<usize> {
        peak_job_bytes(&self.job)
    }
    #[cfg(test)]
    pub(crate) fn probe(&self) -> io::Result<Probe> {
        Ok(Probe {
            process: self.process.try_clone()?,
            job: self.job.try_clone()?,
        })
    }
    pub(crate) fn handle(&self) -> BorrowedHandle<'_> {
        self.process.as_handle()
    }
    pub(crate) fn finish(&self, deadline: Instant, stopped: &impl Fn() -> bool) -> io::Result<()> {
        loop {
            if stopped() || Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Worker child stopped or timed out",
                ));
            }
            let result = unsafe { WaitForSingleObject(HANDLE(self.process.as_raw_handle()), 20) };
            if result == WAIT_OBJECT_0 {
                let mut code = 0;
                unsafe { GetExitCodeProcess(HANDLE(self.process.as_raw_handle()), &mut code) }
                    .map_err(io::Error::other)?;
                return if code == 0 {
                    Ok(())
                } else {
                    Err(io::Error::other("Worker child failed"))
                };
            }
            if result != WAIT_TIMEOUT {
                return Err(io::Error::last_os_error());
            }
        }
    }
}
// 只读观测句柄不拥有终止策略或容量租约；测试在 API 返回后核验真实回收与峰值。
#[cfg(test)]
pub(crate) struct Probe {
    process: OwnedHandle,
    job: OwnedHandle,
}
#[cfg(test)]
impl Probe {
    pub(crate) fn exited(&self) -> bool {
        unsafe { WaitForSingleObject(HANDLE(self.process.as_raw_handle()), 0) == WAIT_OBJECT_0 }
    }
    pub(crate) fn peak_job_bytes(&self) -> io::Result<usize> {
        peak_job_bytes(&self.job)
    }
}
#[cfg(test)]
fn peak_job_bytes(job: &OwnedHandle) -> io::Result<usize> {
    let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    unsafe {
        QueryInformationJobObject(
            Some(HANDLE(job.as_raw_handle())),
            JobObjectExtendedLimitInformation,
            &mut info as *mut _ as *mut _,
            std::mem::size_of_val(&info) as u32,
            None,
        )
    }
    .map_err(io::Error::other)?;
    Ok(info.PeakJobMemoryUsed)
}
impl Drop for Child {
    fn drop(&mut self) {
        unsafe {
            // Job 无 breakaway 且只准一个进程；回收不承诺包含驱动/内核清理的硬实时上界。
            let _ = TerminateJobObject(HANDLE(self.job.as_raw_handle()), 1);
            let _ = WaitForSingleObject(HANDLE(self.process.as_raw_handle()), INFINITE);
        }
    }
}
struct Attributes {
    pointer: LPPROC_THREAD_ATTRIBUTE_LIST,
    _storage: Vec<usize>,
}
impl Drop for Attributes {
    fn drop(&mut self) {
        unsafe { DeleteProcThreadAttributeList(self.pointer) }
    }
}
pub(crate) fn capacity(
    pool: &Arc<crate::byte_budget::Budget>,
    maximum: usize,
    deadline: Instant,
    stopped: &impl Fn() -> bool,
) -> io::Result<crate::byte_budget::Lease> {
    loop {
        if stopped() || Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Worker child capacity wait stopped",
            ));
        }
        if let Some(lease) = pool.reserve(maximum) {
            return Ok(lease);
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}
/// 此入口只接受 crate 内固定 suffix/策略；不得把远端数据拼进命令行。
pub(crate) fn start(
    endpoint: OwnedHandle,
    deadline: Instant,
    stopped: &impl Fn() -> bool,
    policy: Policy<'_>,
) -> io::Result<Child> {
    start_at(
        endpoint,
        deadline,
        stopped,
        policy,
        &std::env::current_exe()?,
    )
}
// 生产只经 start 选择 current_exe；测试才可指定同 target/profile 的产品 EXE。
pub(crate) fn start_at(
    endpoint: OwnedHandle,
    deadline: Instant,
    stopped: &impl Fn() -> bool,
    policy: Policy<'_>,
    executable: &Path,
) -> io::Result<Child> {
    let capacity = capacity(policy.pool, policy.maximum, deadline, stopped)?;
    let environment = policy.environment.block()?;
    let executable: Vec<u16> = executable
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let mut args = vec![34u16];
    args.extend_from_slice(&executable[..executable.len() - 1]);
    args.push(34);
    args.extend(policy.suffix.encode_utf16());
    args.push(0);
    unsafe {
        let job = CreateJobObjectW(None, None).map_err(io::Error::other)?;
        let job = OwnedHandle::from_raw_handle(job.0);
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_PROCESS_MEMORY
            | JOB_OBJECT_LIMIT_JOB_MEMORY
            | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
            | JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        limits.BasicLimitInformation.ActiveProcessLimit = 1;
        limits.ProcessMemoryLimit = policy.maximum;
        limits.JobMemoryLimit = policy.maximum;
        SetInformationJobObject(
            HANDLE(job.as_raw_handle()),
            JobObjectExtendedLimitInformation,
            &limits as *const _ as *const _,
            std::mem::size_of_val(&limits) as u32,
        )
        .map_err(io::Error::other)?;
        let mut input = HANDLE::default();
        DuplicateHandle(
            GetCurrentProcess(),
            HANDLE(endpoint.as_raw_handle()),
            GetCurrentProcess(),
            &mut input,
            0,
            true,
            DUPLICATE_SAME_ACCESS,
        )
        .map_err(io::Error::other)?;
        let input = OwnedHandle::from_raw_handle(input.0);
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
        let handles = [HANDLE(input.as_raw_handle()), HANDLE(null.as_raw_handle())];
        let mut size = 0;
        let _ = InitializeProcThreadAttributeList(None, 1, None, &mut size);
        if size == 0 || size > 65536 {
            return Err(io::Error::other("Invalid worker process attributes"));
        }
        let mut storage = vec![0usize; size.div_ceil(std::mem::size_of::<usize>())];
        let pointer = LPPROC_THREAD_ATTRIBUTE_LIST(storage.as_mut_ptr().cast());
        InitializeProcThreadAttributeList(Some(pointer), 1, None, &mut size)
            .map_err(io::Error::other)?;
        let attributes = Attributes {
            pointer,
            _storage: storage,
        };
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
            Some(PWSTR(args.as_mut_ptr())),
            None,
            None,
            true,
            CREATE_NO_WINDOW
                | CREATE_SUSPENDED
                | EXTENDED_STARTUPINFO_PRESENT
                | CREATE_UNICODE_ENVIRONMENT,
            Some(environment.as_ptr().cast()),
            None,
            &startup.StartupInfo,
            &mut process,
        )
        .map_err(io::Error::other)?;
        let thread = OwnedHandle::from_raw_handle(process.hThread.0);
        let process = OwnedHandle::from_raw_handle(process.hProcess.0);
        if let Err(error) =
            AssignProcessToJobObject(HANDLE(job.as_raw_handle()), HANDLE(process.as_raw_handle()))
        {
            let _ = TerminateProcess(HANDLE(process.as_raw_handle()), 1);
            let _ = WaitForSingleObject(HANDLE(process.as_raw_handle()), INFINITE);
            return Err(io::Error::other(error));
        }
        let child = Child {
            process,
            job,
            _capacity: capacity,
        };
        if ResumeThread(HANDLE(thread.as_raw_handle())) == u32::MAX {
            return Err(io::Error::last_os_error());
        }
        Ok(child)
    }
}
/// child 必须在读取/执行领域载荷前验证父侧 Job，拒绝直接私有入口绕过限额。
pub(crate) fn require_limits(maximum: usize) -> io::Result<()> {
    unsafe {
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        QueryInformationJobObject(
            None,
            JobObjectExtendedLimitInformation,
            &mut limits as *mut _ as *mut _,
            std::mem::size_of_val(&limits) as u32,
            None,
        )
        .map_err(io::Error::other)?;
        let required = JOB_OBJECT_LIMIT_PROCESS_MEMORY
            | JOB_OBJECT_LIMIT_JOB_MEMORY
            | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
            | JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if (limits.BasicLimitInformation.LimitFlags & required) != required
            || limits.BasicLimitInformation.ActiveProcessLimit != 1
            || limits.ProcessMemoryLimit == 0
            || limits.ProcessMemoryLimit > maximum
            || limits.JobMemoryLimit == 0
            || limits.JobMemoryLimit > maximum
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Missing worker Job limits",
            ));
        }
    }
    Ok(())
}

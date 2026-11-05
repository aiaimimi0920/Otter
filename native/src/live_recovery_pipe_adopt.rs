//! 接收继承端点前核对内核 I/O 模式，不把任意 stdin 当作 overlapped 管道。
use super::*;
use windows::{
    Wdk::Storage::FileSystem::{
        FileModeInformation, NtQueryInformationFile, FILE_MODE_INFORMATION,
        FILE_SYNCHRONOUS_IO_ALERT, FILE_SYNCHRONOUS_IO_NONALERT,
    },
    Win32::{
        Storage::FileSystem::{GetFileType, FILE_TYPE_PIPE},
        System::Pipes::{GetNamedPipeInfo, PIPE_TYPE_MESSAGE},
    },
};

impl Pipe {
    pub fn adopt(handle: OwnedHandle) -> io::Result<Self> {
        let raw = HANDLE(handle.as_raw_handle());
        unsafe {
            if GetFileType(raw) != FILE_TYPE_PIPE {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Recovery endpoint is not a pipe",
                ));
            }
            let mut flags = windows::Win32::System::Pipes::NAMED_PIPE_MODE::default();
            GetNamedPipeInfo(raw, Some(&mut flags), None, None, None).map_err(io::Error::other)?;
            if flags.0 & PIPE_TYPE_MESSAGE.0 != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Recovery endpoint must be a byte pipe",
                ));
            }
            let mut mode = FILE_MODE_INFORMATION::default();
            let mut status = IO_STATUS_BLOCK::default();
            let result = NtQueryInformationFile(
                raw,
                &mut status,
                (&mut mode as *mut FILE_MODE_INFORMATION).cast(),
                std::mem::size_of_val(&mode) as u32,
                FileModeInformation,
            );
            // 即使查询返回 pending，也必须等到内核不再持有栈上输出缓冲区。
            let result = if result == STATUS_PENDING {
                if WaitForSingleObject(raw, INFINITE) != WAIT_OBJECT_0 {
                    // OwnedHandle 始终存活且没有外部关闭，正常情况下此等待不会失败。
                    // 无法确认完成时不可释放内核借用的栈内存。
                    std::process::abort();
                }
                status.Anonymous.Status
            } else {
                result
            };
            result.ok().map_err(io::Error::other)?;
            if mode.Mode & (FILE_SYNCHRONOUS_IO_ALERT.0 | FILE_SYNCHRONOUS_IO_NONALERT.0) != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Recovery endpoint must support overlapped I/O",
                ));
            }
            Ok(Self { handle })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::System::Pipes::CreatePipe;
    #[test]
    fn accepts_overlapped_pair_but_rejects_synchronous_pipe() {
        let (server, client) = Pipe::pair().unwrap();
        Pipe::adopt(server.into_handle()).unwrap();
        Pipe::adopt(client.into_handle()).unwrap();
        unsafe {
            let mut read = HANDLE::default();
            let mut write = HANDLE::default();
            CreatePipe(&mut read, &mut write, None, 0).unwrap();
            let read = OwnedHandle::from_raw_handle(read.0);
            let _write = OwnedHandle::from_raw_handle(write.0);
            assert!(Pipe::adopt(read).is_err());
        }
    }
}

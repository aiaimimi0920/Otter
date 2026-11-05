//! 随机命名、单实例、本机 duplex 管道；两端均由当前进程创建，客户端可显式交给子进程。
use super::*;
use windows::{
    core::PCWSTR,
    Win32::{
        Storage::FileSystem::*,
        System::{Com::CoCreateGuid, Pipes::*},
    },
};

pub(super) fn create() -> io::Result<(Pipe, Pipe)> {
    unsafe {
        let guid = CoCreateGuid().map_err(io::Error::other)?;
        let name: Vec<u16> = format!(
            r"\\.\pipe\Otter.Native.Recovery.{}.{guid:?}",
            std::process::id()
        )
        .encode_utf16()
        .chain(Some(0))
        .collect();
        let server = CreateNamedPipeW(
            PCWSTR(name.as_ptr()),
            PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED | FILE_FLAG_FIRST_PIPE_INSTANCE,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            1,
            4096,
            4096,
            0,
            None,
        );
        if server.is_invalid() {
            return Err(io::Error::last_os_error());
        }
        let server = OwnedHandle::from_raw_handle(server.0);
        let client = CreateFileW(
            PCWSTR(name.as_ptr()),
            (GENERIC_READ | GENERIC_WRITE).0,
            FILE_SHARE_MODE::default(),
            None,
            OPEN_EXISTING,
            FILE_FLAG_OVERLAPPED,
            None,
        )
        .map_err(io::Error::other)?;
        let client = OwnedHandle::from_raw_handle(client.0);
        let event = CreateEventW(None, true, false, None).map_err(io::Error::other)?;
        let event = OwnedHandle::from_raw_handle(event.0);
        let mut pending = Pending {
            handle: HANDLE(server.as_raw_handle()),
            overlapped: Box::new(OVERLAPPED {
                hEvent: HANDLE(event.as_raw_handle()),
                ..Default::default()
            }),
            active: false,
        };
        match ConnectNamedPipe(pending.handle, Some(&mut *pending.overlapped)) {
            Ok(()) => {}
            Err(error) if error.code() == HRESULT::from_win32(ERROR_PIPE_CONNECTED.0) => {}
            Err(error) => {
                pending.active = error.code() == HRESULT::from_win32(ERROR_IO_PENDING.0);
                return Err(io::Error::other(error));
            }
        }
        Ok((Pipe::from_overlapped(server), Pipe::from_overlapped(client)))
    }
}

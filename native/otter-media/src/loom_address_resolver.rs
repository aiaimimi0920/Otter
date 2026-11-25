//! Windows asynchronous name resolution; after cancellation, wait for completion before releasing OVERLAPPED/results, leaving no DNS wait threads.
use crate::device_request_cancel::Cancel;
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV6},
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    sync::Arc,
};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::*,
        Networking::WinSock::*,
        System::{Threading::*, IO::OVERLAPPED},
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Input,
    Stopped,
    Startup,
    Event,
    Lookup,
    Timeout,
    Wait,
    Empty,
    Limit,
}

struct Winsock;
impl Drop for Winsock {
    fn drop(&mut self) {
        unsafe {
            WSACleanup();
        }
    }
}
struct Operation {
    host: Vec<u16>,
    service: Vec<u16>,
    hints: ADDRINFOEXW,
    timeout: TIMEVAL,
    result: *mut ADDRINFOEXW,
    overlap: OVERLAPPED,
    cancel: HANDLE,
    event: OwnedHandle,
    pending: bool,
}
struct Query(Option<Box<Operation>>);
impl Drop for Query {
    fn drop(&mut self) {
        let Some(mut operation) = self.0.take() else {
            return;
        };
        unsafe {
            if operation.pending {
                GetAddrInfoExCancel(std::ptr::addr_of_mut!(operation.cancel));
                // The OS still owns the structure pointer; requesting cancellation is not completion, so never release it directly.
                if WaitForSingleObject(HANDLE(operation.event.as_raw_handle()), INFINITE)
                    != WAIT_OBJECT_0
                {
                    // An unexpected wait failure on a valid handle: retain operation memory rather than free memory the OS may still write.
                    std::mem::forget(operation);
                    return;
                }
            }
            if !operation.result.is_null() {
                FreeAddrInfoExW(Some(operation.result));
            }
        }
    }
}
struct Wake<'a>(&'a Cancel);
impl Drop for Wake<'_> {
    fn drop(&mut self) {
        self.0.clear_wake();
    }
}
fn event() -> Result<OwnedHandle, Error> {
    let handle = unsafe { CreateEventW(None, true, false, None) }.map_err(|_| Error::Event)?;
    Ok(unsafe { OwnedHandle::from_raw_handle(handle.0) })
}

/// Background calls only. host comes from a verified origin, not a URL/port; return at most four deduplicated addresses.
pub fn resolve(host: &str, port: u16, cancel: &Cancel) -> Result<Vec<SocketAddr>, Error> {
    if cancel.is_stopped() {
        return Err(Error::Stopped);
    }
    if port == 0
        || host.is_empty()
        || host.len() > 253
        || host.bytes().any(|b| !b.is_ascii_graphic())
    {
        return Err(Error::Input);
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok(vec![SocketAddr::new(ip, port)]);
    }
    if host
        .bytes()
        .any(|b| !(b.is_ascii_alphanumeric() || b == b'.' || b == b'-'))
    {
        return Err(Error::Input);
    }
    // Coexist with other Winsock users through system reference counting; do not assume std::net has initialized it.
    if unsafe { WSAStartup(0x0202, &mut WSADATA::default()) } != 0 {
        return Err(Error::Startup);
    }
    let _winsock = Winsock;
    let completed = event()?;
    let stopped = Arc::new(event()?);
    cancel.install_wake(stopped.clone());
    let _wake = Wake(cancel);
    if cancel.is_stopped() {
        return Err(Error::Stopped);
    }
    let mut query = Query(Some(Box::new(Operation {
        host: host.encode_utf16().chain(Some(0)).collect(),
        service: port.to_string().encode_utf16().chain(Some(0)).collect(),
        hints: ADDRINFOEXW {
            ai_socktype: SOCK_STREAM.0,
            ai_protocol: IPPROTO_TCP.0,
            ..Default::default()
        },
        timeout: TIMEVAL {
            tv_sec: 5,
            tv_usec: 0,
        },
        result: std::ptr::null_mut(),
        overlap: OVERLAPPED {
            hEvent: HANDLE(completed.as_raw_handle()),
            ..Default::default()
        },
        cancel: HANDLE::default(),
        event: completed,
        pending: false,
    })));
    let operation = query.0.as_mut().unwrap();
    let status = unsafe {
        GetAddrInfoExW(
            PCWSTR(operation.host.as_ptr()),
            PCWSTR(operation.service.as_ptr()),
            NS_DNS,
            None,
            Some(&operation.hints),
            &mut operation.result,
            Some(&operation.timeout),
            Some(std::ptr::addr_of_mut!(operation.overlap)),
            None,
            Some(&mut operation.cancel),
        )
    };
    if status == WSA_IO_PENDING.0 {
        operation.pending = true;
        let handles = [
            HANDLE(operation.event.as_raw_handle()),
            HANDLE(stopped.as_raw_handle()),
        ];
        let result = unsafe { WaitForMultipleObjects(&handles, false, 5000) };
        if result != WAIT_OBJECT_0 {
            return Err(if cancel.is_stopped() {
                Error::Stopped
            } else if result == WAIT_TIMEOUT {
                Error::Timeout
            } else {
                Error::Wait
            });
        }
        operation.pending = false;
        if unsafe { GetAddrInfoExOverlappedResult(&operation.overlap) } != 0 {
            return Err(Error::Lookup);
        }
    } else if status != 0 {
        return Err(Error::Lookup);
    }
    if cancel.is_stopped() {
        return Err(Error::Stopped);
    }
    unsafe { collect(operation.result, port) }
}

unsafe fn collect(mut node: *const ADDRINFOEXW, port: u16) -> Result<Vec<SocketAddr>, Error> {
    let mut addresses = Vec::new();
    let mut count = 0;
    while !node.is_null() {
        count += 1;
        if count > 256 {
            return Err(Error::Limit);
        }
        let info = &*node;
        let address = if !info.ai_addr.is_null()
            && info.ai_family == AF_INET.0 as i32
            && info.ai_addrlen >= std::mem::size_of::<SOCKADDR_IN>()
        {
            let addr = std::ptr::read_unaligned(info.ai_addr.cast::<SOCKADDR_IN>());
            Some(SocketAddr::new(
                Ipv4Addr::from(addr.sin_addr.S_un.S_addr.to_ne_bytes()).into(),
                u16::from_be(addr.sin_port),
            ))
        } else if !info.ai_addr.is_null()
            && info.ai_family == AF_INET6.0 as i32
            && info.ai_addrlen >= std::mem::size_of::<SOCKADDR_IN6>()
        {
            let addr = std::ptr::read_unaligned(info.ai_addr.cast::<SOCKADDR_IN6>());
            Some(SocketAddr::V6(SocketAddrV6::new(
                Ipv6Addr::from(addr.sin6_addr.u.Byte),
                u16::from_be(addr.sin6_port),
                u32::from_be(addr.sin6_flowinfo),
                addr.Anonymous.sin6_scope_id,
            )))
        } else {
            None
        };
        if let Some(address) = address.filter(|a| a.port() == port) {
            if !addresses.contains(&address) && addresses.len() < 4 {
                addresses.push(address);
            }
        }
        node = info.ai_next;
    }
    if addresses.is_empty() {
        Err(Error::Empty)
    } else {
        Ok(addresses)
    }
}

#[cfg(test)]
#[path = "loom_address_resolver_tests.rs"]
mod tests;

use super::*;

pub(super) fn fixture(mode: &str) -> Command {
    super::fixture_command(mode)
}

#[test]
fn real_child_owner_handshakes_shuts_down_and_joins() {
    let mut client = Client::start_command(fixture("serve")).unwrap();
    assert!(client.healthy().unwrap());
    assert!(client.join_shutdown().is_err());
    // 本地非法请求不占序号、不毒化会话。
    assert!(client.request(Operation::Retire { id: 0 }).is_err());
    assert!(client.healthy().unwrap());
    assert_eq!(
        client.request(Operation::Retire { id: 1 }).unwrap(),
        Status::Rejected
    );
    assert_eq!(
        client.request(Operation::Shutdown).unwrap(),
        Status::Accepted
    );
    client.join_shutdown().unwrap();
    assert!(!client.healthy().unwrap());
    assert!(client.request(Operation::Shutdown).is_err());
}

#[test]
fn failed_handshake_is_reaped_and_child_exit_prevents_new_requests() {
    assert!(Client::start_command(fixture("exit")).is_err());
    let mut client = Client::start_command(fixture("serve")).unwrap();
    // 只终止本测试创建的空登记子进程，绝不触碰用户应用。
    client.child.kill().unwrap();
    client.child.wait().unwrap();
    assert!(!client.healthy().unwrap());
    assert!(client.request(Operation::Shutdown).is_err());
}

#[test]
#[ignore = "private helper launched only by recovery Client fixtures"]
fn child_entry() {
    let mode = std::env::var("OTTER_RECOVERY_CLIENT_FIXTURE").unwrap();
    if mode == "exit" {
        return;
    }
    assert!(matches!(mode.as_str(), "serve" | "shutdown-hang"));
    if let Ok(raw) = std::env::var("OTTER_RECOVERY_INHERIT_EVENT") {
        let raw = raw.parse::<usize>().unwrap();
        // 只能影响父测试专门创建的事件，不枚举其他句柄。
        let _ = unsafe { SetEvent(HANDLE(raw as *mut _)) };
    }
    let parent = std::env::var("OTTER_RECOVERY_CLIENT_PARENT")
        .unwrap()
        .parse()
        .unwrap();
    crate::live_recovery_runtime::run_inherited(parent).unwrap();
    if mode == "shutdown-hang" {
        // 模拟已发空登记 Shutdown ACK、但进程析构仍未结束；父测试负责回收。
        std::thread::sleep(Duration::from_secs(60));
    }
}

#[test]
fn recovery_child_must_not_inherit_unrelated_event() {
    use std::os::windows::io::{FromRawHandle, OwnedHandle};
    use windows::Win32::Security::SECURITY_ATTRIBUTES;
    let security = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        bInheritHandle: true.into(),
        ..Default::default()
    };
    let event = unsafe {
        OwnedHandle::from_raw_handle(CreateEventW(Some(&security), true, false, None).unwrap().0)
    };
    let mut command = fixture("serve");
    command.env(
        "OTTER_RECOVERY_INHERIT_EVENT",
        (event.as_raw_handle() as usize).to_string(),
    );
    let mut client = Client::start_command(command).unwrap();
    let inherited =
        unsafe { WaitForSingleObject(HANDLE(event.as_raw_handle()), 0) } == WAIT_OBJECT_0;
    assert_eq!(
        client.request(Operation::Shutdown).unwrap(),
        Status::Accepted
    );
    client.join_shutdown().unwrap();
    assert!(
        !inherited,
        "Recovery child inherited an unrelated parent event"
    );
}

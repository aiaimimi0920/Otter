use super::*;
fn wait(owner: &mut Owner) -> io::Result<Reply> {
    let handle = owner
        .completion_handle()
        .expect("Pending operation must expose completion handle");
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), 10000) },
        WAIT_OBJECT_0
    );
    owner.poll().expect("Signaled worker must deliver result")
}
#[test]
fn real_child_async_owner_has_no_idle_worker_and_no_request_queue() {
    let mut owner =
        Owner::start_with(|| Client::start_command(super::super::tests::fixture("serve"))).unwrap();
    assert!(owner.busy());
    assert_eq!(
        owner.request(Operation::Shutdown).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    assert_eq!(wait(&mut owner).unwrap(), Reply::Started);
    assert!(!owner.busy());
    assert!(owner.completion_handle().is_none());
    assert!(owner.child_handle().is_some());
    assert!(owner.healthy().unwrap());
    assert!(owner.poll().is_none());
    owner.request(Operation::Retire { id: 1 }).unwrap();
    assert!(owner.busy());
    assert!(owner.child_handle().is_none());
    assert_eq!(
        owner.request(Operation::Shutdown).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    assert_eq!(
        wait(&mut owner).unwrap(),
        Reply::Completed(Status::Rejected)
    );
    assert!(owner.healthy().unwrap());
    owner.request(Operation::Shutdown).unwrap();
    assert_eq!(
        wait(&mut owner).unwrap(),
        Reply::Completed(Status::Accepted)
    );
    assert!(!owner.healthy().unwrap());
    let handle = owner.child_handle().unwrap();
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), 10000) },
        WAIT_OBJECT_0
    );
}
#[test]
fn startup_failure_delivers_once_and_does_not_accept_commands() {
    let mut owner = Owner::start_with(|| Err(io::Error::other("fixture start failure"))).unwrap();
    assert!(wait(&mut owner).is_err());
    assert!(!owner.busy());
    assert!(owner.poll().is_none());
    assert!(owner.completion_handle().is_none());
    assert!(owner.child_handle().is_none());
    assert!(owner.request(Operation::Shutdown).is_err());
}
#[test]
fn local_request_error_returns_client_without_discarding_owner() {
    let mut owner =
        Owner::start_with(|| Client::start_command(super::super::tests::fixture("serve"))).unwrap();
    wait(&mut owner).unwrap();
    owner.request(Operation::Retire { id: 0 }).unwrap();
    assert!(wait(&mut owner).is_err());
    assert!(owner.healthy().unwrap());
    owner.request(Operation::Shutdown).unwrap();
    assert_eq!(
        wait(&mut owner).unwrap(),
        Reply::Completed(Status::Accepted)
    );
    let handle = owner.child_handle().unwrap();
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), 10000) },
        WAIT_OBJECT_0
    );
}

#[test]
fn request_panic_preserves_original_child_but_poisoned_channel_cannot_be_reused() {
    let mut owner =
        Owner::start_with(|| Client::start_command(super::super::tests::fixture("serve"))).unwrap();
    wait(&mut owner).unwrap();
    let original = owner.child_handle().unwrap().as_raw_handle();
    owner
        .request_with(|client| {
            assert_eq!(
                client.request(Operation::Retire { id: 1 })?,
                Status::Rejected
            );
            panic!("fixture panic after actual acknowledgement");
        })
        .unwrap();
    assert!(wait(&mut owner)
        .unwrap_err()
        .to_string()
        .contains("panicked"));
    assert!(!owner.busy());
    assert_eq!(owner.child_handle().unwrap().as_raw_handle(), original);
    assert!(!owner.healthy().unwrap());
    owner.request(Operation::Shutdown).unwrap();
    assert!(wait(&mut owner).is_err());
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(original), 0) },
        WAIT_TIMEOUT
    );
    // 此夹具没有登记记录，可显式收尾；生产由 Journal 检查所有恢复意图。
    owner.stop_after_restore().unwrap();
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(original), 10000) },
        WAIT_OBJECT_0
    );
}

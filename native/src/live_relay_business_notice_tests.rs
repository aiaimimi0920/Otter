use super::*;
use crate::live_relay_viewer_frames::Cursor;
fn business() -> Arc<crate::live_relay_viewer_business::Business> {
    Arc::new(
        crate::live_relay_viewer_business::Business::from_snapshot(
            br#"{"session":{"observationCapabilities":[]},"observations":[]}"#,
        )
        .unwrap(),
    )
}
#[test]
fn latest_business_replaces_old_and_close_releases_cache() {
    let (owner, snapshot) = prepared();
    owner.activate(&snapshot).unwrap();
    let permit = owner.admit(1).unwrap();
    let first = business();
    let weak = Arc::downgrade(&first);
    owner.publish_business(&permit, first).unwrap();
    let latest = business();
    owner.publish_business(&permit, latest.clone()).unwrap();
    assert!(weak.upgrade().is_none());
    owner
        .commit(&permit, frame(1, vec![255; 4].into()), false)
        .unwrap();
    let update = owner
        .poll_update(Cursor {
            epoch: 1,
            frame_id: 0,
        })
        .unwrap();
    assert!(Arc::ptr_eq(update.business.as_ref().unwrap(), &latest));
    assert_eq!(update.frame.unwrap().metadata().frame_id, 1);
    let weak = Arc::downgrade(&latest);
    drop(update.business);
    drop(latest);
    owner.close();
    assert!(weak.upgrade().is_none());
    assert!(matches!(
        owner.poll_update(Cursor {
            epoch: 1,
            frame_id: 1
        }),
        Err(Error::Closed)
    ));
    assert_eq!(
        owner.publish_business(&permit, business()),
        Err(Error::Closed)
    );
}
#[test]
fn suspended_generation_cannot_publish_business_into_reactivated_session() {
    let (owner, snapshot) = prepared();
    owner.activate(&snapshot).unwrap();
    let old = owner.admit(1).unwrap();
    owner.publish_business(&old, business()).unwrap();
    owner.suspend().unwrap();
    owner.activate(&snapshot).unwrap();
    assert_eq!(
        owner.publish_business(&old, business()),
        Err(Error::StalePermit)
    );
    assert!(owner
        .poll_update(Cursor {
            epoch: 1,
            frame_id: 0
        })
        .unwrap()
        .business
        .is_none());
}
#[cfg(windows)]
#[test]
fn business_uses_existing_notice_even_without_new_frame() {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::{
        Foundation::{HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
        System::Threading::WaitForSingleObject,
    };
    let (owner, snapshot) = prepared();
    owner.activate(&snapshot).unwrap();
    let permit = owner.admit(1).unwrap();
    let cursor = Cursor {
        epoch: 1,
        frame_id: 0,
    };
    owner.poll_update(cursor).unwrap();
    let handle = HANDLE(owner.change_handle().as_raw_handle());
    assert_eq!(unsafe { WaitForSingleObject(handle, 0) }, WAIT_TIMEOUT);
    owner.publish_business(&permit, business()).unwrap();
    assert_eq!(unsafe { WaitForSingleObject(handle, 0) }, WAIT_OBJECT_0);
    let update = owner.poll_update(cursor).unwrap();
    assert!(update.frame.is_none() && update.business.is_some());
    assert_eq!(unsafe { WaitForSingleObject(handle, 0) }, WAIT_TIMEOUT);
}

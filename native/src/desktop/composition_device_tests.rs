//! Fault injection covers only cache/fallback state machines, not actual GPU reset or WGC acceptance.
use super::*;
use std::cell::Cell;

#[test]
fn healthy_cache_reuses_active_lease_and_does_not_retain_last_owner() {
    let slot = RefCell::new(Weak::new());
    let first = cached(&slot, || Ok::<_, ()>(7), |_| true).unwrap();
    let second = cached(
        &slot,
        || -> std::result::Result<i32, ()> { panic!("recreated healthy device") },
        |_| true,
    )
    .unwrap();
    assert!(Rc::ptr_eq(&first, &second));
    let weak = Rc::downgrade(&first);
    drop(first);
    assert!(weak.upgrade().is_some());
    drop(second);
    assert!(weak.upgrade().is_none());
    assert!(slot.borrow().upgrade().is_none());
    let replacement = cached(&slot, || Ok::<_, ()>(9), |_| true).unwrap();
    assert_eq!(*replacement, 9);
    assert_ne!(weak.as_ptr(), Rc::as_ptr(&replacement));
}
#[test]
fn unhealthy_device_replacement_preserves_old_owner_and_new_survivor() {
    let slot = RefCell::new(Weak::new());
    let old = cached(&slot, || Ok::<_, ()>(Cell::new(true)), |_| true).unwrap();
    old.set(false);
    let replacement = cached(&slot, || Ok::<_, ()>(Cell::new(true)), Cell::get).unwrap();
    assert!(!Rc::ptr_eq(&old, &replacement));
    assert!(!old.get() && replacement.get());
    drop(old);
    let survivor = cached(
        &slot,
        || -> std::result::Result<Cell<bool>, ()> { panic!("lost replacement") },
        Cell::get,
    )
    .unwrap();
    assert!(Rc::ptr_eq(&replacement, &survivor));
}
#[test]
fn failed_device_creation_publishes_nothing_and_can_recover_without_retiring_old_owner() {
    let slot = RefCell::new(Weak::new());
    let old = cached(&slot, || Ok::<_, &str>(7), |_| true).unwrap();
    assert_eq!(
        cached(&slot, || Err("creation failed"), |_| false),
        Err("creation failed")
    );
    assert!(Rc::ptr_eq(&old, &slot.borrow().upgrade().unwrap()));
    let next = cached(&slot, || Ok::<_, &str>(9), |_| false).unwrap();
    assert_eq!((*old, *next), (7, 9));
    let empty = RefCell::<Weak<i32>>::new(Weak::new());
    assert!(cached(&empty, || Err::<i32, _>("first creation failed"), |_| true).is_err());
    assert!(empty.borrow().upgrade().is_none());
}
#[test]
fn actual_software_cache_is_distinct_and_does_not_retain_last_lease() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let first = acquire(Backend::D3d11Warp).unwrap();
    let second = acquire(Backend::D3d11Warp).unwrap();
    assert!(Rc::ptr_eq(&first, &second));
    // Software-device creation does not establish that the machine necessarily has usable hardware.
    if let Ok(hardware) = acquire(Backend::D3d11Hardware) {
        assert!(!Rc::ptr_eq(&first, &hardware));
        assert_ne!(first.get(), hardware.get());
    }
    let weak = Rc::downgrade(&first);
    drop(first);
    assert!(weak.upgrade().is_some());
    drop(second);
    assert!(weak.upgrade().is_none());
}
#[test]
fn actual_default_device_cache_is_isolated_by_thread() {
    use windows::core::Interface;
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let (backend, current) = match acquire(Backend::D3d11Hardware) {
        Ok(device) => (Backend::D3d11Hardware, device),
        Err(_) => (Backend::D3d11Warp, acquire(Backend::D3d11Warp).unwrap()),
    };
    let current_id = current.get().as_raw() as usize;
    let (ready, receive) = std::sync::mpsc::sync_channel(1);
    let (release, wait) = std::sync::mpsc::sync_channel(1);
    let other = std::thread::spawn(move || {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        let device = acquire(backend).unwrap();
        let id = device.get().as_raw() as usize;
        ready.send(id).unwrap();
        // Synchronization failures also exit within bounds; do not leave tests waiting at a Barrier for an already-panicked peer.
        let _ = wait.recv_timeout(std::time::Duration::from_secs(10));
    });
    let other_id = receive.recv_timeout(std::time::Duration::from_secs(10));
    let _ = release.send(());
    other.join().unwrap();
    assert_ne!(current_id, other_id.unwrap());
}

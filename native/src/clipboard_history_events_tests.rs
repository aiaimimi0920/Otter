//! 只验证信号与清理协议，不注册真实系统剪贴板监听。
use super::*;
use std::sync::Mutex;
use windows::{core::Error, Win32::Foundation::E_FAIL};

#[test]
fn changes_coalesce_and_late_callbacks_do_not_keep_owner_alive() {
    let dirty = Arc::new(AtomicBool::new(true));
    let weak = Arc::downgrade(&dirty);
    assert!(dirty.swap(false, Ordering::AcqRel));
    assert!(!dirty.swap(false, Ordering::AcqRel));
    for _ in 0..1000 {
        invalidate(&weak);
    }
    assert!(dirty.swap(false, Ordering::AcqRel));
    assert!(!dirty.swap(false, Ordering::AcqRel));
    invalidate(&weak);
    assert!(dirty.load(Ordering::Acquire));
    drop(dirty);
    invalidate(&weak);
    assert!(weak.upgrade().is_none());
}

static REMOVED: Mutex<Vec<i64>> = Mutex::new(Vec::new());
fn remove(token: i64) -> Result<()> {
    REMOVED.lock().unwrap().push(token);
    Err(Error::new(E_FAIL, "Synthetic unsubscribe failure"))
}

#[test]
fn partial_registration_unwinds_all_tokens_even_after_removal_failure() {
    REMOVED.lock().unwrap().clear();
    let register = || -> Result<()> {
        let mut tokens = Tokens(Vec::new());
        tokens.0.push((11, remove));
        tokens.0.push((22, remove));
        Err(Error::new(E_FAIL, "Synthetic registration failure"))
    };
    assert!(register().is_err());
    assert_eq!(*REMOVED.lock().unwrap(), vec![22, 11]);
}

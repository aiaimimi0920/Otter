//! 本地编辑资格不等于动作提交资格；真实授权 HWND/独占 owner 和撤销边界回归。
use super::*;

#[test]
fn surface_edit_policy_survives_batch_owner_transfer_without_authorizing_submission() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    assert!(window.sync_input(true, true, 0).unwrap());
    let events = vec![event(&window)];
    let owner = window.begin_batch(&events, 0).unwrap().unwrap();
    assert!(window.lifecycle().is_none());
    assert!(!window.sync_input(true, true, 0).unwrap());
    assert!(window.local_edit_enabled());
    assert!(window.begin_batch(&events, 0).unwrap().is_none());
    assert!(window.finish_batch(&events, owner).is_ok());
    // 应用的未结算结果/容量门禁仍可只阻止提交，不必丢失本地编辑。
    assert!(!window.sync_input(true, false, 0).unwrap());
    assert!(window.local_edit_enabled());
    assert!(window.sync_input(true, true, 0).unwrap());
    assert!(!window.sync_input(false, true, 0).unwrap());
    assert!(!window.local_edit_enabled());
}
#[test]
fn surface_edit_policy_revocation_closes_even_with_outstanding_batch() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    window.sync_input(true, true, 0).unwrap();
    let events = vec![event(&window)];
    let owner = window.begin_batch(&events, 0).unwrap().unwrap();
    assert!(window.local_edit_enabled());
    drop(grant);
    assert!(!window.sync_input(true, true, 0).unwrap());
    assert!(!window.is_alive());
    assert!(!window.local_edit_enabled());
    assert!(window.finish_batch(&events, owner).is_ok());
    assert!(!window.sync_input(true, true, 0).unwrap());
}
#[test]
fn surface_edit_policy_never_opens_before_confirmed_activation() {
    let _com = Apartment::new();
    let mut grant = grant();
    let lease = grant.lease("https://localhost:1234", 0).unwrap();
    let mut window = AuthorizedWindow::new(&outcome(), lease, 0, [200, 140]).unwrap();
    assert!(!window.sync_input(true, true, 0).unwrap());
    assert!(!window.local_edit_enabled());
    let owner = window.begin_activation(0).unwrap().unwrap();
    assert!(!window.sync_input(true, true, 0).unwrap());
    assert!(!window.local_edit_enabled());
    assert!(window.finish_activation(owner).is_ok());
    assert!(!window.sync_input(true, true, 0).unwrap());
}

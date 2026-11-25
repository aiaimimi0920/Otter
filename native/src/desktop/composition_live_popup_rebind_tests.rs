//! Family scope-update regressions: real source-hook leases and owned fake capture tasks, without WGC or source-input workers.
use super::*;
use crate::live_window_target::recovery::popup_layer::fixture::Fixture;
use std::{
    cell::Cell,
    os::windows::io::AsRawHandle,
    sync::{atomic::AtomicUsize, atomic::Ordering, Arc},
};

fn mirror(scope: Scope, owner: HWND) -> Session {
    let target = scope.target().unwrap();
    let active = Arc::new(AtomicUsize::new(0));
    let (job, window) = LiveJob::spawn_bound(
        &active,
        |consumer| {
            let window = LiveWindow::new(consumer, 80, 60).map_err(io::Error::other)?;
            window
                .configure_popup(owner, true)
                .map_err(io::Error::other)?;
            Ok(window)
        },
        |_, cancel| {
            let deadline = Instant::now() + Duration::from_secs(5);
            while !cancel.load(Ordering::Acquire) {
                if Instant::now() >= deadline {
                    return Err("fake popup capture was not stopped".into());
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            Ok(())
        },
    )
    .unwrap();
    let mut session = Session::new(job, window);
    session.target = Some(target);
    session.popup_scope = Some(scope);
    session
}
fn finish(session: &mut Session) {
    session.stop();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(result) = session.poll() {
            result.unwrap();
            return;
        }
        assert!(Instant::now() < deadline, "popup cleanup did not finish");
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn leased(fixture: &Fixture, hwnd: HWND) -> Scope {
    Scope::bind(
        &Target::bind(fixture.root.0).unwrap(),
        Some(fixture.owner.identity()),
        hwnd,
    )
    .unwrap()
}

#[test]
fn a_new_display_on_the_same_hwnd_cannot_adopt_the_old_mirror_session() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let fixture = Fixture::new();
    let first = fixture.show();
    let mut session = mirror(leased(&fixture, fixture.popup.0), fixture.b.0);
    let second = fixture.show();
    assert_ne!(first.instance, second.instance);
    assert!(!first.valid());
    assert!(second.valid());
    assert!(!session
        .rebind_popup_scope(leased(&fixture, fixture.popup.0), fixture.b.0)
        .unwrap());
    assert!(session.stopping);
    assert!(session.window.is_none());
    match session.popup_scope.as_ref().unwrap() {
        Scope::Leased { lease, .. } => assert_eq!(*lease, first),
        _ => panic!("old display scope must be preserved until cleanup"),
    }
    finish(&mut session);
}

#[test]
fn a_retiring_popup_does_not_accept_a_new_scope_even_if_it_is_valid() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let fixture = Fixture::new();
    fixture.show();
    let scope = leased(&fixture, fixture.popup.0);
    let mut session = mirror(scope.clone(), fixture.b.0);
    session.stop();
    assert!(scope.target().is_ok());
    assert!(!session.rebind_popup_scope(scope, fixture.b.0).unwrap());
    assert!(session.stopping);
    assert!(session.window.is_none());
    finish(&mut session);
}

#[test]
fn a_valid_nearer_parent_keeps_the_same_mirror_and_capture_owner() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let bounds = rect(-28000, -28000, 80, 60);
    let source = Source::new(None, bounds, true);
    let parent = Source::new(Some(source.0), bounds, true);
    let child = Source::new(Some(parent.0), bounds, true);
    let b = Source::new(None, bounds, true);
    let parent_b = Source::new(Some(b.0), bounds, true);
    let owner = crate::live_hover_owner::Owner::new(parent_b.0).unwrap();
    owner.control(true).unwrap();
    let root = Target::bind(source.0).unwrap();
    let initial = Scope::bind(&root, None, child.0).unwrap();
    let next = Scope::bind(&root, None, parent.0)
        .unwrap()
        .bind_descendant(Some(owner.identity()), child.0)
        .unwrap();
    let mut session = mirror(initial, b.0);
    let hwnd = session.window.as_ref().unwrap().popup_hwnd();
    let identity = session.input_identity.clone();
    let capture = session.job.completion_handle().unwrap().as_raw_handle();
    assert!(session.rebind_popup_scope(next, b.0).unwrap());
    assert!(!session.stopping);
    assert_eq!(session.window.as_ref().unwrap().popup_hwnd(), hwnd);
    assert!(Arc::ptr_eq(&identity, &session.input_identity));
    assert_eq!(
        session.job.completion_handle().unwrap().as_raw_handle(),
        capture
    );
    assert_eq!(unsafe { GetWindow(hwnd, GW_OWNER).unwrap() }, parent_b.0);
    finish(&mut session);
}

thread_local! {
    static REOPEN: Cell<Option<(HWND, *const Fixture)>> = const { Cell::new(None) };
}
unsafe extern "system" fn reentrant_show(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code >= 0 && lp.0 != 0 {
        let call = &*(lp.0 as *const CWPSTRUCT);
        if call.message == WM_WINDOWPOSCHANGING {
            if let Some((mirror, fixture)) = REOPEN.with(Cell::get) {
                if call.hwnd == mirror {
                    REOPEN.with(|slot| slot.set(None));
                    // Use pointers only within this test thread's guard scope; remove the slot before reentrancy to prevent recursion.
                    (&*fixture).show();
                }
            }
        }
    }
    CallNextHookEx(None, code, wp, lp)
}
struct Reopen<'a>(HHOOK, std::marker::PhantomData<&'a Fixture>);
impl<'a> Reopen<'a> {
    fn during_reparent(mirror: HWND, fixture: &'a Fixture) -> Self {
        let hook = unsafe {
            SetWindowsHookExW(
                WH_CALLWNDPROC,
                Some(reentrant_show),
                None,
                windows::Win32::System::Threading::GetCurrentThreadId(),
            )
            .unwrap()
        };
        REOPEN.with(|slot| slot.set(Some((mirror, fixture))));
        Self(hook, std::marker::PhantomData)
    }
}
impl Drop for Reopen<'_> {
    fn drop(&mut self) {
        REOPEN.with(|slot| slot.set(None));
        unsafe {
            UnhookWindowsHookEx(self.0).unwrap();
        }
    }
}

#[test]
fn display_change_inside_reparent_is_checked_before_committing_the_scope() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let fixture = Fixture::new();
    let first = fixture.show();
    let bounds = rect(-28000, -28000, 80, 60);
    let child = Source::new(Some(fixture.popup.0), bounds, true);
    child.show();
    let parent_b = Source::new(Some(fixture.b.0), bounds, true);
    let owner = crate::live_hover_owner::Owner::new(parent_b.0).unwrap();
    owner.control(true).unwrap();
    let mut session = mirror(leased(&fixture, child.0), fixture.b.0);
    let next = leased(&fixture, fixture.popup.0)
        .bind_descendant(Some(owner.identity()), child.0)
        .unwrap();
    let guard = Reopen::during_reparent(session.window.as_ref().unwrap().popup_hwnd(), &fixture);
    assert!(!session.rebind_popup_scope(next, fixture.b.0).unwrap());
    assert!(
        REOPEN.with(Cell::get).is_none(),
        "real window reentry must run"
    );
    assert!(!first.valid());
    assert!(session.stopping);
    assert!(session.window.is_none());
    drop(guard);
    finish(&mut session);
}

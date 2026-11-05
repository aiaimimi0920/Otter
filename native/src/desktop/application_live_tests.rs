use super::*;
#[test]
fn surface_revocation_handles_share_the_live_wait_budget_without_overflow() {
    assert!(fits_wait_budget(16, 0));
    assert!(!fits_wait_budget(16, 1));
    assert!(fits_wait_budget(15, 3));
    assert!(!fits_wait_budget(15, 4));
    assert!(fits_wait_budget(14, 4));
    assert!(!fits_wait_budget(usize::MAX, usize::MAX));
}
use crate::live_texture::Texture;
use crate::{
    desktop::{composition::live::window::LiveWindow, document_window::DocumentWindow, Apartment},
    live_job::LiveJob,
};
use std::{
    io,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use windows::Win32::{
    Foundation::WAIT_OBJECT_0,
    System::{LibraryLoader::GetModuleHandleW, Threading::WaitForSingleObject},
};

struct Class(HINSTANCE);
#[test]
fn wait_set_deduplicates_shared_source_handles_and_rejects_overflow() {
    let handle = |value: usize| HANDLE(value as *mut _);
    let mut handles = [HANDLE::default(); MAX_SESSIONS * 2];
    let mut count = 0;
    for value in 1..=MAX_SESSIONS * 2 {
        append_wait_handle(&mut handles, &mut count, handle(value)).unwrap();
        append_wait_handle(&mut handles, &mut count, handle(value)).unwrap();
    }
    assert_eq!(count, MAX_SESSIONS * 2);
    let extra = handle(count + 1);
    assert!(append_wait_handle(&mut handles, &mut count, extra).is_err());
    assert_eq!(count, MAX_SESSIONS * 2);
}
impl Drop for Class {
    fn drop(&mut self) {
        unsafe {
            let _ = UnregisterClassW(w!("Otter.Native.Workspace"), Some(self.0));
        }
    }
}
#[test]
fn application_consumes_invalid_request_and_reaps_real_worker_without_capturing_desktop() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let instance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
        assert_ne!(
            RegisterClassW(&WNDCLASSW {
                hInstance: instance,
                lpszClassName: w!("Otter.Native.Workspace"),
                lpfnWndProc: Some(crate::desktop::procedure::window_proc),
                ..Default::default()
            }),
            0
        );
        let _class = Class(instance);
        let document =
            DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
        let mut app = Application::new(instance, document);
        *app.documents[0].state.live_request.borrow_mut() = Some(
            crate::desktop::capture_input::live::Request::Region(RECT::default()),
        );
        assert!(app.process_live(false).is_err());
        assert!(app.documents[0].state.live_request.borrow().is_none());
        assert!(app.live_sessions.is_empty());
        assert!(app.documents[0].is_alive());
        let target_window = crate::desktop::Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("Live request fixture"),
                WS_POPUP,
                0,
                0,
                1,
                1,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let target = crate::live_window_target::Target::bind(target_window.0).unwrap();
        drop(target_window);
        *app.documents[0].state.live_request.borrow_mut() =
            Some(crate::desktop::capture_input::live::Request::Window(target));
        assert!(app.process_live(false).is_err());
        assert!(app.documents[0].state.live_request.borrow().is_none());
        assert!(app.live_sessions.is_empty());
        for fail in [false, true] {
            let active = Arc::new(AtomicUsize::new(0));
            let (job, window) = LiveJob::<Texture>::spawn_bound(
                &active,
                |consumer| LiveWindow::new(consumer, 128, 128).map_err(io::Error::other),
                move |_, _| {
                    if fail {
                        Err("injected application Live failure".into())
                    } else {
                        Ok(())
                    }
                },
            )
            .unwrap();
            let session = Session::new(job, window);
            assert_eq!(
                WaitForSingleObject(
                    HANDLE(session.completion_handle().unwrap().as_raw_handle()),
                    2000
                ),
                WAIT_OBJECT_0
            );
            app.live_sessions.push(session);
            let result = app.poll_live(false);
            assert_eq!(result.is_err(), fail);
            assert!(app.live_sessions.is_empty());
            assert_eq!(active.load(Ordering::Acquire), 0);
        }
        assert!(app.documents[0].is_alive());
    }
}

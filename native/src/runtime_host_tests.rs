//! Joint unit tests across `otter-runtime`/`otter-native` host boundaries, retaining 100% of original assertion coverage.
use crate::{
    session_vectors::{restore, Error},
    stroke_style::{Color, PaintAlpha},
    viewport::Point,
    workspace_id::UnitId,
};
use serde_json::{value::RawValue, Value};

#[path = "session_vector_alpha_tests.rs"]
mod session_vector_alpha_tests;
#[path = "session_vector_path_tests.rs"]
mod session_vector_path_tests;

#[test]
fn session_old_absence_and_explicit_overrides_restore_but_invalid_candidate_fails() {
    let id = |name: &str| UnitId::parse(name).unwrap();
    for field in [
        "".to_owned(),
        r#", "portVisibility":{"中文":false,"output":true}"#.into(),
    ] {
        let raw = format!(
            r#"{{"future":{{"keep":1}},"stickers":[{{"id":"unit","x":0,"y":0,"w":1,"h":1{field}}}],"links":[]}}"#
        );
        let doc = crate::session_document::decode(raw.as_bytes()).unwrap();
        assert_eq!(doc.original_json(), raw.as_bytes());
        assert_eq!(
            doc.workspace.port_visibility(&id("unit")).get("中文"),
            (!field.is_empty()).then_some(false)
        );
    }
    for invalid in ["null", "[]", r#"{"a":true,"a":false}"#, r#"{"a":"false"}"#] {
        let raw = format!(
            r#"{{"stickers":[{{"id":"unit","x":0,"y":0,"w":1,"h":1,"portVisibility":{invalid}}}],"links":[]}}"#
        );
        assert!(crate::session_document::decode(raw.as_bytes()).is_err());
    }
}

#[cfg(windows)]
#[test]
fn live_job_window_bound_validates_target_before_and_after_destroy() {
    use crate::live_window_target::Target;
    use windows::{
        core::w,
        Win32::{
            Foundation::HWND,
            System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED},
            UI::WindowsAndMessaging::*,
        },
    };
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe {
                CoUninitialize();
            }
        }
    }
    struct Window(HWND);
    impl Drop for Window {
        fn drop(&mut self) {
            unsafe {
                let _ = DestroyWindow(self.0);
            }
        }
    }
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
    }
    let _apartment = Apartment;
    unsafe {
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("target"),
            WS_POPUP,
            0,
            0,
            32,
            32,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let _window = Window(hwnd);
        let target = Target::bind(hwnd).unwrap();
        let mut binding_ran = false;
        let rejected =
            crate::live_job::LiveJob::start_window_bound(Target::bind(hwnd).unwrap(), |_| {
                binding_ran = true;
                Err::<(), _>(std::io::Error::other("injected binding failure"))
            });
        assert!(
            binding_ran,
            "must reach binding rather than fail COM admission"
        );
        assert_eq!(
            rejected.err().unwrap().to_string(),
            "injected binding failure"
        );
        DestroyWindow(hwnd).unwrap();
        assert!(
            crate::live_job::LiveJob::start_window_bound(target, |_| -> std::io::Result<()> {
                panic!("closed target must not bind")
            })
            .is_err()
        );
    }
}

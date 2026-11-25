//! Read-only mapping regression tests on owned framed windows; no source input, DLL loading, or system-mouse movement.
use super::*;
use crate::{
    live_input::{Backend as _, Event},
    live_window_target::input_backend::Backend,
};
use windows::core::w;

struct Window(HWND);
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.0).unwrap();
        }
    }
}
fn framed_window() -> (Window, HWND) {
    unsafe {
        let root = Window(
            CreateWindowExW(
                WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                w!("STATIC"),
                w!(""),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                -28000,
                -28000,
                320,
                240,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let child = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!(""),
            WS_CHILD | WS_VISIBLE,
            20,
            2,
            80,
            4,
            Some(root.0),
            None,
            None,
            None,
        )
        .unwrap();
        (root, child)
    }
}

#[test]
#[ignore = "requires interactive default desktop; creates own off-screen framed window"]
fn full_capture_and_local_hit_test_share_frame_not_client_coordinates() {
    unsafe {
        let old = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        assert!(!old.0.is_null());
        let _dpi = Dpi(old);
        let (window, child) = framed_window();
        let root = Target::bind(window.0).unwrap();
        let destination = Destination::bind(&root, child).unwrap();
        let bounds = frame_bounds(window.0).unwrap();
        let mut screen = POINT { x: 5, y: 1 };
        assert!(ClientToScreen(child, &mut screen).as_bool());
        let point = [
            f64::from(screen.x - bounds.left) / f64::from(bounds.right - bounds.left - 1),
            f64::from(screen.y - bounds.top) / f64::from(bounds.bottom - bounds.top - 1),
        ];
        let mapped = map_capture(&root, &destination, point, None).unwrap();
        assert_eq!(mapped.screen, [screen.x, screen.y]);
        assert_eq!(mapped.destination_client, [5, 1]);
        let mut local = Backend::new_local(root.clone(), None).unwrap();
        assert_eq!(
            local.resolve(Event::Move { point }, None).unwrap().hwnd(),
            child
        );

        // Legacy remote full-window input still uses the client area; local ordinary-application changes must not alter its protocol.
        let mut remote = Backend::new(root.clone(), None).unwrap();
        assert_eq!(
            remote.resolve(Event::Move { point }, None).unwrap().hwnd(),
            window.0
        );
        let mut client = RECT::default();
        GetClientRect(window.0, &mut client).unwrap();
        assert_eq!(
            map(&root, &destination, point, None).unwrap().root_client,
            rectangle_point(client, point).unwrap()
        );
        assert_ne!(
            map(&root, &destination, point, None)
                .unwrap()
                .destination_client,
            mapped.destination_client
        );

        // Full-frame corners retain nonclient offsets; no clamping/stretching into the client area or fake NC mouse messages.
        let corner = map_capture(&root, &destination, [0.0; 2], None).unwrap();
        assert_eq!(corner.screen, [bounds.left, bounds.top]);
        assert!(corner.root_client[1] < 0);
        let end = map_capture(&root, &destination, [1.0; 2], None).unwrap();
        assert_eq!(end.screen, [bounds.right - 1, bounds.bottom - 1]);

        // Selected local pixels retain the original crop semantics without adding the title-bar offset twice.
        let region = Some(FrameRegion {
            left: (screen.x - bounds.left) as u32,
            top: (screen.y - bounds.top) as u32,
            width: 1,
            height: 1,
        });
        for point in [[0.0; 2], [0.5; 2], [1.0; 2]] {
            let captured = map_capture(&root, &destination, point, region).unwrap();
            assert_eq!(captured.destination_client, [5, 1]);
            assert_eq!(captured, map(&root, &destination, point, region).unwrap());
        }
    }
}

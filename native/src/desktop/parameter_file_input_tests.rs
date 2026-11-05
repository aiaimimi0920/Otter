//! 选择结果与请求准入的回归；只建自有隐藏HWND，不打开系统模态或读取文件。
use super::*;

struct Fixture {
    window: super::super::Window,
    state: State,
}
impl Fixture {
    unsafe fn new() -> Self {
        Self {
            window: super::super::Window(
                CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("STATIC"),
                    w!("file choice fixture"),
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
                .unwrap(),
            ),
            state: State::new(None, false).unwrap(),
        }
    }
    fn request(&self) -> Request {
        Request {
            path: PathBuf::from("never-read.png"),
            target: Target::Sticker,
            stamp: Stamp::capture(&self.state),
            view: None,
        }
    }
}

#[test]
fn cancel_is_not_rejection_and_never_queues_or_mutates() {
    unsafe {
        let f = Fixture::new();
        let before = Stamp::capture(&f.state);
        assert_eq!(
            finish_choice(f.window.0, &f.state, None, &|| false).unwrap(),
            Outcome::Cancelled
        );
        assert!(before.matches(&f.state));
        assert!(f.state.parameter_file_request.borrow().is_none());
        assert!(f.state.sticker_input.snapshot().asset.is_none());
    }
}

#[test]
fn selected_path_rechecks_owner_configuration_and_pending_slot() {
    unsafe {
        let f = Fixture::new();
        assert_eq!(
            finish_choice(f.window.0, &f.state, Some(f.request()), &|| false).unwrap(),
            Outcome::Rejected
        );
        let old = f.request();
        f.state.sticker_input.toggle().unwrap();
        assert_eq!(
            finish_choice(f.window.0, &f.state, Some(old), &|| true).unwrap(),
            Outcome::Rejected
        );
        assert!(f.state.parameter_file_request.borrow().is_none());
        let before = Stamp::capture(&f.state);
        assert_eq!(
            finish_choice(f.window.0, &f.state, Some(f.request()), &|| true).unwrap(),
            Outcome::Accepted
        );
        assert_eq!(
            f.state
                .parameter_file_request
                .borrow()
                .as_ref()
                .unwrap()
                .path,
            PathBuf::from("never-read.png")
        );
        assert!(before.matches(&f.state));
        assert_eq!(
            finish_choice(f.window.0, &f.state, Some(f.request()), &|| true).unwrap(),
            Outcome::Rejected
        );
    }
}

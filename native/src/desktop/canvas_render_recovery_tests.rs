//! 恢复状态的确定性失败/超时模型；真实 DComp 成功与窗口调度另由 Canvas/窗口测试覆盖。
use super::*;
use crate::desktop::{image::Image, Apartment};

fn canvas() -> Canvas {
    Canvas::new(Some(Image {
        width: 2,
        height: 2,
        pixels: [30, 60, 90, 255].repeat(4).into(),
    }))
    .unwrap()
}

#[test]
fn pending_and_terminal_recovery_survive_discard_and_source_replacement() {
    let _apartment = Apartment::new().unwrap();
    for status in [
        Status::Pending,
        Status::TimedOut,
        Status::Failed {
            stage: Stage::Completion,
            code: E_FAIL,
        },
    ] {
        let mut canvas = canvas();
        canvas.recovery.status = Some(Status::Pending);
        canvas.resolve_recovery(status);
        let stamp = canvas.editor.document_stamp();
        canvas.discard_device();
        assert!(stamp.matches(&canvas.editor));
        canvas
            .replace_image(Image {
                width: 1,
                height: 1,
                pixels: vec![0, 0, 0, 255].into(),
            })
            .unwrap();
        assert_eq!(canvas.recovery.status, Some(status));
        assert!(canvas.ensure_render_admitted().is_err());
        assert!(!canvas.recover_render());
        assert_eq!(canvas.confirmed_paints, 0);
        assert!(!canvas.compatibility_renderer);
    }
}

#[test]
fn timeout_failure_and_cancel_reject_late_success() {
    let _apartment = Apartment::new().unwrap();
    for status in [
        Status::TimedOut,
        Status::Failed {
            stage: Stage::Commit,
            code: E_FAIL,
        },
    ] {
        let mut canvas = canvas();
        canvas.recovery.status = Some(Status::Pending);
        canvas.resolve_recovery(status);
        canvas.resolve_recovery(Status::Complete);
        assert_eq!(canvas.poll_render_recovery(), Some(status));
        assert!(canvas.render_suspended());
        assert!(!canvas.compatibility_renderer);
    }
    let mut canvas = canvas();
    canvas.recovery.status = Some(Status::Pending);
    canvas.close_render();
    canvas.resolve_recovery(Status::Complete);
    assert!(canvas.render_suspended());
    assert!(!canvas.compatibility_renderer);
}

#[test]
fn completion_is_single_use_and_does_not_touch_document_or_counts() {
    let _apartment = Apartment::new().unwrap();
    let mut canvas = canvas();
    let stamp = canvas.editor.document_stamp();
    canvas.recovery.status = Some(Status::Pending);
    canvas.resolve_recovery(Status::Complete);
    assert!(canvas.ensure_render_admitted().is_ok());
    assert!(canvas.compatibility_renderer);
    assert_eq!(canvas.poll_render_recovery(), None);
    canvas.resolve_recovery(Status::TimedOut);
    assert_eq!(canvas.poll_render_recovery(), None);
    assert!(stamp.matches(&canvas.editor));
    assert_eq!(canvas.paints, 0);
    assert_eq!(canvas.confirmed_paints, 0);
}

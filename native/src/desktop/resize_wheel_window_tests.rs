//! Real wheel continuation during active blur work; no desktop switching or system input injection.
use super::*;
use std::time::{Duration, Instant};

impl Fixture {
    unsafe fn blur() -> Self {
        let f = Self::new();
        {
            let mut c = f.state.canvas.borrow_mut();
            let pixels = c.image.as_ref().unwrap().pixels.clone();
            c.editor
                .begin_blur(
                    Point { x: 30.0, y: 30.0 },
                    crate::blur_image::BlurImage::new(100, 100, pixels, 4).unwrap(),
                )
                .unwrap();
            c.editor.extend(Point { x: 40.0, y: 40.0 }).unwrap();
            assert!(c.editor.commit());
            let revision = c.editor.revision();
            c.selected = Some((0, revision));
            c.multi_selection = Some((vec![0, 2], revision));
        }
        f
    }
    unsafe fn annotation_wheel(&self, delta: i16, own: bool) {
        if own {
            self.wheel(&[VK_CONTROL, VK_MENU, VK_SHIFT], delta);
        } else {
            self.wheel(&[VK_CONTROL, VK_MENU], delta);
        }
    }
    unsafe fn drain(&self) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while self.state.resize_job.borrow().is_busy() {
            assert!(Instant::now() < deadline, "wheel worker did not finish");
            desktop::resize_input::poll(self.window.0, &self.state);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(self.state.error.borrow().is_none());
    }
    unsafe fn burst(&self) {
        for delta in [20, 30, -10, 5] {
            self.annotation_wheel(delta, false);
        }
        assert!(self.state.resize_job.borrow().is_busy());
    }
}

#[test]
fn mixed_burst_matches_serial_reference_and_preserves_each_undo_step() {
    unsafe {
        let burst = Fixture::blur();
        let serial = Fixture::blur();
        let before = burst.rect();
        let viewport = burst.state.canvas.borrow().viewport;
        let revision = burst.state.canvas.borrow().editor.revision();
        burst.state.canvas.borrow_mut().window_opacity = 0.5;
        let mut expected = Vec::new();
        for i in 0..20 {
            let delta = [12, -7, 5, 3][i % 4];
            let own = i % 3 == 0;
            serial.annotation_wheel(delta, own);
            serial.drain();
            expected.push(serial.state.canvas.borrow().editor.elements().to_vec());
            burst.annotation_wheel(delta, own);
        }
        // Without polling, there can be only one computation; all subsequent input is accepted without premature model writes.
        assert_eq!(burst.state.canvas.borrow().editor.revision(), revision);
        burst.drain();
        let mut c = burst.state.canvas.borrow_mut();
        assert_eq!(c.editor.revision(), revision + 20);
        assert_eq!(c.selected_indices(), vec![0, 2]);
        assert_eq!(c.viewport, viewport);
        assert_eq!(c.window_opacity, 0.5);
        assert_eq!(burst.rect(), before);
        for step in (0..20).rev() {
            for (actual, expected) in c.editor.elements().iter().zip(&expected[step]) {
                assert_eq!(actual.points, expected.points);
                assert_eq!(actual.style(), expected.style());
                if let Some(blur) = actual.blur.as_ref() {
                    let other = expected.blur.as_ref().unwrap();
                    close(blur.strength(), other.strength());
                    assert_eq!(&blur.pixels()[..], &other.pixels()[..]);
                }
            }
            assert!(c.editor.undo());
        }
        close(
            c.editor.elements()[2].blur.as_ref().unwrap().strength(),
            4.0,
        );
        assert!(!desktop::pin_window::animation::busy(&burst.state));
        assert!(!desktop::pin_window::preview::active(&burst.state));
    }
}

#[test]
fn completed_but_unpolled_worker_still_accepts_following_inputs_in_order() {
    unsafe {
        let f = Fixture::blur();
        let revision = f.state.canvas.borrow().editor.revision();
        f.annotation_wheel(20, false);
        // is_busy includes completed but undelivered workers; elapsed time must not justify discarding subsequent input.
        std::thread::sleep(Duration::from_millis(80));
        f.annotation_wheel(-10, true);
        f.annotation_wheel(15, false);
        f.drain();
        assert_eq!(f.state.canvas.borrow().editor.revision(), revision + 3);
        f.annotation_wheel(5, false);
        f.drain();
        assert_eq!(f.state.canvas.borrow().editor.revision(), revision + 4);
    }
}

#[test]
fn escape_cancels_current_and_pending_inputs_without_deleting_pin() {
    unsafe {
        let f = Fixture::blur();
        let stamp = f.state.canvas.borrow().editor.document_stamp();
        f.burst();
        let _keys = Keys::held(&[]);
        SendMessageW(
            f.window.0,
            WM_KEYDOWN,
            Some(WPARAM(VK_ESCAPE.0 as usize)),
            None,
        );
        f.drain();
        assert!(IsWindow(Some(f.window.0)).as_bool());
        assert!(stamp.matches(&f.state.canvas.borrow().editor));
        let mut c = f.state.canvas.borrow_mut();
        c.selected = Some((2, c.editor.revision()));
        c.multi_selection = None;
        drop(c);
        f.annotation_wheel(10, false);
        f.drain();
        close(
            f.state.canvas.borrow().editor.elements()[2]
                .blur
                .as_ref()
                .unwrap()
                .strength(),
            4.0 * 0.01f32.exp(),
        );
    }
}

#[test]
fn changed_edit_target_or_document_rejects_the_entire_old_batch() {
    unsafe {
        for case in 0..6 {
            let f = Fixture::blur();
            f.burst();
            match case {
                0 => desktop::edit_mode::toolbar_mode(f.window.0, &f.state, false).unwrap(),
                1 => {
                    f.state.canvas.borrow_mut().select_tool(Tool::Pan);
                }
                2 => desktop::edit_mode::execute(f.window.0, &f.state, false).unwrap(),
                3 => f.state.canvas.borrow_mut().multi_selection = None,
                4 => {
                    assert!(f.state.canvas.borrow_mut().editor.undo());
                }
                _ => {
                    let replacement = f.state.canvas.borrow().editor.export_snapshot();
                    f.state.canvas.borrow_mut().editor = replacement;
                }
            }
            let stamp = f.state.canvas.borrow().editor.document_stamp();
            let selected = f.state.canvas.borrow().selected_indices();
            f.drain();
            assert!(
                stamp.matches(&f.state.canvas.borrow().editor),
                "case {case}"
            );
            assert_eq!(f.state.canvas.borrow().selected_indices(), selected);
        }
    }
}

#[test]
fn modal_hidden_capture_and_lock_changes_do_not_resume_old_batch() {
    unsafe {
        for case in 0..4 {
            let f = Fixture::blur();
            let stamp = f.state.canvas.borrow().editor.document_stamp();
            f.burst();
            match case {
                0 => f.state.file_dialog.set(true),
                1 => f.state.group_hidden.set(true),
                2 => f.state.capture_active.set(true),
                _ => f.state.group_locked.set(true),
            }
            f.drain();
            assert!(stamp.matches(&f.state.canvas.borrow().editor));
            f.state.file_dialog.set(false);
            f.state.group_hidden.set(false);
            f.state.capture_active.set(false);
            f.state.group_locked.set(false);
            f.annotation_wheel(5, true);
            f.drain();
            assert_eq!(f.state.canvas.borrow().editor.revision(), 4);
        }
    }
}

#[test]
fn wheel_cannot_adopt_a_resize_worker_started_by_the_menu() {
    unsafe {
        let f = Fixture::blur();
        let revision = f.state.canvas.borrow().editor.revision();
        desktop::resize_input::uniform(f.window.0, &f.state, 1.1);
        assert!(f.state.resize_job.borrow().is_busy());
        f.annotation_wheel(120, false);
        f.annotation_wheel(120, true);
        f.drain();
        assert_eq!(f.state.canvas.borrow().editor.revision(), revision + 1);
        close(
            f.state.canvas.borrow().editor.elements()[2]
                .blur
                .as_ref()
                .unwrap()
                .strength(),
            4.4,
        );
    }
}

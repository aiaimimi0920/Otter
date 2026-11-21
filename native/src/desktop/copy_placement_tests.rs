//! 注入鼠标采样与最终剪贴板回执，其余复制 worker、缓存匹配、粘贴解码共用产品路径。
use super::*;
use crate::desktop::{clipboard_placement::Origin, open_job, paste_input};

#[test]
fn editable_art_and_minified_copies_keep_placement_without_preserving_flattened_edits() {
    unsafe {
        for mode in 0..3 {
            let fixture = Fixture::new();
            let mut source = state();
            source.canvas.borrow_mut().pinned = true;
            if mode == 1 {
                source.restored_art = true;
            } else if mode == 2 {
                let mut c = source.canvas.borrow_mut();
                let view = crate::session_minified::View {
                    size: crate::crop_geometry::Size {
                        width: 2.,
                        height: 2.,
                    },
                    offset: (0., 0.),
                };
                c.minified = Some(
                    crate::desktop::canvas::minified::Presentation::restored(
                        c.image.clone(),
                        crate::session_minified::Views {
                            image: view,
                            annotations: view,
                        },
                        view.size,
                        &c.editor,
                    )
                    .unwrap(),
                );
            }
            SetWindowPos(
                fixture.hwnd,
                None,
                -400,
                80,
                320,
                160,
                SWP_NOZORDER | SWP_NOACTIVATE,
            )
            .unwrap();
            let cursor = POINT { x: -350, y: 120 };
            let origin = Origin::capture_at(fixture.hwnd, &source, cursor).unwrap();
            let step = origin.step;
            assert_eq!(origin.offset, (50, 40));
            start_guarded_at(fixture.hwnd, &source, None, Some(40), Some(origin));
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
            while !source
                .copy_job
                .borrow()
                .as_ref()
                .unwrap()
                .worker
                .as_ref()
                .unwrap()
                .is_finished()
            {
                assert!(std::time::Instant::now() < deadline);
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            let mut png = None;
            poll_guarded(
                fixture.hwnd,
                &source,
                || 40,
                |_, payload, _, _| {
                    png = Some(payload.png.clone());
                    Ok(Some(41))
                },
            );
            let mut target = state();
            target.editable_clipboard = source.editable_clipboard.clone();
            let content = target.editable_clipboard.borrow_mut().matching(41);
            assert_eq!(content.is_some(), mode == 0);
            // 图内首次 +20 DIP；解码完成后只推进已提交锚点。
            let request = target
                .editable_clipboard
                .borrow()
                .position((cursor.x, cursor.y))
                .unwrap();
            let first = request.frame(None);
            assert_eq!(
                (first.x, first.y, first.width, first.height),
                (-400 + step, 80 + step, 320, 160)
            );
            for (point, expected) in [
                (cursor, (-400 + step, 80 + step)),
                (POINT { x: 1000, y: -100 }, (950, -140)),
            ] {
                let snapshot = content
                    .clone()
                    .map(paste_input::Snapshot::Editable)
                    .unwrap_or_else(|| paste_input::Snapshot::Png(png.clone().unwrap()));
                paste_input::deliver_at(fixture.hwnd, &target, snapshot, true, point).unwrap();
                open_job::finish(fixture.hwnd, &target, true);
                let prepared = target.new_pin.borrow_mut().take().unwrap().pop().unwrap();
                assert_eq!(prepared.editable.is_some(), mode == 0);
                let frame = prepared.placement.unwrap().frame(None);
                assert_eq!((frame.x, frame.y), expected);
                assert_eq!((frame.width, frame.height), (320, 160));
            }
            target
                .editable_clipboard
                .borrow_mut()
                .placed(&request, source.unit_id.clone(), first);
            assert_eq!(
                target
                    .editable_clipboard
                    .borrow()
                    .position((cursor.x, cursor.y))
                    .unwrap()
                    .frame(None)
                    .x,
                -400 + 2 * step
            );
            // 外部覆盖剪贴板必须清掉旧位置；不能沿用最后一次 Otter 复制的偏移。
            assert!(target
                .editable_clipboard
                .borrow_mut()
                .matching(42)
                .is_none());
            assert!(target
                .editable_clipboard
                .borrow()
                .position((0, 0))
                .is_none());
        }
    }
}

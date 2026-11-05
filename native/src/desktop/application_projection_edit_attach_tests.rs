//! 旧PNG首次接入冻结basis的原Application事务；独立源像素/历史和内容框oracle。
use super::*;

#[test]
fn collaboration_first_attach_requires_actual_matching_source_dimensions_pixels_and_history() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        // metadata-only、同版带PNG、新版但同digest，均不能跳过原本地源证明。
        for shape in 0..3 {
            for case in 0..4 {
                let mut fixture = Fixture::new();
                saved(&mut fixture);
                let mut response = wire(&fixture.origin, 1, false);
                if shape == 0 {
                    response["snapshot"] = Value::Null;
                } else if shape == 2 {
                    response["revision"] = json!(6);
                }
                if case == 1 {
                    response["width"] = json!(3);
                    response["editing"]["basis"]["width"] = json!(3);
                } else if case == 2 {
                    let mut canvas = state(&fixture).canvas.borrow_mut();
                    // 尺寸/Editor身份一致，但自然像素不同；不是合法的basis来源。
                    canvas.minified_source =
                        Some(crate::desktop::canvas::minified_source::Source::new(
                            crate::image_decode::Image {
                                width: 2,
                                height: 1,
                                pixels: vec![255; 8].into(),
                            },
                            &canvas.editor,
                        ));
                } else if case == 3 {
                    let mut canvas = state(&fixture).canvas.borrow_mut();
                    let old = canvas.editor.layers().unwrap().clone();
                    let erased = old
                        .erase(
                            crate::erase_layers::Mode::Content,
                            &[Point::default()],
                            4.0,
                            || false,
                        )
                        .unwrap();
                    let revision = canvas.editor.revision();
                    assert!(canvas
                        .editor
                        .replace_layers(revision, &old, erased)
                        .unwrap());
                    assert!(canvas.editor.undo());
                    // 当前像素已回原图，但redo中仍有破坏性底图，不能静默建立新权威。
                    assert!(canvas.editor.layers().unwrap().base.same_storage(&old.base));
                    assert!(!canvas.editor.source_replaceable());
                }
                let before = source(&fixture);
                let revision = state(&fixture).canvas.borrow().editor.revision();
                let base = state(&fixture)
                    .canvas
                    .borrow()
                    .editor
                    .layers()
                    .unwrap()
                    .base
                    .clone();
                let rx = peer(&mut fixture, vec![response]);
                cycle(&mut fixture, &rx);
                joined(&mut fixture);
                if case != 0 || shape != 2 {
                    assert!(
                        source(&fixture).same_storage(&before),
                        "shape {shape}, case {case}"
                    );
                }
                assert!(state(&fixture)
                    .canvas
                    .borrow()
                    .editor
                    .layers()
                    .unwrap()
                    .base
                    .same_storage(&base));
                if case == 0 {
                    assert_eq!(snapshot(&fixture).document().revision(), 1);
                    assert_eq!(
                        binding(&fixture)["revision"],
                        if shape == 2 { json!(6) } else { json!(5) }
                    );
                    assert_eq!(
                        state(&fixture).canvas.borrow().editor.elements()[0].annotation_id(),
                        "remote"
                    );
                } else {
                    assert!(
                        state(&fixture).projection_edit.borrow().is_none(),
                        "shape {shape}, case {case}"
                    );
                    assert_eq!(
                        state(&fixture).canvas.borrow().editor.revision(),
                        revision,
                        "shape {shape}, case {case}"
                    );
                    if case == 3 {
                        assert!(
                            state(&fixture).canvas.borrow_mut().editor.redo(),
                            "destructive redo must survive rejected attach"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn collaboration_plain_receiver_can_adopt_frozen_new_basis_and_vectors_in_one_transaction() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = Fixture::new();
        saved(&mut fixture);
        let (frame, local) = {
            let mut canvas = state(&fixture).canvas.borrow_mut();
            let frame = canvas.editor.full_logical_frame().unwrap();
            canvas
                .editor
                .begin(crate::editor::Tool::Brush, Point { x: 20.0, y: 20.0 })
                .unwrap();
            assert!(canvas.editor.commit());
            (
                frame,
                canvas.editor.elements()[0].annotation_id().to_owned(),
            )
        };
        let mut response = sync_fixture::changed(&fixture.origin);
        response["editing"] =
            crate::projection_delivery::accept::edit_tests::document(&response, 1);
        let expected = response["editing"].clone();
        let rx = peer(&mut fixture, vec![response]);
        cycle(&mut fixture, &rx);
        joined(&mut fixture);
        assert_eq!(
            serde_json::to_value(snapshot(&fixture).document()).unwrap(),
            expected
        );
        assert_eq!(binding(&fixture)["revision"], 6);
        assert_eq!(state(&fixture).canvas.borrow().editor.elements().len(), 2);
        let received = snapshot(&fixture);
        let view = received.editor_view().unwrap();
        assert!(view.basis_objects().contains_key(&local));
        assert_eq!(received.conflict(), Some(edit::Code::ReadOnly));
        // 独立fit oracle；矢量必须随3:1内容居中，而不是拉伸到旧2:1逻辑框。
        let scale = (frame.width / 3.0).min(frame.height);
        let x = frame.x + (frame.width - 3.0 * scale) / 2.0;
        let y = frame.y + (frame.height - scale) / 2.0;
        assert_eq!(view.size().height(), scale);
        {
            let canvas = state(&fixture).canvas.borrow();
            let remote = canvas
                .editor
                .elements()
                .iter()
                .find(|v| v.annotation_id() == "remote")
                .unwrap();
            for (actual, expected) in [
                (remote.points[0].x, x),
                (remote.points[0].y, y),
                (remote.points[1].x, x + scale),
                (remote.points[1].y, y + scale),
            ] {
                assert!(
                    (actual as f64 - expected).abs() <= 0.0001,
                    "{actual} != {expected}"
                );
            }
        }
        assert_eq!(
            &*state(&fixture)
                .canvas
                .borrow()
                .minified_source
                .as_ref()
                .unwrap()
                .image()
                .pixels,
            &[255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255]
        );
        save(&mut fixture);
        super::super::super::fixture::reopen_workspace(&mut fixture);
        assert_eq!(snapshot(&fixture).document().basis().size(), (3, 1));
        assert_eq!(
            snapshot(&fixture).editor_view().unwrap().basis_objects(),
            view.basis_objects()
        );
    }
}

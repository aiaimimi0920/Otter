//! 输入资源来源凭证、内联边界与原始底图/正式输出的准备合同。
use super::*;
use crate::desktop::{loom_output_batch, loom_result_state::Results};
#[test]
fn shader_inline_png_input_preserves_exact_bytes_hidden_rgb_and_existing_budget() {
    let staging = crate::desktop::shader_prepare::tests::png;
    let rgba = [253, 137, 79, 1, 83, 97, 109, 0];
    let png = staging(2, 1, &rgba);
    let encoded = crate::desktop::shader_prepare::tests::encoded(&png);
    let uri = format!("data:image/png;base64,{encoded}");
    let mut actual = 0;
    let prepared = Data::inline(&uri)
        .unwrap()
        .prepare(&AtomicBool::new(false), &mut actual)
        .unwrap();
    assert_eq!(
        actual,
        rgba.len() + png.len(),
        "count display pixels and retained PNG"
    );
    let crate::loom_art_wire::Input::InlinePng {
        base64,
        width,
        height,
    } = prepared.descriptor()
    else {
        panic!("inline PNG")
    };
    assert_eq!((width, height), (2, 1));
    let bytes = crate::desktop::loom_output_inline::decode_bytes(
        "image/png",
        base64,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(&*bytes, &*png, "the worker must not reencode PBGRA");
    assert_eq!(
        &*crate::desktop::shader_prepare::tests::decode_png(base64).rgba,
        &rgba
    );
    let mut actual = 256 * 1024 * 1024 - rgba.len() - png.len() + 1;
    let before = actual;
    assert!(matches!(
        Data::inline(&uri)
            .unwrap()
            .prepare(&AtomicBool::new(false), &mut actual),
        Err(Error::Budget)
    ));
    assert_eq!(actual, before, "failed budget admission is atomic");
}
#[test]
fn shader_own_base_preparation_omits_overlay_and_crop_but_retains_real_document_stamp() {
    let image = crate::desktop::image::Image {
        width: 2,
        height: 1,
        pixels: vec![10, 20, 30, 255, 40, 50, 60, 255].into(),
    };
    let mut editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (2, 1),
        base: image.pixels.clone(),
        annotations: Some(vec![255, 0, 0, 255, 255, 0, 0, 255].into()),
        annotation_size: Some((2, 1)),
    })
    .unwrap();
    editor
        .crop_to(crate::crop_geometry::Rect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        })
        .unwrap();
    let target = UnitId::parse("own").unwrap();
    let input = Source::capture_base("input".into(), target.clone(), &editor).unwrap();
    let batch = prepare(
        target,
        editor.document_stamp(),
        vec![input],
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(batch.is_current(|_| Some(&editor)));
    let replacement = Editor::default();
    assert!(!batch.is_current(|_| Some(&replacement)));
    let (
        _,
        crate::loom_art_wire::Input::InlinePng {
            base64,
            width,
            height,
        },
    ) = batch.descriptors().next().unwrap()
    else {
        panic!("PNG")
    };
    assert_eq!((width, height), (2, 1));
    let decoded = crate::desktop::loom_output_inline::decode_base64(
        "image/png",
        base64,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(&*decoded.pixels, &*image.pixels);
    editor
        .begin(
            crate::editor::Tool::Brush,
            crate::viewport::Point { x: 0.0, y: 0.0 },
        )
        .unwrap();
    assert!(Source::capture_base("input".into(), UnitId::parse("own").unwrap(), &editor).is_err());
}
#[test]
fn inline_capture_rejects_external_references_and_worker_rejects_invalid_or_cancelled_data() {
    let unit = UnitId::parse("inline").unwrap();
    let editor = Editor::default();
    for uri in [
        "https://example.test/image.png",
        "file:///image.png",
        "data:text/plain;base64,YQ==",
        "data:image/png;base64,",
    ] {
        assert!(Source::capture_inline("image".into(), unit.clone(), uri, &editor).is_err());
    }
    let source = || {
        Source::capture_inline(
            "image".into(),
            unit.clone(),
            "data:image/png;base64,invalid",
            &editor,
        )
        .unwrap()
    };
    assert!(matches!(
        prepare(
            unit.clone(),
            editor.document_stamp(),
            vec![source()],
            &AtomicBool::new(false)
        ),
        Err(Error::InvalidInput)
    ));
    assert!(matches!(
        prepare(
            unit.clone(),
            editor.document_stamp(),
            vec![source()],
            &AtomicBool::new(true)
        ),
        Err(Error::Cancelled)
    ));
}
#[test]
fn parameter_only_batch_retains_formal_output_dependency_without_images() {
    let target = UnitId::parse("target").unwrap();
    let source = UnitId::parse("source").unwrap();
    let editor = Editor::default();
    let mut results = Results::new(source.clone());
    let mut batch = prepare(
        target.clone(),
        editor.document_stamp(),
        Vec::new(),
        &AtomicBool::new(false),
    )
    .unwrap();
    batch
        .set_dependencies(vec![(
            source.clone(),
            SourceStamp::Output(results.output_stamp()),
        )])
        .unwrap();
    assert_eq!(batch.descriptors().len(), 0);
    assert!(!batch.is_current(|_| Some(&editor)));
    let freshness = batch.freshness().unwrap();
    let current = |results: &Results| {
        freshness.is_current(|unit, stamp| match stamp {
            SourceStamp::Document(stamp) => *unit == target && stamp.matches(&editor),
            SourceStamp::Output(stamp) => *unit == source && stamp.matches(results),
            SourceStamp::Node { .. } | SourceStamp::Live(_) => false,
        })
    };
    assert!(current(&results));
    let request = crate::graph_requests::Request {
        id: UnitId::parse("scalar-result").unwrap(),
        generation: 1,
    };
    results.begin(request.clone());
    let scalar = loom_output_batch::decode(
        &serde_json::json!({"output":{"kind":"value","value":3}}),
        None,
        &AtomicBool::new(false),
    )
    .unwrap();
    results.finish(&request, scalar).unwrap();
    assert!(!current(&results));
}
fn finish(results: &mut Results, generation: u64, color: u8) {
    let request = crate::graph_requests::Request {
        id: UnitId::parse("result").unwrap(),
        generation,
    };
    results.begin(request.clone());
    let inline = loom_inline::prepare(
        1,
        1,
        vec![color, 0, 0, 255].into(),
        &Editor::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    let crate::loom_art_wire::Input::InlinePng { base64, .. } = inline.descriptor() else {
        panic!("inline");
    };
    let batch = loom_output_batch::decode(
        &serde_json::json!({"output_image":{
            "kind":"inline_resource","mime":"image/png","dataBase64":base64
        }}),
        None,
        &AtomicBool::new(false),
    )
    .unwrap();
    results.finish(&request, batch).unwrap();
}
#[test]
fn prepared_formal_source_retains_output_stamp_and_rejects_editor_only_validation() {
    let target = UnitId::parse("target").unwrap();
    let source = UnitId::parse("source").unwrap();
    let editor = Editor::default();
    let mut results = Results::new(source.clone());
    finish(&mut results, 1, 20);
    let input = Source::capture_output(
        "image".into(),
        source.clone(),
        results.image_source("output_image").unwrap(),
    )
    .unwrap();
    let batch = prepare(
        target.clone(),
        editor.document_stamp(),
        vec![input],
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(!batch.is_current(|_| Some(&editor)));
    let freshness = batch.freshness().unwrap();
    let current = |results: &Results| {
        freshness.is_current(|unit, stamp| match stamp {
            SourceStamp::Document(stamp) => *unit == target && stamp.matches(&editor),
            SourceStamp::Output(stamp) => *unit == source && stamp.matches(results),
            SourceStamp::Node { .. } | SourceStamp::Live(_) => false,
        })
    };
    assert!(current(&results));
    finish(&mut results, 2, 40);
    assert!(!current(&results));
    let (
        _,
        crate::loom_art_wire::Input::InlinePng {
            base64,
            width,
            height,
        },
    ) = batch.descriptors().next().unwrap()
    else {
        panic!("inline");
    };
    assert_eq!((width, height), (1, 1));
    let image = crate::desktop::loom_output_inline::decode(
        &serde_json::json!({"kind":"inline_resource",
        "mime":"image/png","dataBase64":base64}),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(&*image.pixels, &[20, 0, 0, 255]);
}

#[test]
fn prepared_relay_dependency_tracks_capability_selection_and_parameter_revision() {
    use crate::desktop::{capability_job::Loaded, capability_selection::Selection, State};
    let state = State::new(None, false).unwrap();
    let (_, identity, _) = crate::loom_bridge_client::tests::fixture();
    let handshake = crate::loom_handshake::response(
        &serde_json::to_vec(&crate::loom_handshake::tests::fixture()).unwrap(),
    )
    .unwrap()
    .unwrap();
    let raw = serde_json::json!({"id":"test-art","parameters":[{"id":"amount","default":1}]});
    let parameters = crate::loom_parameter_state::Parameters::new(
        &crate::loom_capability::Capability::new(&raw).unwrap(),
    )
    .unwrap();
    let catalog = std::rc::Rc::new(Loaded {
        identity,
        handshake,
    });
    let capture = || {
        let mut input = Source::capture(
            "image".into(),
            state.unit_id.clone(),
            1,
            1,
            vec![1, 2, 3, 255].into(),
            &state.canvas.borrow().editor,
        )
        .unwrap();
        input.depend_on(&state).unwrap();
        prepare(
            state.unit_id.clone(),
            state.canvas.borrow().editor.document_stamp(),
            vec![input],
            &AtomicBool::new(false),
        )
        .unwrap()
    };
    let batch = capture();
    let freshness = batch.freshness().unwrap();
    assert!(freshness.is_current(|_, stamp| stamp.matches_state(&state)));
    {
        let canvas = state.canvas.borrow();
        assert!(!batch.is_current(|_| Some(&canvas.editor)));
    }
    *state.selected_art.borrow_mut() = Some(Selection {
        execution: Default::default(),
        identity: crate::desktop::document_state::new_unit_id().unwrap(),
        catalog,
        parameters,
    });
    assert!(!freshness.is_current(|_, stamp| stamp.matches_state(&state)));
    let selected = capture().freshness().unwrap();
    state
        .selected_art
        .borrow_mut()
        .as_mut()
        .unwrap()
        .parameters
        .set("amount", Some(serde_json::json!(1)))
        .unwrap();
    assert!(selected.is_current(|_, stamp| stamp.matches_state(&state)));
    state
        .selected_art
        .borrow_mut()
        .as_mut()
        .unwrap()
        .parameters
        .set("amount", Some(serde_json::json!(2)))
        .unwrap();
    assert!(!selected.is_current(|_, stamp| stamp.matches_state(&state)));
    let edited = capture().freshness().unwrap();
    state.selected_art.borrow_mut().as_mut().unwrap().identity =
        crate::desktop::document_state::new_unit_id().unwrap();
    assert!(!edited.is_current(|_, stamp| stamp.matches_state(&state)));
    let reselected = capture().freshness().unwrap();
    state.selected_art.borrow_mut().take();
    assert!(!reselected.is_current(|_, stamp| stamp.matches_state(&state)));
}

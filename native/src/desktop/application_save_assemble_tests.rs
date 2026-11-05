use super::*;
use crate::session_source::Source;
use crate::{
    editor::Editor, pin_geometry::PinRect, session_placement::Space, workspace_id::UnitId,
};
fn id(value: &str) -> UnitId {
    UnitId::parse(value).unwrap()
}
pub(super) fn doc(unit: &str) -> Document {
    Document {
        output_images: Vec::new(),
        outputs: None,
        capture_original: None,
        capture_meta: None,
        unit: id(unit),
        editor: Editor::default(),
        frame: PinRect {
            x: 10,
            y: 20,
            width: 40,
            height: 30,
        },
        viewport: Default::default(),
        dpi: 96,
        minified: None,
        transition: None,
        normal_opacity: 1.0,
        mini_opacity: 0.9,
        pinned: false,
        art: false,
        local_art_id: None,
        local_art_ports: None,
        group: None,
        parameters: None,
        sticker: None,
        execution: None,
        restored_source: None,
        local_image: None,
    }
}
pub(super) fn data(documents: Vec<Document>) -> Data {
    Data {
        port_visibility: Vec::new(),
        projection_edits: Vec::new(),
        disk_revision: None,
        projection_scope: crate::session_projection_scope::Scope::generate().unwrap(),
        live: Vec::new(),
        original: Source::from(br#"{"stickers":[],"links":[]}"#.to_vec()),
        space: Space {
            origin: (0, 0),
            scale: 1.0,
        },
        documents,
        links: vec![],
        groups: vec![],
        transient_host: None,
    }
}
fn plain(_: Document, fields: Map<String, Value>) -> Result<WindowCandidate, Error> {
    Ok(WindowCandidate::new(fields))
}
#[test]
fn current_output_snapshot_replaces_history_and_empty_map_clears_it() {
    for outputs in [
        None,
        Some(Map::new()),
        Some(
            serde_json::from_value(serde_json::json!({
                "fresh": false, "resource": {"uri":"opaque:test"}
            }))
            .unwrap(),
        ),
    ] {
        let mut document = doc("node:test");
        document.outputs = outputs.clone();
        let mut snapshot = data(vec![document]);
        if outputs.is_some() {
            snapshot.transient_host = Some(id("node:test"));
        }
        snapshot.original = Source::from(br#"{"stickers":[{"id":"node:test","x":0,"y":0,"w":40,"h":30,"outputs":{"old":1}}],"links":[]}"#.to_vec());
        let candidate = snapshot.assemble(&AtomicBool::new(false), plain).unwrap();
        let saved: Value = serde_json::from_slice(&candidate.bytes).unwrap();
        assert_eq!(
            saved["stickers"][0]["outputs"],
            outputs
                .map(Value::Object)
                .unwrap_or(serde_json::json!({"old":1}))
        );
    }
}
#[test]
fn session_save_embeds_exact_hdr_original_and_non_capture_replacement_removes_it() {
    struct FileGuard(std::path::PathBuf);
    impl Drop for FileGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let path = FileGuard(
        std::path::PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi")
            .join(format!("otter-session-original-{:?}.png", unsafe {
                windows::Win32::System::Com::CoCreateGuid().unwrap()
            })),
    );
    let cancel = AtomicBool::new(false);
    let pixels: Vec<_> = [0x4000u16; 4]
        .iter()
        .flat_map(|n| n.to_le_bytes())
        .collect();
    let frame = crate::capture_color::ScRgb::new(&pixels, 1, 1, 8)
        .unwrap()
        .to_hdr(0.01, 1000.0, 1.0, &cancel)
        .unwrap();
    crate::capture_file::PreparedHdr::prepare(&path.0, &frame, &cancel)
        .unwrap()
        .commit_new(&cancel)
        .unwrap();
    let expected =
        crate::loom_base64::encode_png_data_url(&std::fs::read(&path.0).unwrap(), &cancel).unwrap();
    let mut document = doc("s");
    document.capture_meta =
        Some(serde_json::json!({"dynamicRange":"hdr","bitDepth":16,"colorSpace":"bt2020-pq"}));
    document.capture_original = Some(std::sync::Arc::new(std::fs::File::open(&path.0).unwrap()));
    let candidate = data(vec![document]).encode(&cancel).unwrap();
    let saved: Value = serde_json::from_slice(&candidate.bytes).unwrap();
    assert_eq!(
        saved["stickers"][0]["otterNativeCaptureOriginalSrc"],
        expected.as_str()
    );
    let mut replacement = data(vec![doc("s")]);
    replacement.projection_scope = crate::session_document::decode(&candidate.bytes)
        .unwrap()
        .projection_scope
        .unwrap();
    replacement.original = Source::from(candidate.bytes.to_vec());
    let candidate = replacement.assemble(&cancel, plain).unwrap();
    let saved: Value = serde_json::from_slice(&candidate.bytes).unwrap();
    assert!(saved["stickers"][0]
        .get("otterNativeCaptureOriginalSrc")
        .is_none());
}
#[test]
fn capture_metadata_is_saved_and_replaced_source_clears_old_metadata() {
    let cancel = AtomicBool::new(false);
    let meta = serde_json::json!({"dynamicRange":"hdr", "bitDepth":16, "colorSpace":"bt2020-pq", "extension":{"keep":true}});
    let mut captured = doc("s");
    captured.capture_meta = Some(meta.clone());
    let candidate = data(vec![captured]).assemble(&cancel, plain).unwrap();
    let saved: Value = serde_json::from_slice(&candidate.bytes).unwrap();
    assert_eq!(saved["stickers"][0]["captureMeta"], meta);
    let mut replacement = data(vec![doc("s")]);
    replacement.projection_scope = crate::session_document::decode(&candidate.bytes)
        .unwrap()
        .projection_scope
        .unwrap();
    replacement.original = Source::from(candidate.bytes.to_vec());
    let candidate = replacement.assemble(&cancel, plain).unwrap();
    let saved: Value = serde_json::from_slice(&candidate.bytes).unwrap();
    assert!(saved["stickers"][0].get("captureMeta").is_none());
}
#[test]
fn capture_metadata_snapshot_rejects_invalid_range_depth_before_save_candidate() {
    let mut captured = doc("s");
    captured.capture_meta = Some(serde_json::json!({"dynamicRange":"hdr", "bitDepth":8}));
    assert!(data(vec![captured])
        .assemble(&AtomicBool::new(false), plain)
        .is_err());
}
#[test]
fn current_snapshot_not_original_topology_drives_add_remove_and_groups() {
    let mut snapshot = data(vec![doc("retained"), doc("new")]);
    snapshot.original = Source::from(br#"{"stickers":[{"id":"removed","x":0,"y":0,"w":10,"h":10},{"id":"retained","x":0,"y":0,"w":10,"h":10,"extension":123}],"links":[]}"#.to_vec());
    snapshot.groups.push(crate::workspace::Group {
        id: id("group"),
        name: "分组".into(),
        hidden: true,
        locked: false,
    });
    snapshot.documents[1].group = Some(id("group"));
    snapshot.links.push(crate::workspace::Link {
        id: id("link"),
        from_unit: id("retained"),
        from_port: "out".into(),
        to_unit: id("new"),
        to_port: "in".into(),
    });
    let candidate = snapshot.assemble(&AtomicBool::new(false), plain).unwrap();
    let read = crate::session_document::decode(&candidate.bytes).unwrap();
    assert_eq!(
        read.windows
            .iter()
            .map(|w| w.id.as_str())
            .collect::<Vec<_>>(),
        ["retained", "new"]
    );
    assert_eq!(read.windows[1].group_id.as_deref(), Some("group"));
    assert_eq!(read.workspace.links().len(), 1);
    assert!(read.workspace.groups()[0].hidden);
    let value: Value = serde_json::from_slice(&candidate.bytes).unwrap();
    assert_eq!(value["stickers"][0]["extension"], 123);
    assert_eq!(read.windows[0].x, 10.0);
}
#[test]
fn empty_transient_is_omitted_but_edits_and_relationships_promote_it() {
    for mode in 0..4 {
        let mut snapshot = data(vec![doc("host")]);
        snapshot.transient_host = Some(id("host"));
        match mode {
            1 => {
                snapshot.documents[0]
                    .editor
                    .begin(crate::editor::Tool::Brush, Default::default())
                    .unwrap();
                assert!(snapshot.documents[0].editor.commit());
            }
            2 => snapshot.documents[0].pinned = true,
            3 => {
                snapshot.documents.push(doc("other"));
                snapshot.links.push(crate::workspace::Link {
                    id: id("link"),
                    from_unit: id("host"),
                    from_port: "out".into(),
                    to_unit: id("other"),
                    to_port: "in".into(),
                });
            }
            _ => {}
        }
        let candidate = snapshot.assemble(&AtomicBool::new(false), plain).unwrap();
        let read = crate::session_document::decode(&candidate.bytes).unwrap();
        assert_eq!(read.windows.iter().any(|w| w.id == "host"), mode != 0);
    }
}
#[test]
fn duplicate_identity_and_cancel_never_call_content_encoder() {
    assert!(matches!(
        data(vec![doc("same"), doc("same")])
            .assemble(&AtomicBool::new(false), |_, _| panic!("duplicate")),
        Err(Error::Topology)
    ));
    assert!(matches!(
        data(vec![doc("one")]).assemble(&AtomicBool::new(true), |_, _| panic!("cancelled")),
        Err(Error::Cancelled)
    ));
}

#[test]
fn snapshot_annotation_writer_uses_current_editor_and_real_font_baseline() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let mut document = doc("text");
    let text =
        crate::text_annotation::TextAnnotation::new("中文保存", "Arial", 18.0, false).unwrap();
    let size = unsafe { crate::desktop::annotation_text::measure(&text) }.unwrap();
    document
        .editor
        .insert_text(crate::viewport::Point { x: 10.0, y: 20.0 }, size, text)
        .unwrap();
    document.viewport.scale = 2.0;
    // 默认视口还有 (20,20) 平移：保存点应为 (10,20)*2+(20,20)，不是仅乘比例。
    let old = serde_json::json!({"annotationState":{"extension":123,"elements":[]}});
    let fields = document
        .write_annotations(
            old.as_object().unwrap().clone(),
            Space {
                origin: (-100, -200),
                scale: 1.0,
            },
            &AtomicBool::new(false),
        )
        .unwrap();
    assert_eq!(fields["annotationState"]["extension"], 123);
    let raw = serde_json::value::to_raw_value(&fields["annotationState"]).unwrap();
    let read = crate::session_vectors::restore_with_text(Some(&raw), |text| {
        Ok(crate::session_vectors::TextMetrics {
            size: unsafe { crate::desktop::annotation_text::measure(text) }.unwrap(),
            baseline: crate::desktop::annotation_text::measure_flip_metrics(text)
                .unwrap()
                .baseline_from_top,
        })
    })
    .unwrap();
    assert_eq!(read.editor.elements().len(), 1);
    assert_eq!(
        read.editor.elements()[0].points[0],
        crate::viewport::Point { x: 40.0, y: 60.0 }
    );
    assert_eq!(
        read.editor.elements()[0].text.as_ref().unwrap().font_size(),
        36.0
    );
    assert_eq!(
        read.editor.elements()[0].annotation_id(),
        document.editor.elements()[0].annotation_id()
    );
}

#[test]
fn annotation_writer_rejects_cancel_and_minified_context_without_mutation() {
    let mut document = doc("one");
    let space = Space {
        origin: (0, 0),
        scale: 1.0,
    };
    let before = document.editor.document_stamp();
    assert!(matches!(
        document.write_annotations(Map::new(), space, &AtomicBool::new(true)),
        Err(Error::Cancelled)
    ));
    document.transition = Some(crate::minified_transition::Transition {
        initial: document.frame,
        expanded: document.frame,
        opacity: 1.0,
        persist_expanded: true,
    });
    assert!(matches!(
        document.write_annotations(Map::new(), space, &AtomicBool::new(false)),
        Err(Error::Window)
    ));
    assert!(before.matches(&document.editor));
}

#[test]
fn assembly_writes_current_crop_and_reset_instead_of_retaining_old_state() {
    let mut editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (8, 4),
        base: vec![255; 128].into(),
        annotation_size: None,
        annotations: None,
    })
    .unwrap();
    editor
        .crop_to(crate::crop_geometry::Rect {
            x: 1.25,
            y: 0.5,
            width: 4.5,
            height: 2.5,
        })
        .unwrap();
    for reset in [false, true] {
        if reset {
            assert!(editor.reset_crop());
        }
        let mut document = doc("one");
        document.editor = editor.export_snapshot();
        let mut snapshot = data(vec![document]);
        snapshot.original = Source::from(br#"{"stickers":[{"id":"one","x":0,"y":0,"w":40,"h":30,"imageEditState":{"cropRect":{"x":9,"y":8,"w":7,"h":6},"sourceSize":{"w":100,"h":100},"extension":123}}],"links":[]}"#.to_vec());
        let output = snapshot.assemble(&AtomicBool::new(false), plain).unwrap();
        let read = crate::session_document::decode(&output.bytes).unwrap();
        let crop = crate::session_crop::restore(&read.windows[0]).unwrap();
        assert_eq!(
            crop.map(|value| value.crop),
            editor.crop_view().map(|view| view.source)
        );
        let fields: Value =
            serde_json::from_str(read.windows[0].image_edit_state.unwrap().get()).unwrap();
        assert_eq!(fields["extension"], 123);
    }
}

#[test]
fn minified_annotation_write_keeps_expanded_coordinates_and_single_view_offset() {
    let mut document = doc("mini");
    document.editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (100, 50),
        base: vec![255; 100 * 50 * 4].into(),
        annotation_size: None,
        annotations: None,
    })
    .unwrap();
    document
        .editor
        .begin(
            crate::editor::Tool::Rectangle,
            crate::viewport::Point { x: 10.0, y: 20.0 },
        )
        .unwrap();
    document
        .editor
        .extend(crate::viewport::Point { x: 30.0, y: 40.0 })
        .unwrap();
    assert!(document.editor.commit());
    let view = crate::session_minified::View {
        size: crate::crop_geometry::Size {
            width: 200.0,
            height: 100.0,
        },
        offset: (30.0, 40.0),
    };
    document.minified = Some((
        crate::session_minified::Views {
            image: view,
            annotations: view,
        },
        crate::crop_geometry::Size {
            width: 50.0,
            height: 50.0,
        },
    ));
    document.frame = PinRect {
        x: 40,
        y: 60,
        width: 50,
        height: 50,
    };
    document.transition = Some(crate::minified_transition::Transition {
        initial: document.frame,
        expanded: PinRect {
            x: 10,
            y: 20,
            width: 200,
            height: 100,
        },
        opacity: 1.0,
        persist_expanded: true,
    });
    let cancel = AtomicBool::new(false);
    let snapshot = data(vec![document]);
    let space = snapshot.space;
    let saved = snapshot
        .assemble(&cancel, |document, fields| {
            Ok(WindowCandidate::new(
                document.write_annotations(fields, space, &cancel)?,
            ))
        })
        .unwrap();
    let read = crate::session_document::decode(&saved.bytes).unwrap();
    let window = &read.windows[0];
    let annotations = crate::session_vectors::restore(window.annotation_state).unwrap();
    let point = annotations.editor.elements()[0].points[0];
    assert_eq!(point, crate::viewport::Point { x: 20.0, y: 40.0 });
    let views = crate::session_minified::restore(window, None)
        .unwrap()
        .unwrap();
    assert_eq!(views.annotations, view);
    assert_eq!(
        (
            point.x as f64 - view.offset.0,
            point.y as f64 - view.offset.1
        ),
        (-10.0, 0.0)
    );
}

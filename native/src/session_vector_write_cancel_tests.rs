use super::*;
use std::cell::Cell;

#[test]
fn cancellation_at_multiple_stages_discards_output_and_keeps_editor() {
    let mut editor = Editor::default();
    editor
        .append_restored(
            Tool::Brush,
            (0..512)
                .map(|index| Point {
                    x: index as f32,
                    y: index as f32 / 2.0,
                })
                .collect(),
            0.0,
            vec![],
        )
        .unwrap();
    let stamp = editor.document_stamp();
    let mapping = Mapping::from_viewport(
        crate::viewport::Viewport::default(),
        96,
        crate::session_placement::Space {
            origin: (0, 0),
            scale: 1.0,
        },
    )
    .unwrap();
    let calls = Cell::new(0);
    let complete = encode_mapped_cancellable(
        &editor,
        None,
        mapping,
        |_| unreachable!(),
        || {
            calls.set(calls.get() + 1);
            false
        },
    )
    .unwrap();
    let total = calls.get();
    assert!(
        total > 1536,
        "point traversal and serialization must observe cancellation"
    );
    for threshold in [0, total / 4, total / 2, total - 1] {
        calls.set(0);
        assert_eq!(
            encode_mapped_cancellable(
                &editor,
                None,
                mapping,
                |_| unreachable!(),
                || {
                    let count = calls.get();
                    calls.set(count + 1);
                    count >= threshold
                }
            ),
            Err(Error::Cancelled)
        );
        assert!(stamp.matches(&editor));
    }
    assert_eq!(
        encode_mapped(&editor, None, mapping, |_| unreachable!()).unwrap(),
        complete
    );
}

#[test]
fn output_stream_stops_before_append_when_cancelled() {
    use std::io::Write;
    let cancel = Cell::new(false);
    let mut output = Bounded(Vec::new());
    {
        let check = || cancel.get();
        let mut sink = Cancellable {
            output: &mut output,
            cancelled: &check,
        };
        sink.write_all(b"before").unwrap();
        cancel.set(true);
        assert!(sink.write_all(b"after").is_err());
    }
    assert_eq!(output.0, b"before");
}

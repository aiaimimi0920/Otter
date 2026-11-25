use super::*;
#[test]
fn fixed_edge_conversion_preserves_export_reference_and_original_session() {
    let cancel = AtomicBool::new(false);
    let mut session = Session::new(Axis::Vertical, frame(0, true), options()).unwrap();
    session.append(frame(9, true), &cancel).unwrap();
    assert!(matches!(
        session.to_spatial(&cancel),
        Err(Error::UnconfirmedEdges)
    ));
    assert_eq!(session.frame_count(), 2);
    session.append(frame(18, true), &cancel).unwrap();
    let original = session.compose(&cancel).unwrap();
    let mut spatial = session.to_spatial(&cancel).unwrap();
    assert_eq!(spatial.frame_count(), 3);
    assert_eq!(spatial.position(), (0, 11));
    let result = spatial.compose(&cancel).unwrap();
    assert_eq!(
        (result.x, result.y, result.width, result.height),
        (0, -7, 20, 82)
    );
    assert_eq!(&*result.pixels, &*original.pixels);
    assert_eq!(
        spatial.append(frame(18, true), (0, 0), &cancel),
        Ok(SpatialAppend::Duplicate)
    );
    assert_eq!(
        &*session.compose(&cancel).unwrap().pixels,
        &*original.pixels
    );
    assert!(matches!(
        session.to_spatial(&AtomicBool::new(true)),
        Err(Error::Cancelled)
    ));
}
#[test]
fn aggregate_seed_can_exceed_capture_frame_axis_limit_without_becoming_match_reference() {
    let make = |origin: u32| {
        Frame::new(
            1,
            32768,
            (origin..origin + 32768)
                .flat_map(u32::to_le_bytes)
                .collect::<Vec<_>>()
                .into(),
        )
        .unwrap()
    };
    let cancel = AtomicBool::new(false);
    let mut session = Session::new(
        Axis::Vertical,
        make(0),
        MatchOptions {
            minimum_overlap: 32767,
            maximum_shift: 1,
            maximum_mean_error: 0.0,
        },
    )
    .unwrap();
    session.append(make(1), &cancel).unwrap();
    let mut spatial = session.to_spatial(&cancel).unwrap();
    assert_eq!(spatial.compose(&cancel).unwrap().height, 32769);
    assert_eq!(
        spatial.append(make(2), (0, 1), &cancel),
        Ok(SpatialAppend::Added { x: 0, y: 2 })
    );
    assert_eq!(spatial.compose(&cancel).unwrap().height, 32770);
}
#[test]
fn output_headroom_failure_and_cancellation_preserve_session_and_allow_retry() {
    let cancel = AtomicBool::new(false);
    let mut session = Session::new(Axis::Vertical, frame(0, false), options()).unwrap();
    session.append(frame(9, false), &cancel).unwrap();
    let previous = session.compose(&cancel).unwrap();
    let before = (
        session.total,
        session.minimum,
        session.maximum,
        session.edges,
        session.confirmations,
    );
    let bytes = 20 * 82 * 4;
    let budget = crate::byte_budget::Budget::new(bytes);
    let occupied = budget.reserve(1).unwrap();
    assert_eq!(
        session.append_with_reserve(frame(18, false), &cancel, |requested| {
            assert_eq!(requested, bytes);
            budget.reserve(requested)
        }),
        Err(Error::Budget)
    );
    assert_eq!(budget.used(), 1);
    drop(occupied);
    assert_eq!(
        session.append_with_reserve(frame(18, false), &cancel, |requested| {
            let lease = budget.reserve(requested);
            cancel.store(true, Ordering::Release);
            lease
        }),
        Err(Error::Cancelled)
    );
    assert_eq!(budget.used(), 0);
    cancel.store(false, Ordering::Release);
    assert_eq!(session.frame_count(), 2);
    assert_eq!(
        (
            session.total,
            session.minimum,
            session.maximum,
            session.edges,
            session.confirmations
        ),
        before
    );
    assert_eq!(
        &*session.compose(&cancel).unwrap().pixels,
        &*previous.pixels
    );
    assert_eq!(
        &*session.preview(&cancel).unwrap().pixels,
        &*previous.pixels
    );
    assert_eq!(
        session.append_with_reserve(frame(18, false), &cancel, |requested| budget
            .reserve(requested)),
        Ok(Append::Added { origin: 18 })
    );
    assert_eq!(budget.used(), 0);
    assert_eq!(session.compose(&cancel).unwrap().height, 82);
}
#[test]
fn scroll_admission_checks_input_and_frame_limits_before_an_action() {
    let mut session = Session::new(Axis::Vertical, frame(0, false), options()).unwrap();
    assert_eq!(session.admit_next(), Ok(()));
    session.total = MAX_INPUT_PIXELS;
    assert_eq!(session.admit_next(), Err(Error::InputLimit));
    let first = session.frames[0].frame.clone();
    session.frames = (0..MAX_FRAMES)
        .map(|_| Positioned {
            frame: first.clone(),
            origin: 0,
        })
        .collect();
    assert_eq!(session.admit_next(), Err(Error::FrameLimit));
}
fn frame(origin: u32, fixed: bool) -> Frame {
    let mut bytes = Vec::new();
    for row in 0u32..64 {
        for x in 0..20 {
            let value = if fixed && !(7..59).contains(&row) {
                13
            } else {
                let mut seed = (row + origin) * 997 + x * 7919 + 1;
                seed = (seed ^ (seed >> 16)).wrapping_mul(0x7feb352d);
                seed = (seed ^ (seed >> 15)).wrapping_mul(0x846ca68b);
                ((seed ^ (seed >> 16)) % 220) as u8
            };
            bytes.extend_from_slice(&[value, value / 2, 240 - value, 255]);
        }
    }
    Frame::new(20, 64, bytes.into()).unwrap()
}
fn options() -> MatchOptions {
    MatchOptions {
        minimum_overlap: 24,
        maximum_shift: 20,
        maximum_mean_error: 3.0,
    }
}
#[test]
fn session_confirms_fixed_edges_accumulates_and_preserves_accepted_state() {
    let cancel = AtomicBool::new(false);
    let mut session = Session::new(Axis::Vertical, frame(0, true), options()).unwrap();
    assert_eq!(
        session.append(frame(9, true), &cancel).unwrap(),
        Append::Added { origin: 9 }
    );
    assert_eq!(session.confirmed_edges(), None);
    assert!(matches!(
        session.compose(&cancel),
        Err(Error::UnconfirmedEdges)
    ));
    assert_eq!(
        session.append(frame(9, true), &cancel).unwrap(),
        Append::Duplicate
    );
    assert_eq!(session.frame_count(), 2);
    assert_eq!(
        session.append(frame(18, true), &cancel).unwrap(),
        Append::Added { origin: 18 }
    );
    assert_eq!(session.confirmed_edges(), Some((7, 5)));
    let result = session.compose(&cancel).unwrap();
    assert_eq!((result.width, result.height, result.origin), (20, 82, -7));
    let preview = session.preview(&cancel).unwrap();
    assert_eq!(
        (preview.width, preview.height, preview.origin),
        (20, 82, -7)
    );
    assert_eq!(&*preview.pixels, &*result.pixels);
    assert_eq!(
        session.append(frame(0, true), &cancel).unwrap(),
        Append::Added { origin: 0 }
    );
    assert_eq!(&*session.compose(&cancel).unwrap().pixels, &*result.pixels);
}
#[test]
fn rejected_cancelled_and_changed_boundary_frames_do_not_commit() {
    let cancel = AtomicBool::new(false);
    let mut session = Session::new(Axis::Vertical, frame(0, true), options()).unwrap();
    session.append(frame(9, true), &cancel).unwrap();
    let mut flat_pixels = frame(0, true).pixels.to_vec();
    flat_pixels[7 * 80..59 * 80].fill(0);
    let flat = Frame::new(20, 64, flat_pixels.into()).unwrap();
    assert!(matches!(
        session.append(flat, &cancel),
        Ok(Append::Rejected(_))
    ));
    assert!(matches!(
        session.append(frame(18, true), &AtomicBool::new(true)),
        Err(Error::Cancelled)
    ));
    assert_eq!(session.frame_count(), 2);
    let clean = frame(18, true);
    let mut pixels = clean.pixels.to_vec();
    pixels[0] ^= 1;
    let changed = Frame::new(20, 64, pixels.into()).unwrap();
    assert!(matches!(
        session.append(changed, &cancel),
        Err(Error::ChangedEdges)
    ));
    assert_eq!(session.frame_count(), 2);
    assert_eq!(
        session.append(frame(18, true), &cancel).unwrap(),
        Append::Added { origin: 18 }
    );
}
#[test]
fn untrimmed_session_can_compose_two_frames_and_single_frame_shares_pixels() {
    let cancel = AtomicBool::new(false);
    let first = frame(0, false);
    let shared = first.clone();
    let mut session = Session::new(Axis::Vertical, first, options()).unwrap();
    assert!(session
        .compose(&cancel)
        .unwrap()
        .pixels
        .same_storage(&shared.pixels));
    session.append(frame(9, false), &cancel).unwrap();
    assert_eq!(session.confirmed_edges(), Some((0, 0)));
    assert_eq!(session.compose(&cancel).unwrap().height, 73);
}

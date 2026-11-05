use super::*;
fn source() -> Vec<u8> {
    [0x4000u16; 4]
        .iter()
        .flat_map(|n| n.to_le_bytes())
        .collect()
}
fn hdr() -> Frame {
    let data = source();
    let input = ScRgb::new(&data, 1, 1, 8).unwrap();
    let options = Options {
        mode: Mode::Hdr,
        standard_region: true,
        windows_11_or_newer: true,
        gain: 1.0,
    };
    let info = DisplayInfo {
        hdr_enabled: true,
        sdr_white_nits: 320.0,
        minimum_nits: 0.01,
        maximum_nits: 1000.0,
        configuration_known: true,
        white_level_known: true,
    };
    convert(&input, options, info, &AtomicBool::new(false)).unwrap()
}
#[test]
fn hdr_and_sdr_preview_share_source_not_encoding_and_move_without_copying() {
    let frame = hdr();
    validate_output(&frame, (1, 1)).unwrap();
    let Frame::Hdr { image, preview } = &frame else {
        panic!("expected HDR pair")
    };
    assert_eq!(&*preview.pixels, &[188, 188, 188, 255]);
    assert!((image.analysis.maximum_nits - 160.0).abs() < 0.01);
    assert_eq!(image.rgb16_be.len(), 6);
    let hdr_ref = image.rgb16_be.clone();
    let preview_ref = preview.pixels.clone();
    let (preview, original) = frame.into_parts();
    assert!(preview.pixels.same_storage(&preview_ref));
    assert!(original.unwrap().rgb16_be.same_storage(&hdr_ref));
}
#[test]
fn preview_dimensions_and_pixel_lengths_must_match_hdr_and_requested_region() {
    let mut frame = hdr();
    let Frame::Hdr { preview, .. } = &mut frame else {
        unreachable!()
    };
    preview.width = 2;
    assert!(matches!(
        validate_output(&frame, (1, 1)),
        Err(Error::Dimensions)
    ));
    let Frame::Hdr { preview, .. } = &mut frame else {
        unreachable!()
    };
    preview.width = 1;
    preview.pixels = vec![0; 3].into();
    assert!(matches!(
        validate_output(&frame, (1, 1)),
        Err(Error::Dimensions)
    ));
    let sdr = Frame::Sdr(SdrFrame {
        width: 1,
        height: 1,
        pixels: vec![255; 4].into(),
    });
    let (preview, original) = sdr.into_parts();
    assert!(original.is_none());
    assert_eq!(&*preview.pixels, &[255; 4]);
}
#[test]
fn cancelling_a_converted_pair_releases_both_outputs_without_publishing() {
    let frame = hdr();
    let Frame::Hdr { image, preview } = &frame else {
        unreachable!()
    };
    let image = image.rgb16_be.downgrade();
    let preview = preview.pixels.downgrade();
    let cancel = AtomicBool::new(false);
    let options = Options {
        mode: Mode::Hdr,
        standard_region: true,
        windows_11_or_newer: true,
        gain: 1.0,
    };
    let info = DisplayInfo {
        hdr_enabled: true,
        sdr_white_nits: 80.0,
        minimum_nits: 0.0,
        maximum_nits: 1000.0,
        configuration_known: true,
        white_level_known: true,
    };
    let result = execute(
        options,
        &cancel,
        || Ok(((), info)),
        |_, _| {
            cancel.store(true, Ordering::Release);
            Ok(frame)
        },
        || panic!("cancelled pair must not trigger another capture"),
    );
    assert!(matches!(result, Err(Error::Cancelled)));
    assert!(image.upgrade().is_none());
    assert!(preview.upgrade().is_none());
}

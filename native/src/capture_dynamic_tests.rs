use super::*;
fn options(mode: Mode) -> Options {
    Options {
        mode,
        standard_region: true,
        windows_11_or_newer: true,
        gain: 1.0,
    }
}
fn display() -> DisplayInfo {
    DisplayInfo {
        hdr_enabled: true,
        sdr_white_nits: 80.0,
        minimum_nits: 0.01,
        maximum_nits: 1000.0,
        configuration_known: true,
        white_level_known: true,
    }
}
fn sdr() -> SdrFrame {
    SdrFrame {
        width: 1,
        height: 1,
        pixels: vec![255; 4].into(),
    }
}
#[test]
fn explicit_sdr_and_nonstandard_workload_skip_monitor_probe_and_hdr() {
    let cancel = AtomicBool::new(false);
    for (options, expected) in [
        (options(Mode::Sdr), Reason::ExplicitSdr),
        (
            Options {
                standard_region: false,
                ..options(Mode::Hdr)
            },
            Reason::Workload,
        ),
    ] {
        let result = execute::<()>(
            options,
            &cancel,
            || panic!("no probe"),
            |_, _| panic!("no HDR"),
            || Ok(sdr()),
        )
        .unwrap();
        assert_eq!(result.reason, Some(expected));
        assert!(!result.downgraded);
    }
}
#[test]
fn unavailable_target_display_and_os_have_explicit_downgrade_rules() {
    let cancel = AtomicBool::new(false);
    for mode in [Mode::Auto, Mode::Hdr] {
        for reason in [Reason::CrossMonitor, Reason::DisplayUnknown] {
            let result = execute::<()>(
                options(mode),
                &cancel,
                || Err(reason),
                |_, _| panic!("no HDR"),
                || Ok(sdr()),
            )
            .unwrap();
            assert_eq!(result.reason, Some(reason));
            assert_eq!(result.downgraded, mode == Mode::Hdr);
        }
        let result = execute(
            options(mode),
            &cancel,
            || {
                Ok((
                    (),
                    DisplayInfo {
                        hdr_enabled: false,
                        ..display()
                    },
                ))
            },
            |_, _| panic!("no HDR"),
            || Ok(sdr()),
        )
        .unwrap();
        assert_eq!(result.reason, Some(Reason::DisplaySdr));
        assert_eq!(result.downgraded, mode == Mode::Hdr);
        let result = execute(
            Options {
                windows_11_or_newer: false,
                ..options(mode)
            },
            &cancel,
            || Ok(((), display())),
            |_, _| panic!("no HDR"),
            || Ok(sdr()),
        )
        .unwrap();
        assert_eq!(result.reason, Some(Reason::UnsupportedOs));
        assert!(result.downgraded);
    }
}
#[test]
fn automatic_sdr_uses_float_frame_without_recapture_and_force_hdr_retains_pq() {
    let cancel = AtomicBool::new(false);
    for (bits, mode, hdr) in [
        (0x3c00u16, Mode::Auto, false),
        (0x4000, Mode::Auto, true),
        (0x3c00, Mode::Hdr, true),
    ] {
        let data: Vec<_> = [bits; 4].iter().flat_map(|n| n.to_le_bytes()).collect();
        let input = ScRgb::new(&data, 1, 1, 8).unwrap();
        let result = execute(
            options(mode),
            &cancel,
            || Ok(((), display())),
            |_, info| {
                convert(&input, options(mode), info, &cancel)
                    .map_err(|e| Failure::Retryable(format!("{e:?}")))
            },
            || panic!("no second capture after successful FP16 capture"),
        )
        .unwrap();
        assert_eq!(matches!(result.frame, Frame::Hdr { .. }), hdr);
        assert_eq!(result.reason, (!hdr).then_some(Reason::AutoSdr));
        assert_eq!(result.downgraded, !hdr);
    }
}
#[test]
fn automatic_sdr_delivery_preserves_downgrade_and_wgc_session_metadata() {
    let cancel = AtomicBool::new(false);
    let bytes: Vec<_> = [0x3c00u16; 4]
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    let input = ScRgb::new(&bytes, 1, 1, 8).unwrap();
    let result = execute(
        options(Mode::Auto),
        &cancel,
        || Ok(((), display())),
        |_, info| {
            convert(&input, options(Mode::Auto), info, &cancel)
                .map_err(|error| Failure::Retryable(format!("{error:?}")))
        },
        || panic!("automatic SDR must not recapture through GDI"),
    )
    .unwrap();
    let captured = crate::capture_job::PreparedCapture::prepare(
        result,
        std::path::Path::new("C:/Users/Public/nas_home/AI/GameEditor/linshi"),
        "auto-sdr-no-file",
        Default::default(),
        &cancel,
    )
    .unwrap()
    .commit(&cancel, || true)
    .unwrap();
    assert!(captured.original.is_none());
    assert!(captured.original_lease.is_none());
    assert_eq!(captured.reason, Some(Reason::AutoSdr));
    assert!(captured.diagnostic.is_none());
    let metadata = crate::capture_job::Context::region(
        None,
        captured.preview.width,
        captured.preview.height,
        captured.backend,
    )
    .metadata(false, captured.downgraded);
    assert_eq!(metadata["captureBackend"], "wgc");
    assert_eq!(metadata["dynamicRange"], "sdr");
    assert_eq!(metadata["downgradedFromHdr"], true);
    assert!(crate::session_capture::decode(&metadata.to_string()).is_ok());
}
#[test]
fn retryable_failure_falls_back_once_but_cancel_and_layout_change_never_do() {
    let cancel = AtomicBool::new(false);
    let calls = std::cell::Cell::new(0);
    let result = execute(
        options(Mode::Auto),
        &cancel,
        || Ok(((), display())),
        |_, _| Err(Failure::Retryable("injected timeout".into())),
        || {
            calls.set(calls.get() + 1);
            Ok(sdr())
        },
    )
    .unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(result.reason, Some(Reason::HdrFailed));
    assert!(result.downgraded);
    assert_eq!(result.diagnostic.as_deref(), Some("injected timeout"));
    for failure in [Failure::Cancelled, Failure::Changed] {
        assert!(execute(
            options(Mode::Auto),
            &cancel,
            || Ok(((), display())),
            |_, _| Err(failure),
            || panic!("terminal failure must not recapture")
        )
        .is_err());
    }
    assert!(matches!(
        execute(
            options(Mode::Auto),
            &cancel,
            || Ok(((), display())),
            |_, _| {
                cancel.store(true, Ordering::Release);
                Err(Failure::Retryable("late cancellation".into()))
            },
            || panic!("cancel before fallback")
        ),
        Err(Error::Cancelled)
    ));
}
#[test]
fn nested_cancellation_and_resize_are_not_retryable_and_late_sdr_is_discarded() {
    use crate::{capture_readback as r, capture_wgc as w};
    for error in [
        capture_target::Error::Cancelled,
        capture_target::Error::Capture(w::Error::Cancelled),
        capture_target::Error::Capture(w::Error::Readback(r::Error::Cancelled)),
        capture_target::Error::Capture(w::Error::Readback(r::Error::Conversion(
            capture_color::Error::Cancelled,
        ))),
    ] {
        assert!(matches!(failure::classify(error), Failure::Cancelled));
    }
    for error in [
        capture_target::Error::Changed,
        capture_target::Error::NoMonitor,
        capture_target::Error::Capture(w::Error::Resized),
    ] {
        assert!(matches!(failure::classify(error), Failure::Changed));
    }
    let cancel = AtomicBool::new(false);
    assert!(matches!(
        execute::<()>(
            options(Mode::Sdr),
            &cancel,
            || panic!(),
            |_, _| panic!(),
            || {
                cancel.store(true, Ordering::Release);
                Ok(sdr())
            }
        ),
        Err(Error::Cancelled)
    ));
}

#[test]
fn production_sdr_entry_rejects_wrong_size_and_truncated_pixels_without_hdr_probe() {
    let cancel = AtomicBool::new(false);
    let screen = RECT {
        left: -20,
        top: -30,
        right: -19,
        bottom: -29,
    };
    let output = region(screen, options(Mode::Sdr), &cancel, || Ok(sdr())).unwrap();
    assert_eq!(output.reason, Some(Reason::ExplicitSdr));
    assert!(matches!(
        region(
            RECT {
                right: -18,
                ..screen
            },
            options(Mode::Sdr),
            &cancel,
            || Ok(sdr())
        ),
        Err(Error::Dimensions)
    ));
    assert!(matches!(
        region(screen, options(Mode::Sdr), &cancel, || Ok(SdrFrame {
            width: 1,
            height: 1,
            pixels: vec![0; 3].into()
        })),
        Err(Error::Dimensions)
    ));
    assert!(matches!(
        region(RECT::default(), options(Mode::Sdr), &cancel, || panic!(
            "invalid selection must not capture"
        )),
        Err(Error::Dimensions)
    ));
}

#[test]
fn mode_parsing_and_detected_system_options_are_not_guessed() {
    for (value, expected) in [
        (None, Mode::Auto),
        (Some(""), Mode::Auto),
        (Some("unknown"), Mode::Auto),
        (Some(" hDr "), Mode::Hdr),
        (Some("SDR"), Mode::Sdr),
    ] {
        assert_eq!(Mode::parse(value), expected);
    }
    assert!(
        !Options::detect(Mode::Sdr, true, 1.0)
            .unwrap()
            .windows_11_or_newer
    );
    assert!(
        !Options::detect(Mode::Hdr, false, 1.0)
            .unwrap()
            .windows_11_or_newer
    );
    assert_eq!(
        Options::detect(Mode::Auto, true, 1.0)
            .unwrap()
            .windows_11_or_newer,
        crate::capture_platform::query()
            .unwrap()
            .windows_11_or_newer()
    );
}

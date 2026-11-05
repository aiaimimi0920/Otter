use super::*;
fn description() -> DXGI_OUTPUT_DESC1 {
    DXGI_OUTPUT_DESC1 {
        ColorSpace: DXGI_COLOR_SPACE_RGB_FULL_G2084_NONE_P2020,
        MinLuminance: 0.01,
        MaxLuminance: 1000.0,
        ..Default::default()
    }
}
#[test]
fn hdr_requires_matching_dxgi_and_available_system_state_and_tracks_fallbacks() {
    let desc = description();
    let value = combine(&desc, Some((true, Some(2000))));
    assert!(value.hdr_enabled && value.configuration_known && value.white_level_known);
    assert_eq!(value.sdr_white_nits, 160.0);
    assert!(!combine(&desc, Some((false, Some(2000)))).hdr_enabled);
    let sdr = DXGI_OUTPUT_DESC1 {
        ColorSpace: DXGI_COLOR_SPACE_RGB_FULL_G22_NONE_P709,
        ..desc
    };
    assert!(!combine(&sdr, Some((true, Some(2000)))).hdr_enabled);
    let fallback = combine(&description(), None);
    assert!(fallback.hdr_enabled);
    assert!(!fallback.configuration_known && !fallback.white_level_known);
    assert_eq!(fallback.sdr_white_nits, 203.0);
    let partial = combine(&description(), Some((true, None)));
    assert!(partial.configuration_known && !partial.white_level_known);
}
#[test]
fn luminance_metadata_is_sanitized_and_raw_white_matches_windows_units() {
    assert_eq!(
        combine(&description(), Some((true, Some(1000)))).sdr_white_nits,
        80.0
    );
    assert_eq!(
        combine(&description(), Some((true, Some(0)))).sdr_white_nits,
        80.0
    );
    for (minimum, maximum, expected_minimum, expected_maximum) in [
        (f32::NAN, f32::INFINITY, 0.0, 1000.0),
        (-1.0, -5.0, 0.0, 1.0),
        (2000.0, 1000.0, 1000.0, 1000.0),
        (0.01, 20000.0, 0.01, 10000.0),
    ] {
        let desc = DXGI_OUTPUT_DESC1 {
            MinLuminance: minimum,
            MaxLuminance: maximum,
            ..description()
        };
        let info = combine(&desc, None);
        assert_eq!(
            (info.minimum_nits, info.maximum_nits),
            (expected_minimum, expected_maximum)
        );
    }
}
#[test]
fn null_monitor_rejects_before_creating_a_dxgi_factory() {
    assert!(matches!(
        query(HMONITOR::default()),
        Err(Error::InvalidMonitor)
    ));
}

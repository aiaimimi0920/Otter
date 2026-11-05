//! 部分夹具已与真实 Chromium range 归一结果对照；不是 HWND/交互验收。
use super::*;
use crate::surface_scene::Scene;
fn range(props: serde_json::Value) -> Result<Range, Error> {
    let scene = Scene::parse(
        &serde_json::to_vec(&serde_json::json!({"id":"s","type":"slider","props":props})).unwrap(),
    )
    .unwrap();
    Range::parse(scene.root())
}
#[test]
fn surface_slider_defaults_and_strict_numeric_props_match_jsx() {
    for props in [
        serde_json::json!({}),
        serde_json::json!({"min":"-10","max":"20","step":"any","value":"5"}),
    ] {
        let value = range(props).unwrap();
        assert_eq!(
            (value.min(), value.max(), value.step(), value.source()),
            (0., 100., 1., 0.)
        );
    }
    let scene = Scene::parse(br#"{"id":"s","type":"number"}"#).unwrap();
    assert!(matches!(Range::parse(scene.root()), Err(Error::Kind)));
}
#[test]
fn surface_slider_browser_range_normalization_fixtures() {
    for (min, max, step, value, expected) in [
        (0., 0., 1., 0., 0.),
        (0., 100., 20., 50., 60.),
        (10., 5., 1., 20., 10.),
        (0., 10., 3., 10., 9.),
        (-10., 10., 4., -8., -6.),
        (0.1, 0.9, 0.2, 0.4, 0.5),
        (0., 10., 0., 3.5, 4.),
        (0., 10., -2., 3.5, 4.),
        (0., 10., 1., 100., 10.),
        (0., 10., 1., -2., 0.),
        (0.1, 0.9, 0.4, 0.3, 0.5),
    ] {
        let r = range(serde_json::json!({"min":min,"max":max,"step":step,"value":value})).unwrap();
        assert!(
            (r.source() - expected).abs() < 1e-12,
            "{r:?} expected {expected}"
        );
    }
}
#[test]
fn surface_slider_geometry_and_steps_are_bounded_and_monotonic() {
    let r = range(serde_json::json!({"min":-2,"max":3,"step":0.5})).unwrap();
    let mut previous = r.min();
    for i in 0..=100 {
        let next = r.at_fraction(i as f64 / 100.).unwrap();
        assert!(next >= previous && next >= r.min() && next <= r.max());
        assert!((0. ..=1.).contains(&r.fraction(next).unwrap()));
        previous = next;
    }
    assert_eq!(r.advance(3., true), Ok(3.));
    assert_eq!(r.advance(-2., false), Ok(-2.));
    assert_eq!(r.advance(0., true), Ok(0.5));
    assert_eq!(r.advance(0., false), Ok(-0.5));
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(r.normalize(invalid), Err(Error::Value));
        assert_eq!(r.at_fraction(invalid), Err(Error::Value));
    }
    assert!(matches!(
        range(serde_json::json!({"min":-1e308,"max":1e308})),
        Err(Error::Precision)
    ));
    assert!(matches!(
        range(serde_json::json!({"min":0,"max":1,"step":1e-300})),
        Err(Error::Precision)
    ));
}

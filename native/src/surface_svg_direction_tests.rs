//! 浏览器像素基线与真实 SVG worker 回归；不绕过生产准入与字体代理。
#[test]
fn combined_browser_0_rtl_start_natural() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><text x='150' y='40' direction='rtl' text-anchor='start' >FH</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(132.40625 40) scale(1 1)'>F</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(141.203125 40) scale(1 1)'>H</text></svg>"###).unwrap();
    assert_eq!(
        actual
            .pixels
            .iter()
            .zip(expected.pixels.iter())
            .position(|(a, b)| a != b),
        None,
        "renderer pixel mismatch"
    );
}
#[test]
fn combined_browser_1_rtl_start_textlength_80() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><text x='150' y='40' direction='rtl' text-anchor='start' textLength='80'>FH</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(70 40) scale(1 1)'>F</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(141.203125 40) scale(1 1)'>H</text></svg>"###).unwrap();
    assert_eq!(
        actual
            .pixels
            .iter()
            .zip(expected.pixels.iter())
            .position(|(a, b)| a != b),
        None,
        "renderer pixel mismatch"
    );
}
#[test]
fn combined_browser_2_rtl_start_textlength_80_lengthadjust_spacingandglyphs() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><text x='150' y='40' direction='rtl' text-anchor='start' textLength='80' lengthAdjust='spacingAndGlyphs'>FH</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(70.00000762939453 40) scale(4.5470684044738015 1)'>F</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(110 40) scale(4.547069271758437 1)'>H</text></svg>"###).unwrap();
    assert_eq!(
        actual
            .pixels
            .iter()
            .zip(expected.pixels.iter())
            .position(|(a, b)| a != b),
        None,
        "renderer pixel mismatch"
    );
}
#[test]
fn combined_browser_3_rtl_middle_natural() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><text x='150' y='40' direction='rtl' text-anchor='middle' >FH</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(141.203125 40) scale(1 1)'>F</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(150 40) scale(1 1)'>H</text></svg>"###).unwrap();
    assert_eq!(
        actual
            .pixels
            .iter()
            .zip(expected.pixels.iter())
            .position(|(a, b)| a != b),
        None,
        "renderer pixel mismatch"
    );
}
#[test]
fn combined_browser_4_rtl_middle_textlength_80() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><text x='150' y='40' direction='rtl' text-anchor='middle' textLength='80'>FH</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(110 40) scale(1 1)'>F</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(181.203125 40) scale(1 1)'>H</text></svg>"###).unwrap();
    assert_eq!(
        actual
            .pixels
            .iter()
            .zip(expected.pixels.iter())
            .position(|(a, b)| a != b),
        None,
        "renderer pixel mismatch"
    );
}
#[test]
fn combined_browser_5_rtl_middle_textlength_80_lengthadjust_spacingandglyphs() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><text x='150' y='40' direction='rtl' text-anchor='middle' textLength='80' lengthAdjust='spacingAndGlyphs'>FH</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(109.99998474121094 40) scale(4.547069271758437 1)'>F</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(149.99998474121094 40) scale(4.547069271758437 1)'>H</text></svg>"###).unwrap();
    assert_eq!(
        actual
            .pixels
            .iter()
            .zip(expected.pixels.iter())
            .position(|(a, b)| a != b),
        None,
        "renderer pixel mismatch"
    );
}
#[test]
fn combined_browser_6_rtl_end_natural() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><text x='150' y='40' direction='rtl' text-anchor='end' >FH</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(150 40) scale(1 1)'>F</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(158.796875 40) scale(1 1)'>H</text></svg>"###).unwrap();
    assert_eq!(
        actual
            .pixels
            .iter()
            .zip(expected.pixels.iter())
            .position(|(a, b)| a != b),
        None,
        "renderer pixel mismatch"
    );
}
#[test]
fn combined_browser_7_rtl_end_textlength_80() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><text x='150' y='40' direction='rtl' text-anchor='end' textLength='80'>FH</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(150 40) scale(1 1)'>F</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(221.203125 40) scale(1 1)'>H</text></svg>"###).unwrap();
    assert_eq!(
        actual
            .pixels
            .iter()
            .zip(expected.pixels.iter())
            .position(|(a, b)| a != b),
        None,
        "renderer pixel mismatch"
    );
}
#[test]
fn combined_browser_8_rtl_end_textlength_80_lengthadjust_spacingandglyphs() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><text x='150' y='40' direction='rtl' text-anchor='end' textLength='80' lengthAdjust='spacingAndGlyphs'>FH</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(150 40) scale(4.547069271758437 1)'>F</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(190 40) scale(4.547069271758437 1)'>H</text></svg>"###).unwrap();
    assert_eq!(
        actual
            .pixels
            .iter()
            .zip(expected.pixels.iter())
            .position(|(a, b)| a != b),
        None,
        "renderer pixel mismatch"
    );
}
#[test]
fn combined_browser_9_inherited_rtl() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><g direction='rtl'><text x='150' y='40' textLength='80'>FH</text></g></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(70 40) scale(1 1)'>F</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(141.203125 40) scale(1 1)'>H</text></svg>"###).unwrap();
    assert_eq!(
        actual
            .pixels
            .iter()
            .zip(expected.pixels.iter())
            .position(|(a, b)| a != b),
        None,
        "renderer pixel mismatch"
    );
}
#[test]
fn combined_browser_10_override_ltr() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><g direction='rtl'><text direction='ltr' x='150' y='40' textLength='80'>FH</text></g></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(150 40) scale(1 1)'>F</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(221.203125 40) scale(1 1)'>H</text></svg>"###).unwrap();
    assert_eq!(
        actual
            .pixels
            .iter()
            .zip(expected.pixels.iter())
            .position(|(a, b)| a != b),
        None,
        "renderer pixel mismatch"
    );
}
#[test]
fn style_direction_remains_rejected() {
    assert!(super::decode("<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text style='direction:rtl'>FH</text></svg>").is_err());
}

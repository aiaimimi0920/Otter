//! 浏览器像素基线与真实 SVG worker 回归；不绕过生产准入与字体代理。
#[test]
fn combined_browser_0_single_child_under_scale() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><text x='20' y='40' textLength='80' lengthAdjust='spacingAndGlyphs'><tspan textLength='40'>F́</tspan>H</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(20 40) scale(1 1)'>F́</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(59.99998474121094 40) scale(4.547069271758437 1)'>H</text></svg>"###).unwrap();
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
fn combined_browser_1_combining_spacing() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><text x='20' y='40' textLength='80'>F́H</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(20 40) scale(1 1)'>F́</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(91.203125 40) scale(1 1)'>H</text></svg>"###).unwrap();
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
fn combined_browser_2_combining_scale() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><text x='20' y='40' textLength='80' lengthAdjust='spacingAndGlyphs'>F́H</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(19.999998092651367 40) scale(4.547069054937278 1)'>F́</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(59.999996185302734 40) scale(4.547068838116119 1)'>H</text></svg>"###).unwrap();
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
fn combined_browser_3_composed_spacing() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><text x='20' y='40' textLength='80'>éH</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(20 40) scale(1 1)'>é</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(91.203125 40) scale(1 1)'>H</text></svg>"###).unwrap();
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
fn combined_browser_4_decomposed_spacing() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><text x='20' y='40' textLength='80'>éH</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(20 40) scale(1 1)'>é</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(91.203125 40) scale(1 1)'>H</text></svg>"###).unwrap();
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
fn combined_browser_5_combining_nested() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><text x='20' y='40' textLength='120'>F<tspan textLength='40'>H́I</tspan>F</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(20 40) scale(1 1)'>F</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(48.796875 40) scale(1 1)'>H́</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(100 40) scale(1 1)'>I</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(131.203125 40) scale(1 1)'>F</text></svg>"###).unwrap();
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
fn combined_browser_6_single_combining_spacing() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><text x='20' y='40' textLength='80'>F́</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(20 40) scale(1 1)'>F́</text></svg>"###).unwrap();
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

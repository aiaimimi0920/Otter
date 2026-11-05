//! 浏览器像素基线与真实 SVG worker 回归；不绕过生产准入与字体代理。
#[test]
fn combined_browser_0_scale_child_two_leaves() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><text x='20' y='40' textLength='160' lengthAdjust='spacingAndGlyphs'>F<tspan textLength='40' lengthAdjust='spacingAndGlyphs'>H<tspan>I</tspan></tspan>F</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(20 40) scale(2.7780792073712255 1)'>F</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(44.43841552734375 40) scale(2.2735346358792183 1)'>H</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(100 40) scale(2.2735346358792183 1)'>I</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(155.56158447265625 40) scale(2.7780792073712255 1)'>F</text></svg>"###).unwrap();
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
fn combined_browser_1_scale_child_dx_zero() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><text x='20' y='40' textLength='160' lengthAdjust='spacingAndGlyphs'>F<tspan textLength='40' lengthAdjust='spacingAndGlyphs' dx='0 0'>HI</tspan>F</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(20 40) scale(2.7780792073712255 1)'>F</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(44.43841552734375 40) scale(2.2735346358792183 1)'>H</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(100 40) scale(2.2735346358792183 1)'>I</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(155.56158447265625 40) scale(2.7780792073712255 1)'>F</text></svg>"###).unwrap();
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
fn combined_browser_2_scale_child_rotate_zero() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><text x='20' y='40' textLength='160' lengthAdjust='spacingAndGlyphs'>F<tspan textLength='40' lengthAdjust='spacingAndGlyphs' rotate='0 0'>HI</tspan>F</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(20 40) scale(2.7780792073712255 1)'>F</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(44.43841552734375 40) scale(2.2735346358792183 1)'>H</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(100 40) scale(2.2735346358792183 1)'>I</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(155.56158447265625 40) scale(2.7780792073712255 1)'>F</text></svg>"###).unwrap();
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
fn combined_browser_3_scale_parent_spacing_child() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><text x='20' y='40' textLength='160' lengthAdjust='spacingAndGlyphs'>F<tspan textLength='40'>HI</tspan>F</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(20 40) scale(2 1)'>F</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(37.59375 40) scale(1 1)'>H</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(100 40) scale(1 1)'>I</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(162.40625 40) scale(2 1)'>F</text></svg>"###).unwrap();
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
fn combined_browser_4_spacing_parent_scale_child() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><text x='20' y='40' textLength='160'>F<tspan textLength='40' lengthAdjust='spacingAndGlyphs'>HI</tspan>F</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(20 40) scale(1 1)'>F</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(80 40) scale(2.2735337685945827 1)'>H</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(151.203125 40) scale(2.2735346358792183 1)'>I</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(171.203125 40) scale(1 1)'>F</text></svg>"###).unwrap();
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
fn combined_browser_5_scale_both() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><text x='20' y='40' textLength='160' lengthAdjust='spacingAndGlyphs'>F<tspan textLength='40' lengthAdjust='spacingAndGlyphs'>HI</tspan>F</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(20 40) scale(2.7780792073712255 1)'>F</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(44.43841552734375 40) scale(2.2735346358792183 1)'>H</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(64.43841552734375 40) scale(2.2735346358792183 1)'>I</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(155.56158447265625 40) scale(2.7780792073712255 1)'>F</text></svg>"###).unwrap();
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
fn combined_browser_6_scale_child_tail() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><text x='20' y='40'>F<tspan textLength='40' lengthAdjust='spacingAndGlyphs'>HI</tspan>F</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(20 40) scale(1 1)'>F</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(28.796876907348633 40) scale(2.2735344190580595 1)'>H</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(48.796875 40) scale(2.2735346358792183 1)'>I</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(68.796875 40) scale(1 1)'>F</text></svg>"###).unwrap();
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
fn combined_browser_7_scale_both_middle() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><text x='150' y='40' text-anchor='middle' textLength='160' lengthAdjust='spacingAndGlyphs'>F<tspan textLength='40' lengthAdjust='spacingAndGlyphs'>HI</tspan>F</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(70 40) scale(2.7780800746558616 1)'>F</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(94.43841552734375 40) scale(2.2735346358792183 1)'>H</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(114.43841552734375 40) scale(2.2735346358792183 1)'>I</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(205.56155395507812 40) scale(2.7780792073712255 1)'>F</text></svg>"###).unwrap();
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
fn combined_browser_8_scale_compress() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><text x='20' y='40' textLength='20' lengthAdjust='spacingAndGlyphs'>F<tspan textLength='40' lengthAdjust='spacingAndGlyphs'>HI</tspan>F</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(20 40) scale(0.3472599009214032 1)'>F</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(23.054800033569336 40) scale(2.2735344190580595 1)'>H</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(43.0547981262207 40) scale(2.2735346358792183 1)'>I</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(36.94519805908203 40) scale(0.3472599009214032 1)'>F</text></svg>"###).unwrap();
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
fn combined_browser_9_scale_siblings() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Consolas' font-size='16' fill='red'><text x='20' y='40' textLength='160'><tspan textLength='40' lengthAdjust='spacingAndGlyphs'>FH</tspan><tspan textLength='30' lengthAdjust='spacingAndGlyphs'>IF</tspan></text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Consolas' font-size='16' fill='red' transform='translate(20 40) scale(2.2735346358792183 1)'>F</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(130.00001525878906 40) scale(2.2735346358792183 1)'>H</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(150 40) scale(1.7051509769094138 1)'>I</text><text font-family='Consolas' font-size='16' fill='red' transform='translate(254.99998474121094 40) scale(1.7051527114786855 1)'>F</text></svg>"###).unwrap();
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

//! 浏览器像素基线与真实 SVG worker 回归；不绕过生产准入与字体代理。
#[test]
fn combined_browser_0_mixed_ltr_natural() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Arial' font-size='32' fill='red'><text x='150' y='50' direction='ltr' >FאבH</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Arial' font-size='32' fill='red' transform='translate(150 50) scale(1 1)'>F</text><text font-family='Arial' font-size='32' fill='red' transform='translate(186.890625 50) scale(1 1)'>א</text><text font-family='Arial' font-size='32' fill='red' transform='translate(169.546875 50) scale(1 1)'>ב</text><text font-family='Arial' font-size='32' fill='red' transform='translate(204.90625 50) scale(1 1)'>H</text></svg>"###).unwrap();
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
fn combined_browser_1_mixed_ltr_spacing() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Arial' font-size='32' fill='red'><text x='150' y='50' direction='ltr' textLength='120' lengthAdjust='spacing'>FאבH</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Arial' font-size='32' fill='red' transform='translate(150 50) scale(1 1)'>F</text><text font-family='Arial' font-size='32' fill='red' transform='translate(214.88021850585938 50) scale(1 1)'>א</text><text font-family='Arial' font-size='32' fill='red' transform='translate(183.5416717529297 50) scale(1 1)'>ב</text><text font-family='Arial' font-size='32' fill='red' transform='translate(246.890625 50) scale(1 1)'>H</text></svg>"###).unwrap();
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
fn combined_browser_2_mixed_ltr_spacingandglyphs() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Arial' font-size='32' fill='red'><text x='150' y='50' direction='ltr' textLength='120' lengthAdjust='spacingAndGlyphs'>FאבH</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Arial' font-size='32' fill='red' transform='translate(150 50) scale(1.5381538519184652 1)'>F</text><text font-family='Arial' font-size='32' fill='red' transform='translate(206.74342346191406 50) scale(1.5381536955225499 1)'>א</text><text font-family='Arial' font-size='32' fill='red' transform='translate(180.0660858154297 50) scale(1.5381528012387387 1)'>ב</text><text font-family='Arial' font-size='32' fill='red' transform='translate(234.45423889160156 50) scale(1.538151966066599 1)'>H</text></svg>"###).unwrap();
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
fn combined_browser_3_mixed_rtl_natural() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Arial' font-size='32' fill='red'><text x='150' y='50' direction='rtl' >FאבH</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Arial' font-size='32' fill='red' transform='translate(130.453125 50) scale(1 1)'>F</text><text font-family='Arial' font-size='32' fill='red' transform='translate(112.4375 50) scale(1 1)'>א</text><text font-family='Arial' font-size='32' fill='red' transform='translate(95.09375 50) scale(1 1)'>ב</text><text font-family='Arial' font-size='32' fill='red' transform='translate(71.984375 50) scale(1 1)'>H</text></svg>"###).unwrap();
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
fn combined_browser_4_mixed_rtl_spacing() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Arial' font-size='32' fill='red'><text x='150' y='50' direction='rtl' textLength='120' lengthAdjust='spacing'>FאבH</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Arial' font-size='32' fill='red' transform='translate(130.453125 50) scale(1 1)'>F</text><text font-family='Arial' font-size='32' fill='red' transform='translate(98.44271087646484 50) scale(1 1)'>א</text><text font-family='Arial' font-size='32' fill='red' transform='translate(67.10417175292969 50) scale(1 1)'>ב</text><text font-family='Arial' font-size='32' fill='red' transform='translate(30 50) scale(1 1)'>H</text></svg>"###).unwrap();
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
fn combined_browser_5_mixed_rtl_spacingandglyphs() {
    let actual=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><g font-family='Arial' font-size='32' fill='red'><text x='150' y='50' direction='rtl' textLength='120' lengthAdjust='spacingAndGlyphs'>FאבH</text></g></svg>"###).unwrap();
    let expected=super::decode(r###"<svg xmlns='http://www.w3.org/2000/svg' width='300' height='80'><text font-family='Arial' font-size='32' fill='red' transform='translate(119.93391418457031 50) scale(1.5381530712929656 1)'>F</text><text font-family='Arial' font-size='32' fill='red' transform='translate(92.22312927246094 50) scale(1.538153272034909 1)'>א</text><text font-family='Arial' font-size='32' fill='red' transform='translate(65.5457763671875 50) scale(1.5381536810247747 1)'>ב</text><text font-family='Arial' font-size='32' fill='red' transform='translate(30.0000057220459 50) scale(1.5381533691736393 1)'>H</text></svg>"###).unwrap();
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

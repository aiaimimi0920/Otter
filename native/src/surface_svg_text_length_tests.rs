//! Chrome 基线的元素范围与三种锚点；通过真实 SVG child 比较独立绘制结果。
use super::*;
#[test]
fn textlength_case_0() {
    let a=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='20' y='40' text-anchor='start' textLength='80' lengthAdjust='spacing'>FH</text>"###)).unwrap();
    let b=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='20' y='40'>F</text><text font-family='Consolas' font-size='20' fill='red' x='100' y='40' text-anchor='end'>H</text>"###)).unwrap();
    assert_eq!(
        a.pixels
            .iter()
            .zip(b.pixels.iter())
            .position(|(a, b)| a != b),
        None
    );
}
#[test]
fn textlength_case_1() {
    let a=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='20' y='40' text-anchor='start' ><tspan textLength='80' lengthAdjust='spacing'>FH</tspan></text>"###)).unwrap();
    let b=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='20' y='40'>F</text><text font-family='Consolas' font-size='20' fill='red' x='100' y='40' text-anchor='end'>H</text>"###)).unwrap();
    assert_eq!(
        a.pixels
            .iter()
            .zip(b.pixels.iter())
            .position(|(a, b)| a != b),
        None
    );
}
#[test]
fn textlength_case_2() {
    let a=decode(&svg("width='240' height='80'",r###"<defs><path id='p' d='M0 40H240'/></defs><text font-family='Consolas' font-size='20' fill='red' text-anchor='start'><textPath href='#p' startOffset='20' textLength='80' lengthAdjust='spacing'>FH</textPath></text>"###)).unwrap();
    let b=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='20' y='40'>F</text><text font-family='Consolas' font-size='20' fill='red' x='100' y='40' text-anchor='end'>H</text>"###)).unwrap();
    assert_eq!(
        a.pixels
            .iter()
            .zip(b.pixels.iter())
            .position(|(a, b)| a != b),
        None
    );
}
#[test]
fn textlength_case_3() {
    let a=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='120' y='40' text-anchor='middle' textLength='80' lengthAdjust='spacing'>FH</text>"###)).unwrap();
    let b=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='80' y='40'>F</text><text font-family='Consolas' font-size='20' fill='red' x='160' y='40' text-anchor='end'>H</text>"###)).unwrap();
    assert_eq!(
        a.pixels
            .iter()
            .zip(b.pixels.iter())
            .position(|(a, b)| a != b),
        None
    );
}
#[test]
fn textlength_case_4() {
    let a=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='120' y='40' text-anchor='middle' ><tspan textLength='80' lengthAdjust='spacing'>FH</tspan></text>"###)).unwrap();
    let b=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='80' y='40'>F</text><text font-family='Consolas' font-size='20' fill='red' x='160' y='40' text-anchor='end'>H</text>"###)).unwrap();
    assert_eq!(
        a.pixels
            .iter()
            .zip(b.pixels.iter())
            .position(|(a, b)| a != b),
        None
    );
}
#[test]
fn textlength_case_5() {
    let a=decode(&svg("width='240' height='80'",r###"<defs><path id='p' d='M0 40H240'/></defs><text font-family='Consolas' font-size='20' fill='red' text-anchor='middle'><textPath href='#p' startOffset='120' textLength='80' lengthAdjust='spacing'>FH</textPath></text>"###)).unwrap();
    let b=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='80' y='40'>F</text><text font-family='Consolas' font-size='20' fill='red' x='160' y='40' text-anchor='end'>H</text>"###)).unwrap();
    assert_eq!(
        a.pixels
            .iter()
            .zip(b.pixels.iter())
            .position(|(a, b)| a != b),
        None
    );
}
#[test]
fn textlength_case_6() {
    let a=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='200' y='40' text-anchor='end' textLength='80' lengthAdjust='spacing'>FH</text>"###)).unwrap();
    let b=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='120' y='40'>F</text><text font-family='Consolas' font-size='20' fill='red' x='200' y='40' text-anchor='end'>H</text>"###)).unwrap();
    assert_eq!(
        a.pixels
            .iter()
            .zip(b.pixels.iter())
            .position(|(a, b)| a != b),
        None
    );
}
#[test]
fn textlength_case_7() {
    let a=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='200' y='40' text-anchor='end' ><tspan textLength='80' lengthAdjust='spacing'>FH</tspan></text>"###)).unwrap();
    let b=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='120' y='40'>F</text><text font-family='Consolas' font-size='20' fill='red' x='200' y='40' text-anchor='end'>H</text>"###)).unwrap();
    assert_eq!(
        a.pixels
            .iter()
            .zip(b.pixels.iter())
            .position(|(a, b)| a != b),
        None
    );
}
#[test]
fn textlength_case_8() {
    let a=decode(&svg("width='240' height='80'",r###"<defs><path id='p' d='M0 40H240'/></defs><text font-family='Consolas' font-size='20' fill='red' text-anchor='end'><textPath href='#p' startOffset='200' textLength='80' lengthAdjust='spacing'>FH</textPath></text>"###)).unwrap();
    let b=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='120' y='40'>F</text><text font-family='Consolas' font-size='20' fill='red' x='200' y='40' text-anchor='end'>H</text>"###)).unwrap();
    assert_eq!(
        a.pixels
            .iter()
            .zip(b.pixels.iter())
            .position(|(a, b)| a != b),
        None
    );
}
#[test]
fn textlength_case_9() {
    let a=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='20' y='40' textLength='80'>H</text>"###)).unwrap();
    let b = decode(&svg(
        "width='240' height='80'",
        r###"<text font-family='Consolas' font-size='20' fill='red' x='20' y='40'>H</text>"###,
    ))
    .unwrap();
    assert_eq!(
        a.pixels
            .iter()
            .zip(b.pixels.iter())
            .position(|(a, b)| a != b),
        None
    );
}
#[test]
fn textlength_case_10() {
    let a=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='20' y='40' textLength='43.984375' lengthAdjust='spacingAndGlyphs'>FH</text>"###)).unwrap();
    let b=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' transform='translate(20 40) scale(2 1)'>FH</text>"###)).unwrap();
    assert_eq!(
        a.pixels
            .iter()
            .zip(b.pixels.iter())
            .position(|(a, b)| a != b),
        None
    );
}
#[test]
fn textlength_case_11() {
    let a=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='20' y='40' textLength='0'>FH</text>"###)).unwrap();
    let b = decode(&svg(
        "width='240' height='80'",
        r###"<text font-family='Consolas' font-size='20' fill='red' x='20' y='40'>FH</text>"###,
    ))
    .unwrap();
    assert_eq!(
        a.pixels
            .iter()
            .zip(b.pixels.iter())
            .position(|(a, b)| a != b),
        None
    );
}
#[test]
fn textlength_case_12() {
    let a=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='20' y='40' textLength='0' lengthAdjust='spacingAndGlyphs'>FH</text>"###)).unwrap();
    let b = decode(&svg(
        "width='240' height='80'",
        r###"<text font-family='Consolas' font-size='20' fill='red' x='20' y='40'>FH</text>"###,
    ))
    .unwrap();
    assert_eq!(
        a.pixels
            .iter()
            .zip(b.pixels.iter())
            .position(|(a, b)| a != b),
        None
    );
}
#[test]
fn textlength_case_13() {
    let a=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='20' y='40' textLength='80'>F<tspan>H</tspan></text>"###)).unwrap();
    let b=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='20' y='40'>F</text><text font-family='Consolas' font-size='20' fill='red' x='100' y='40' text-anchor='end'>H</text>"###)).unwrap();
    assert_eq!(
        a.pixels
            .iter()
            .zip(b.pixels.iter())
            .position(|(a, b)| a != b),
        None
    );
}
#[test]
fn textlength_case_14() {
    let a=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='20' y='40' textLength='80'>F<tspan fill='blue'>H</tspan></text>"###)).unwrap();
    let b=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='20' y='40'>F</text><text font-family='Consolas' font-size='20' fill='blue' x='100' y='40' text-anchor='end'>H</text>"###)).unwrap();
    assert_eq!(
        a.pixels
            .iter()
            .zip(b.pixels.iter())
            .position(|(a, b)| a != b),
        None
    );
}
#[test]
fn textlength_case_15() {
    let a=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='100' y='40' text-anchor='middle' textLength='43.984375' lengthAdjust='spacingAndGlyphs'>FH</text>"###)).unwrap();
    let b=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' transform='translate(100 40) scale(2 1)' text-anchor='middle'>FH</text>"###)).unwrap();
    assert_eq!(
        a.pixels
            .iter()
            .zip(b.pixels.iter())
            .position(|(a, b)| a != b),
        None
    );
}
#[test]
fn textlength_case_16() {
    let a=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='20' y='40' textLength='15'>FH</text>"###)).unwrap();
    let b=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='20 24.00390625' y='40'>FH</text>"###)).unwrap();
    assert_eq!(
        a.pixels
            .iter()
            .zip(b.pixels.iter())
            .position(|(a, b)| a != b),
        None
    );
}
#[test]
fn textlength_case_17() {
    let a=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='20' y='40'><tspan textLength='80'>FH</tspan>F</text>"###)).unwrap();
    let b=decode(&svg("width='240' height='80'",r###"<text font-family='Consolas' font-size='20' fill='red' x='20' y='40'>F</text><text font-family='Consolas' font-size='20' fill='red' x='100' y='40' text-anchor='end'>H</text><text font-family='Consolas' font-size='20' fill='red' x='158.0078125' y='40'>F</text>"###)).unwrap();
    assert_eq!(
        a.pixels
            .iter()
            .zip(b.pixels.iter())
            .position(|(a, b)| a != b),
        None
    );
}

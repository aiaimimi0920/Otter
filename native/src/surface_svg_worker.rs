//! 私有静态 SVG renderer；所有 XML/树/栅格化仅在受限 Job child 生产入口执行。
use crate::surface_image_worker_host::{self as host, Decoded};
use std::{ffi::OsString, io};
#[path = "surface_svg_filter_budget.rs"]
mod filter_budget;
#[cfg(test)]
#[path = "surface_svg_filter_reference_tests.rs"]
mod filter_references;
#[path = "surface_svg_fonts.rs"]
pub mod fonts;
#[path = "surface_svg_normalize.rs"]
mod normalize;
#[path = "surface_svg_policy.rs"]
mod policy;
#[path = "surface_svg_reference_policy.rs"]
mod reference_policy;
#[path = "surface_svg_text_fonts.rs"]
mod text_fonts;
#[path = "surface_svg_viewport.rs"]
mod viewport;
pub const FLAG: &str = "--native-svg-worker";
// 累计滤镜策略积分；不是耗时或峰值内存承诺，不能替代 Job 与 XML 准入限制。
const MAX_FILTER_WORK: u64 = 4_000_000_000;
#[cfg(test)]
#[path = "surface_svg_work_resource_tests.rs"]
mod work_resources;
fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "Unsupported or invalid Surface SVG",
    )
}
pub fn parse(args: &[OsString]) -> io::Result<bool> {
    if !args.iter().any(|arg| arg == FLAG) {
        return Ok(false);
    }
    if args.len() != 1 {
        return Err(invalid());
    }
    Ok(true)
}
pub fn run_inherited() -> io::Result<()> {
    host::run_for(crate::surface_image_worker_wire::Codec::Svg, decode)
}
fn decode(bytes: &[u8]) -> io::Result<Decoded> {
    let source = std::str::from_utf8(bytes).map_err(|_| invalid())?;
    let doc = policy::parse(source)?;
    filter_budget::validate(&doc)?;
    policy::validate(&doc)?;
    reference_policy::validate(&doc)?;
    let viewport = viewport::inspect(doc.root_element())?;
    let normalized = normalize::normalize(source, doc.root_element(), &viewport)?;
    let doc = policy::parse(&normalized)?;
    let text_fonts = doc
        .descendants()
        .any(|n| n.is_element() && n.tag_name().name() == "text")
        .then(text_fonts::TextFonts::new)
        .transpose()?;
    // 不用 from_str/from_data：它们会重新启用 DTD。双 resolver 拒绝是第二道防线。
    let options = resvg::usvg::Options {
        font_family: text_fonts::DEFAULT_FAMILY.into(),
        font_size: 16.0,
        font_resolver: text_fonts
            .as_ref()
            .map(|fonts| fonts.resolver())
            .unwrap_or_default(),
        image_href_resolver: resvg::usvg::ImageHrefResolver {
            resolve_data: Box::new(|_, _, _| None),
            resolve_string: Box::new(|_, _| None),
        },
        ..Default::default()
    };
    let tree = resvg::usvg::Tree::from_xmltree(&doc, &options).map_err(|_| invalid())?;
    if let Some(fonts) = text_fonts {
        fonts.finish(&tree)?;
    }
    let (inner_width, inner_height) = (viewport.inner_width as f32, viewport.inner_height as f32);
    let (sx, sy) = (
        viewport.logical_size[0] / inner_width,
        viewport.logical_size[1] / inner_height,
    );
    if [inner_width, inner_height, sx, sy]
        .iter()
        .any(|n| !n.is_finite() || *n <= 0.0)
        || tree.size().width() != inner_width
        || tree.size().height() != inner_height
    {
        return Err(invalid());
    }
    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(viewport.width, viewport.height).ok_or_else(invalid)?;
    let transform = resvg::tiny_skia::Transform::from_scale(sx, sy);
    rasterize(&tree, transform, &mut pixmap.as_mut()).map_err(|_| invalid())?;
    let mut pixels = pixmap.take();
    // tiny-skia 已预乘；只交换 RGBA/BGRA，不能二次预乘 alpha。
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    Ok(Decoded {
        logical_size: viewport.logical_size,
        width: viewport.width,
        height: viewport.height,
        pixels,
    })
}
fn rasterize(
    tree: &resvg::usvg::Tree,
    transform: resvg::tiny_skia::Transform,
    pixmap: &mut resvg::tiny_skia::PixmapMut,
) -> Result<(), resvg::RenderError> {
    // 失败时不交付部分像素；嵌套 image/pattern/mask/feImage 共享同一次请求的额度。
    resvg::render_with_filter_budget(tree, transform, pixmap, MAX_FILTER_WORK)
}
#[cfg(test)]
mod tests {
    #[test]
    fn surface_svg_backend_budget_rejects_before_turbulence_work() {
        // 直接验证 worker 后端兜底，不绕过或开放生产 decode 的滤镜拒绝策略。
        let source = "<svg xmlns='http://www.w3.org/2000/svg' width='16' height='16'><defs><filter id='f' x='0' y='0' width='100%' height='100%'><feTurbulence baseFrequency='.1' numOctaves='1000000000'/></filter></defs><rect width='16' height='16' filter='url(#f)'/></svg>";
        let options = resvg::usvg::Options::default();
        let tree = resvg::usvg::Tree::from_str(source, &options).unwrap();
        assert_eq!(tree.filters().len(), 1);
        let mut pixmap = resvg::tiny_skia::Pixmap::new(16, 16).unwrap();
        let transform = resvg::tiny_skia::Transform::identity();
        assert_eq!(
            super::rasterize(&tree, transform, &mut pixmap.as_mut()).unwrap_err(),
            resvg::RenderError::FilterBudgetExceeded
        );
        let tree = resvg::usvg::Tree::from_str(
            "<svg xmlns='http://www.w3.org/2000/svg' width='16' height='16'><rect width='16' height='16' fill='red'/></svg>",
            &options,
        ).unwrap();
        super::rasterize(&tree, transform, &mut pixmap.as_mut()).unwrap();
        assert_eq!(&pixmap.data()[..4], &[255, 0, 0, 255]);
    }
    #[test]
    fn surface_svg_private_mode_is_exclusive() {
        assert!(super::parse(&[super::FLAG.into()]).unwrap());
        assert!(!super::parse(&["--settings".into()]).unwrap());
        assert!(super::parse(&[super::FLAG.into(), "--settings".into()]).is_err());
        assert!(super::parse(&[super::FLAG.into(), "--native-webp-worker".into()]).is_err());
    }
    #[test]
    #[ignore = "private SVG child invoked by bounded parent tests"]
    fn svg_worker_entry() {
        std::process::exit(if super::run_inherited().is_ok() { 0 } else { 1 });
    }
}

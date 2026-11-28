//! Command mapping for original baseline SVG assets; export-native-edit-icons.cjs exports graphic data unchanged.
use neuro_mini_ui::Icon;
macro_rules! svg {
    ($name:literal) => {
        Icon::svg(include_str!(concat!(
            "../../assets/edit-strip/",
            $name,
            ".svg"
        )))
    };
}
pub(super) fn icon(command: usize) -> Icon {
    match command {
        0x56 => svg!("MoveMode"),
        0x53 | 0x6101 => svg!("SelectMode"),
        0x52 => svg!("RectTool"),
        0x21f1 | 0x21f2 | 0x21f3 => Icon::svg(
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="5" width="16" height="14" rx="2" stroke-dasharray="2.5 2.5" /><rect x="7.5" y="8" width="9" height="8" /></svg>"#,
        ),
        0x51 => Icon::svg(
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><rect x="5" y="6" width="14" height="12" rx="3.8" /></svg>"#,
        ),
        0x45 => svg!("EllipseTool"),
        0x21f4 => Icon::svg(
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><circle cx="10.5" cy="10.5" r="6" /><path d="M15 15L20 20M8 10.5H13M10.5 8V13" /></svg>"#,
        ),
        0x54 => svg!("TriangleTool"),
        0x50 => svg!("PolygonTool"),
        0x4c | 0x41 | 0x55 => svg!("LineTool"),
        0x4b => svg!("BezierTool"),
        0x42 | 0x48 => svg!("BrushTool"),
        0x4a => svg!("Paintbrush"),
        0x58 => svg!("TextTool"),
        0x4e => svg!("SerialTool"),
        0x4d => svg!("MosaicTool"),
        0x47 => svg!("BlurTool"),
        0x57 => svg!("EraserTool"),
        0x102e => svg!("CropTool"),
        0x1007 => svg!("UndoTool"),
        0x1008 => svg!("RedoTool"),
        0x1022 => svg!("RasterizeSelectedTool"),
        0x1023 => svg!("RasterizeAllTool"),
        0x6108 => svg!("Ocr"),
        0x6109 => svg!("Translate"),
        _ => svg!("Settings"),
    }
}

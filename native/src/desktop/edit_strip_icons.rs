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
        0x52 | 0x51 | 0x21f1 | 0x21f2 => svg!("RectTool"),
        0x45 | 0x21f3 | 0x21f4 => svg!("EllipseTool"),
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

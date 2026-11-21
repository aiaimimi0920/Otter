//! 原主干 SVG 资产的命令映射；图形数据由 export-native-edit-icons.cjs 原样导出。
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
        0x52 | 0x51 => svg!("RectTool"),
        0x45 => svg!("EllipseTool"),
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

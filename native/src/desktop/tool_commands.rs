//! 菜单与键盘共用工具命令映射，不通过模拟按键触发工具选择。
use crate::editor::Tool;
pub(super) struct ToolCommand {
    pub id: usize,
    pub label: &'static str,
    pub tool: Tool,
}
// 菜单与后续原生工具栏共享顺序、名称和快捷键；模糊必须走后台准备入口。
pub(super) const TOOLS: &[ToolCommand] = &[
    ToolCommand {
        id: 0x56,
        label: "移动\tV",
        tool: Tool::Pan,
    },
    ToolCommand {
        id: 0x53,
        label: "选择标注\tS",
        tool: Tool::Select,
    },
    ToolCommand {
        id: 0x42,
        label: "画笔\tB",
        tool: Tool::Brush,
    },
    ToolCommand {
        id: 0x48,
        label: "荧光笔\tH",
        tool: Tool::Highlighter,
    },
    ToolCommand {
        id: 0x4C,
        label: "直线\tL",
        tool: Tool::Line,
    },
    ToolCommand {
        id: 0x41,
        label: "箭头\tA",
        tool: Tool::Arrow,
    },
    ToolCommand {
        id: 0x4B,
        label: "贝塞尔曲线\tK",
        tool: Tool::Bezier,
    },
    ToolCommand {
        id: 0x55,
        label: "折线\tU",
        tool: Tool::Polyline,
    },
    ToolCommand {
        id: 0x4A,
        label: "笔刷\tJ",
        tool: Tool::Paintbrush,
    },
    ToolCommand {
        id: 0x58,
        label: "文字\tX",
        tool: Tool::Text,
    },
    ToolCommand {
        id: 0x4E,
        label: "序号\tN",
        tool: Tool::Serial,
    },
    ToolCommand {
        id: 0x4D,
        label: "马赛克\tM",
        tool: Tool::Mosaic,
    },
    ToolCommand {
        id: 0x47,
        label: "模糊\tG",
        tool: Tool::Blur,
    },
    ToolCommand {
        id: 0x57,
        label: "内容橡皮擦\tW",
        tool: Tool::Eraser,
    },
    ToolCommand {
        id: 0x52,
        label: "矩形\tR",
        tool: Tool::Rectangle,
    },
    ToolCommand {
        id: 0x51,
        label: "圆角矩形\tQ",
        tool: Tool::RoundedRectangle,
    },
    ToolCommand {
        id: 0x45,
        label: "椭圆\tE",
        tool: Tool::Ellipse,
    },
    ToolCommand {
        id: 0x54,
        label: "三角形\tT",
        tool: Tool::Triangle,
    },
    ToolCommand {
        id: 0x50,
        label: "多边形\tP",
        tool: Tool::Polygon,
    },
];
pub(super) fn tool_for_command(command: usize) -> Option<Tool> {
    TOOLS
        .iter()
        .find(|entry| entry.id == command && entry.tool != Tool::Blur)
        .map(|entry| entry.tool)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tool_catalog_is_unique_complete_and_preserves_async_blur_boundary() {
        let expected = [
            Tool::Pan,
            Tool::Select,
            Tool::Brush,
            Tool::Highlighter,
            Tool::Line,
            Tool::Arrow,
            Tool::Bezier,
            Tool::Polyline,
            Tool::Paintbrush,
            Tool::Text,
            Tool::Serial,
            Tool::Mosaic,
            Tool::Blur,
            Tool::Eraser,
            Tool::Rectangle,
            Tool::RoundedRectangle,
            Tool::Ellipse,
            Tool::Triangle,
            Tool::Polygon,
        ];
        assert_eq!(TOOLS.len(), expected.len());
        for tool in expected {
            assert_eq!(TOOLS.iter().filter(|entry| entry.tool == tool).count(), 1);
        }
        for (index, entry) in TOOLS.iter().enumerate() {
            assert!(!TOOLS[..index]
                .iter()
                .any(|previous| previous.id == entry.id));
            let (name, shortcut) = entry.label.split_once('\t').unwrap();
            assert!(!name.is_empty());
            assert_eq!(shortcut.as_bytes(), &[entry.id as u8]);
            assert_eq!(
                tool_for_command(entry.id),
                (entry.tool != Tool::Blur).then_some(entry.tool)
            );
        }
        assert_eq!(tool_for_command(0), None);
        assert_eq!(tool_for_command(0x1009), None);
    }
}

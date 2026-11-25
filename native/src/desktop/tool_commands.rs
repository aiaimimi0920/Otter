//! Menus/keyboards share tool-command mapping, without selecting tools through simulated keys.
use crate::editor::Tool;
pub(super) struct ToolCommand {
    pub id: usize,
    pub label_id: &'static str,
    pub tool: Tool,
}
impl ToolCommand {
    pub(super) fn label(&self) -> &'static str {
        crate::ui_text::text(self.label_id)
    }
}
// Menus and subsequent native toolbars share order, names, and shortcuts; blur must use background preparation.
pub(super) const TOOLS: &[ToolCommand] = &[
    ToolCommand {
        id: 0x56,
        label_id: "tool-pan",
        tool: Tool::Pan,
    },
    ToolCommand {
        id: 0x53,
        label_id: "tool-select",
        tool: Tool::Select,
    },
    ToolCommand {
        id: 0x42,
        label_id: "tool-brush",
        tool: Tool::Brush,
    },
    ToolCommand {
        id: 0x4C,
        label_id: "tool-line",
        tool: Tool::Line,
    },
    ToolCommand {
        id: 0x4B,
        label_id: "tool-bezier",
        tool: Tool::Bezier,
    },
    ToolCommand {
        id: 0x4A,
        label_id: "tool-paintbrush",
        tool: Tool::Paintbrush,
    },
    ToolCommand {
        id: 0x58,
        label_id: "tool-text",
        tool: Tool::Text,
    },
    ToolCommand {
        id: 0x4E,
        label_id: "tool-serial",
        tool: Tool::Serial,
    },
    ToolCommand {
        id: 0x4D,
        label_id: "tool-mosaic",
        tool: Tool::Mosaic,
    },
    ToolCommand {
        id: 0x47,
        label_id: "tool-blur",
        tool: Tool::Blur,
    },
    ToolCommand {
        id: 0x57,
        label_id: "tool-eraser",
        tool: Tool::Eraser,
    },
    ToolCommand {
        id: 0x52,
        label_id: "tool-rectangle",
        tool: Tool::Rectangle,
    },
    ToolCommand {
        id: 0x45,
        label_id: "tool-ellipse",
        tool: Tool::Ellipse,
    },
    ToolCommand {
        id: 0x54,
        label_id: "tool-triangle",
        tool: Tool::Triangle,
    },
    ToolCommand {
        id: 0x50,
        label_id: "tool-polygon",
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
            Tool::Line,
            Tool::Bezier,
            Tool::Paintbrush,
            Tool::Text,
            Tool::Serial,
            Tool::Mosaic,
            Tool::Blur,
            Tool::Eraser,
            Tool::Rectangle,
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
            for language in crate::ui_text::DisplayLanguage::ALL {
                let label = crate::ui_text::text_for(language, entry.label_id);
                let (name, shortcut) = label.split_once('\t').unwrap();
                assert!(!name.is_empty());
                assert_eq!(shortcut.as_bytes(), &[entry.id as u8]);
            }
            assert_eq!(
                tool_for_command(entry.id),
                (entry.tool != Tool::Blur).then_some(entry.tool)
            );
        }
        assert_eq!(tool_for_command(0), None);
        assert_eq!(tool_for_command(0x1009), None);
        assert_eq!(tool_for_command(0x51), None);
        assert_eq!(tool_for_command(0x41), None);
        assert_eq!(tool_for_command(0x55), None);
        assert_eq!(tool_for_command(0x48), None);
        assert!(!TOOLS
            .iter()
            .any(|entry| entry.tool == Tool::RoundedRectangle));
    }
}

//! Stroke-width/tool-size menus; menu hosts provide command IDs, while existing property entry points handle actions.
use crate::{editor::Tool, stroke_style::StrokeStyle};
use windows::{
    core::{Result, HSTRING},
    Win32::UI::WindowsAndMessaging::*,
};

pub(super) unsafe fn append_current(
    menu: HMENU,
    state: &super::State,
    tool: Tool,
    commands: [usize; 4],
) -> Result<()> {
    let canvas = state.canvas.borrow();
    let selected = super::blur_input::selected_strength(&canvas);
    AppendMenuW(
        menu,
        MF_STRING
            | if tool == Tool::Mosaic {
                MF_ENABLED
            } else {
                MF_GRAYED
            },
        super::numeric_style::MOSAIC,
        &HSTRING::from(crate::ui_text::text("workspace-menu-exact-cell")),
    )?;
    append(
        menu,
        if selected.is_some() { Tool::Blur } else { tool },
        canvas.editor.style(),
        selected.unwrap_or(canvas.tool_profiles.blur_strength()),
        commands,
    )
}

pub(super) unsafe fn append(
    menu: HMENU,
    tool: Tool,
    style: StrokeStyle,
    blur_strength: f32,
    commands: [usize; 4],
) -> Result<()> {
    let mosaic = tool == Tool::Mosaic;
    let dimension = crate::ui_text::text(if mosaic {
        "workspace-menu-cell-width"
    } else if tool == Tool::Blur {
        "workspace-menu-blur-strength"
    } else {
        "workspace-menu-corner-radius"
    });
    let value = if mosaic {
        style.mosaic_cell()
    } else if tool == Tool::Blur {
        blur_strength
    } else {
        style.corner_radius()
    };
    let summary = HSTRING::from(crate::ui_text::format(
        "workspace-menu-dimensions",
        &[
            ("width", style.width().into()),
            ("dimension", dimension.into()),
            ("value", value.into()),
        ],
    ));
    AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, &summary)?;
    let labels = [
        "workspace-menu-width-down",
        "workspace-menu-width-up",
        if mosaic {
            "workspace-menu-cell-down"
        } else if tool == Tool::Blur {
            "workspace-menu-blur-down"
        } else {
            "workspace-menu-corner-down"
        },
        if mosaic {
            "workspace-menu-cell-up"
        } else if tool == Tool::Blur {
            "workspace-menu-blur-up"
        } else {
            "workspace-menu-corner-up"
        },
    ];
    for (command, label) in commands.into_iter().zip(labels) {
        AppendMenuW(
            menu,
            MF_STRING,
            command,
            &HSTRING::from(crate::ui_text::text(label)),
        )?;
    }
    Ok(())
}

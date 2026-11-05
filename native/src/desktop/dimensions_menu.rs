//! 线宽与工具尺寸菜单；命令编号由菜单宿主提供，操作仍由现有属性入口处理。
use crate::{editor::Tool, stroke_style::StrokeStyle};
use windows::{
    core::{w, Result, HSTRING},
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
        w!("精确色块大小…"),
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
    let summary = HSTRING::from(format!(
        "线宽：{} px　{}：{} px",
        style.width(),
        if mosaic {
            "格宽"
        } else if tool == Tool::Blur {
            "模糊强度"
        } else {
            "圆角"
        },
        if mosaic {
            style.mosaic_cell()
        } else if tool == Tool::Blur {
            blur_strength
        } else {
            style.corner_radius()
        }
    ));
    AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, &summary)?;
    let labels = [
        w!("减小线宽\t["),
        w!("增大线宽\t]"),
        if mosaic {
            w!("减小格宽\tShift+[")
        } else if tool == Tool::Blur {
            w!("减小模糊强度\tShift+[")
        } else {
            w!("减小圆角\tShift+[")
        },
        if mosaic {
            w!("增大格宽\tShift+]")
        } else if tool == Tool::Blur {
            w!("增大模糊强度\tShift+]")
        } else {
            w!("增大圆角\tShift+]")
        },
    ];
    for (command, label) in commands.into_iter().zip(labels) {
        AppendMenuW(menu, MF_STRING, command, label)?;
    }
    Ok(())
}

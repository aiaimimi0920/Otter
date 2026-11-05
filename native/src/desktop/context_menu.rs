//! 按需构建业务菜单快照；自绘宿主重入消息循环，不能跨调用借用 Canvas。
use super::tool_commands::tool_for_command;
use super::{application as app, State};
use crate::{editor::Tool, stroke_style::DashPattern};
#[path = "context_menu_shapes.rs"]
mod shapes;
use windows::{
    core::{w, Result},
    Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*},
};

struct Menu(HMENU);
impl Drop for Menu {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyMenu(self.0);
        }
    }
}

pub(super) const COLOR: usize = 0x1001;
pub(super) const FILL_COLOR: usize = 0x1002;
pub(super) const WIDTH_DOWN: usize = 0x1003;
pub(super) const WIDTH_UP: usize = 0x1004;
pub(super) const RADIUS_DOWN: usize = 0x1005;
pub(super) const RADIUS_UP: usize = 0x1006;
const UNDO: usize = 0x1007;
const REDO: usize = 0x1008;
pub(super) const EXPORT: usize = 0x1009;
pub(super) const OPEN: usize = 0x100A;
const REMOVE_LAST: usize = 0x100B;
pub(super) const APPLY_STYLE: usize = 0x100C;
const ENLARGE: usize = 0x100D;
const SHRINK: usize = 0x100E;
const ROTATE_LEFT: usize = 0x100F;
const ROTATE_RIGHT: usize = 0x1010;
const WIDEN: usize = 0x1011;
const NARROW: usize = 0x1012;
const TALLER: usize = 0x1013;
const SHORTER: usize = 0x1014;
const DUPLICATE: usize = 0x1015;
const ANGLE_SNAP: usize = 0x1016;
const LINE_ARROW: usize = 0x1017;
const BRUSH_SMOOTHING: usize = 0x1018;
const EDIT_TEXT: usize = 0x101E;
const SHAPE_LOCK: usize = 0x1019;
const SHAPE_STEP: usize = 0x101A;
const SHAPE_RATIO: usize = 0x101B;
const SHAPE_CUSTOM_RATIO: usize = 0x101C;
const SHAPE_CUSTOM_STEP: usize = 0x101D;

use super::popup_menu::target::MenuTarget;

pub(super) unsafe fn show(hwnd: HWND, state: &State, position: LPARAM) -> Result<()> {
    super::menu_transition::prepare(hwnd, state)?;
    let _ = InvalidateRect(Some(hwnd), None, false);
    let target = MenuTarget::capture(&state.canvas.borrow());
    let (tool, style) = {
        let canvas = state.canvas.borrow();
        (canvas.tool, canvas.editor.style())
    };
    let menu = Menu(CreatePopupMenu()?);
    super::capture_input::append_pin(menu.0)?;
    super::capture_input::info::append(menu.0, state)?;
    super::export_job::original::append(menu.0, state)?;
    super::application::append_menu(menu.0, state)?;
    super::execution_cancel::append(menu.0, state)?;
    let node_snapshot = super::node_menu::Snapshot::append(menu.0, state)?;
    super::group_menu::append(menu.0, state)?;
    super::capture_protection::append(menu.0, state)?;
    super::render_commands::append(menu.0, state)?;
    super::global_capture::append(menu.0, state)?;
    super::blur_numeric::append(menu.0, state)?;
    super::blur_default::append(menu.0, state)?;
    label(menu.0, super::tray::COMMAND, "隐藏到托盘", MF_STRING)?;
    let shape_flags = MF_STRING
        | if crate::shape_options::ShapeOptions::supports(tool) {
            MF_ENABLED
        } else {
            MF_GRAYED
        };
    label(menu.0, SHAPE_CUSTOM_STEP, "自定义形状步进…", shape_flags)?;
    label(
        menu.0,
        SHAPE_CUSTOM_RATIO,
        "自定义矩形 / 椭圆比例…",
        MF_STRING
            | if crate::shape_options::ShapeOptions::supports_ratio(tool) {
                MF_ENABLED
            } else {
                MF_GRAYED
            },
    )?;
    super::open_image::append(menu.0, state)?;
    AppendMenuW(menu.0, MF_SEPARATOR, 0, None)?;
    for entry in super::tool_commands::TOOLS {
        label(
            menu.0,
            entry.id,
            entry.label,
            MF_STRING
                | if tool == entry.tool {
                    MF_CHECKED
                } else {
                    MF_UNCHECKED
                },
        )?;
    }
    if state.blur_job.borrow().is_busy() {
        AppendMenuW(menu.0, MF_STRING | MF_GRAYED, 0, w!("模糊准备中…"))?;
    }
    super::erase_input::append_menu(menu.0, state, tool)?;
    super::rasterize_input::append_menu(menu.0, state)?;
    super::crop_input::append(menu.0, state)?;
    super::flip_input::append(menu.0, state)?;
    super::copy_job::append(menu.0, state)?;
    super::paste_input::append(menu.0, state)?;
    label(
        menu.0,
        super::capture_input::COMMAND,
        "区域截图并替换当前图片…",
        MF_STRING,
    )?;
    label(menu.0, super::numeric_style::WIDTH, "精确粗细…", MF_STRING)?;
    label(
        menu.0,
        super::capture_input::FULL_SCREEN,
        "全屏截图并替换当前图片（所有显示器）",
        MF_STRING,
    )?;
    label(
        menu.0,
        super::numeric_style::OPACITY,
        "精确透明度…",
        MF_STRING,
    )?;
    super::transform_mode::append(menu.0, state)?;
    AppendMenuW(menu.0, MF_SEPARATOR, 0, None)?;
    label(
        menu.0,
        0,
        &format!("多边形边数：{}", style.sides()),
        MF_STRING | MF_GRAYED,
    )?;
    AppendMenuW(menu.0, MF_STRING, 0x21, w!("增加边数\tPageUp"))?;
    AppendMenuW(menu.0, MF_STRING, 0x22, w!("减少边数\tPageDown"))?;
    AppendMenuW(
        menu.0,
        MF_STRING
            | if matches!(tool, Tool::Line | Tool::Arrow) {
                MF_ENABLED
            } else {
                MF_GRAYED
            }
            | if state.canvas.borrow().tool_profiles.angle_snap(tool) {
                MF_CHECKED
            } else {
                MF_UNCHECKED
            },
        ANGLE_SNAP,
        w!("直线 / 箭头角度吸附（5°）"),
    )?;
    AppendMenuW(menu.0, MF_STRING, COLOR, w!("边框 / 画笔颜色…\tC"))?;
    super::screen_color::append(menu.0)?;
    label(
        menu.0,
        BRUSH_SMOOTHING,
        &format!(
            "画笔平滑：{}%（切换）",
            state.canvas.borrow().tool_profiles.smoothing(tool)
        ),
        MF_STRING
            | if matches!(tool, Tool::Brush | Tool::Paintbrush) {
                MF_ENABLED
            } else {
                MF_GRAYED
            },
    )?;
    AppendMenuW(
        menu.0,
        MF_STRING
            | if matches!(tool, Tool::Line | Tool::Bezier) {
                MF_ENABLED
            } else {
                MF_GRAYED
            }
            | if style.arrow() {
                MF_CHECKED
            } else {
                MF_UNCHECKED
            },
        LINE_ARROW,
        w!("直线 / 贝塞尔箭头"),
    )?;
    AppendMenuW(menu.0, MF_STRING, FILL_COLOR, w!("填充颜色…\tShift+C"))?;
    {
        let canvas = state.canvas.borrow();
        let shapes = &canvas.tool_profiles.shapes;
        let flags = MF_STRING
            | if crate::shape_options::ShapeOptions::supports(tool) {
                MF_ENABLED
            } else {
                MF_GRAYED
            };
        label(
            menu.0,
            SHAPE_LOCK,
            "形状等比例绘制",
            flags
                | if shapes.lock_aspect {
                    MF_CHECKED
                } else {
                    MF_UNCHECKED
                },
        )?;
        label(
            menu.0,
            SHAPE_STEP,
            &format!("形状尺寸步进：{} px（0 为关闭，点击切换）", shapes.step()),
            flags,
        )?;
        label(
            menu.0,
            SHAPE_RATIO,
            &format!("矩形 / 椭圆比例：{}（点击切换）", shapes.ratio_label()),
            MF_STRING
                | if crate::shape_options::ShapeOptions::supports_ratio(tool) {
                    MF_ENABLED
                } else {
                    MF_GRAYED
                },
        )?;
    }
    AppendMenuW(
        menu.0,
        MF_STRING
            | if style.fill().is_some() {
                MF_CHECKED
            } else {
                MF_UNCHECKED
            },
        0x46,
        w!("形状填充\tF"),
    )?;
    let dash = match style.dash() {
        DashPattern::Solid => "实线",
        DashPattern::Dash => "虚线",
        DashPattern::DashDot => "点划线",
    };
    label(menu.0, 0x44, &format!("切换线型：{dash}\tD"), MF_STRING)?;
    label(
        menu.0,
        0x4F,
        &format!("切换不透明度：{:.0}%\tO", style.opacity() * 100.0),
        MF_STRING,
    )?;
    super::dimensions_menu::append_current(
        menu.0,
        state,
        tool,
        [WIDTH_DOWN, WIDTH_UP, RADIUS_DOWN, RADIUS_UP],
    )?;
    AppendMenuW(menu.0, MF_SEPARATOR, 0, None)?;
    AppendMenuW(menu.0, MF_STRING, UNDO, w!("撤销\tCtrl+Z"))?;
    AppendMenuW(menu.0, MF_STRING, REDO, w!("重做\tCtrl+Y"))?;
    let resize_flags = MF_STRING
        | if state.canvas.borrow().selected_index().is_some() {
            MF_ENABLED
        } else {
            MF_GRAYED
        };
    let group_flags = MF_STRING
        | if state.canvas.borrow().selected_indices().is_empty() {
            MF_GRAYED
        } else {
            MF_ENABLED
        };
    AppendMenuW(menu.0, group_flags, ENLARGE, w!("放大选中标注\t+"))?;
    AppendMenuW(menu.0, group_flags, SHRINK, w!("缩小选中标注\t-"))?;
    AppendMenuW(menu.0, resize_flags, DUPLICATE, w!("复制选中标注\tCtrl+D"))?;
    super::text_input::append_edit_menu(menu.0, EDIT_TEXT, &state.canvas.borrow())?;
    super::serial_input::append_menu(menu.0, &state.canvas.borrow())?;
    for (id, text) in [
        (WIDEN, w!("增加选中标注宽度")),
        (NARROW, w!("减少选中标注宽度")),
        (TALLER, w!("增加选中标注高度")),
        (SHORTER, w!("减少选中标注高度")),
    ] {
        AppendMenuW(menu.0, group_flags, id, text)?;
    }
    AppendMenuW(menu.0, group_flags, ROTATE_LEFT, w!("逆时针旋转 15°\t,"))?;
    AppendMenuW(menu.0, group_flags, ROTATE_RIGHT, w!("顺时针旋转 15°\t."))?;
    AppendMenuW(
        menu.0,
        MF_STRING,
        APPLY_STYLE,
        if tool == Tool::Select {
            w!("将当前样式应用到选中标注\tCtrl+Enter")
        } else {
            w!("将当前样式应用到最后一个标注\tCtrl+Enter")
        },
    )?;
    AppendMenuW(
        menu.0,
        MF_STRING,
        REMOVE_LAST,
        if tool == Tool::Select {
            w!("删除选中标注\tDelete")
        } else {
            w!("删除最后一个标注\tDelete")
        },
    )?;
    super::export_job::append(menu.0, state, EXPORT)?;
    let mut point = POINT {
        x: position.0 as i16 as i32,
        y: (position.0 >> 16) as i16 as i32,
    };
    if position.0 == -1 {
        point = POINT { x: 16, y: 16 };
        if !ClientToScreen(hwnd, &mut point).as_bool() {
            return Err(windows::core::Error::from_win32());
        }
    }
    super::group_lock::restrict_menu(menu.0, state);
    let current = || target.matches(&state.canvas.borrow());
    let command = super::popup_menu::track(hwnd, state, menu.0, point, current)?;
    if command == 0 || !IsWindow(Some(hwnd)).as_bool() || !target.matches(&state.canvas.borrow()) {
        return Ok(());
    }
    if node_snapshot.apply(hwnd, state, command)? {
        return Ok(());
    }
    execute(hwnd, state, command)
}

/// 即时 UI 命令的共用执行入口；延迟 UI 必须在调用前校验其目标快照。
pub(super) unsafe fn execute(hwnd: HWND, state: &State, command: usize) -> Result<()> {
    if command == 0 || !IsWindow(Some(hwnd)).as_bool() {
        return Ok(());
    }
    if super::render_commands::handles(command) {
        return super::render_commands::execute(hwnd, state, command);
    }
    if state.canvas.borrow().minified.is_some()
        && (((COLOR..=EDIT_TEXT).contains(&command) && !matches!(command, EXPORT | OPEN))
            || tool_for_command(command).is_some())
    {
        return Ok(());
    }
    if state.group_locked.get() && !super::group_lock::allows(command) {
        return Ok(());
    }
    super::menu_transition::prepare(hwnd, state)?;
    match command {
        command if super::group_menu::handles(command) => {
            super::group_menu::execute(hwnd, state, command)?
        }
        command if app::handles(command) => app::execute(hwnd, state, command)?,
        super::execution_cancel::COMMAND => super::execution_cancel::stop(hwnd, state)?,
        super::capability_catalog::COMMAND => super::capability_catalog::request(hwnd, state)?,
        super::execution_start::COMMAND => super::execution_start::request(hwnd, state)?,
        super::sticker_propagation::COMMAND => super::sticker_propagation::toggle(hwnd, state)?,
        command if super::execution_settings::handles(command) => {
            super::execution_settings::toggle(hwnd, state, command)?
        }
        super::capability_parameters::COMMAND => super::capability_parameters::show(hwnd, state)?,
        super::blur_default::COMMAND => super::blur_default::show(hwnd, state),
        super::blur_numeric::COMMAND => super::blur_numeric::show(hwnd, state),
        super::global_capture::COMMAND => super::global_capture::toggle(hwnd, state),
        super::tray::COMMAND => super::tray::hide(hwnd, state)?,
        super::capture_protection::COMMAND => super::capture_protection::toggle(hwnd, state),
        super::capture_input::info::COMMAND => super::capture_input::info::show(hwnd, state),
        super::export_job::original::COMMAND => super::export_job::original::start(hwnd, state),
        super::capture_input::COMMAND
        | super::capture_input::FULL_SCREEN
        | super::capture_input::NEW_PIN
        | super::capture_input::NEW_FULL_PIN
        | super::capture_input::LONG_RECORDING
        | super::capture_input::LONG_VERTICAL
        | super::capture_input::LONG_HORIZONTAL => {
            super::capture_input::start(hwnd, state, command)
        }
        super::numeric_style::WIDTH
        | super::numeric_style::OPACITY
        | super::numeric_style::MOSAIC => super::numeric_style::show(hwnd, state, command),
        super::paste_input::COMMAND => super::paste_input::start(hwnd, state),
        super::paste_input::REPLACE => super::paste_input::start_with(hwnd, state, false),
        super::copy_job::COMMAND => super::copy_job::start(hwnd, state),
        super::flip_input::HORIZONTAL | super::flip_input::VERTICAL => {
            super::flip_input::start(hwnd, state, command)
        }
        super::crop_input::START | super::crop_input::RESET => {
            super::crop_input::command(hwnd, state, command)
        }
        0x47 => super::blur_input::start(hwnd, state),
        EDIT_TEXT => super::text_input::edit_selected(hwnd, state),
        super::text_font::COMMAND => super::text_font::show(hwnd, state),
        super::serial_input::COMMAND => super::serial_input::show(hwnd, state),
        SHAPE_CUSTOM_RATIO | SHAPE_CUSTOM_STEP => {
            if !shapes::edit(hwnd, state, command == SHAPE_CUSTOM_RATIO) {
                return Ok(());
            }
        }
        SHAPE_LOCK | SHAPE_STEP | SHAPE_RATIO => {
            let mut canvas = state.canvas.borrow_mut();
            let tool = canvas.tool;
            let shapes = &mut canvas.tool_profiles.shapes;
            if crate::shape_options::ShapeOptions::supports(tool) {
                match command {
                    SHAPE_LOCK => shapes.lock_aspect = !shapes.lock_aspect,
                    SHAPE_STEP => shapes.cycle_step(),
                    SHAPE_RATIO if crate::shape_options::ShapeOptions::supports_ratio(tool) => {
                        shapes.cycle_ratio()
                    }
                    _ => {}
                }
            }
        }
        BRUSH_SMOOTHING => {
            let mut canvas = state.canvas.borrow_mut();
            let tool = canvas.tool;
            canvas.tool_profiles.cycle_smoothing(tool);
        }
        LINE_ARROW => {
            let mut canvas = state.canvas.borrow_mut();
            if matches!(canvas.tool, Tool::Line | Tool::Bezier) {
                let style = canvas.editor.style();
                canvas.editor.set_style(style.with_arrow(!style.arrow()));
            }
        }
        ANGLE_SNAP => {
            let mut canvas = state.canvas.borrow_mut();
            let tool = canvas.tool;
            canvas.tool_profiles.toggle_angle_snap(tool);
        }
        command if super::style_input::execute(hwnd, state, command)? => (),
        UNDO | REDO => {
            let mut canvas = state.canvas.borrow_mut();
            if command == UNDO {
                canvas.editor.undo();
            } else {
                canvas.editor.redo();
            }
            canvas.sync_raster_source();
        }
        super::erase_input::MODE_COMMAND => super::erase_input::toggle_mode(state),
        super::transform_mode::COMMAND..=super::transform_mode::SCALE_Y => {
            super::transform_mode::dispatch(state, command)
        }
        super::rasterize_input::SELECTED => super::rasterize_input::start(hwnd, state, true),
        super::rasterize_input::ALL => super::rasterize_input::start(hwnd, state, false),
        EXPORT => super::export_job::start(hwnd, state),
        OPEN => super::open_image::start(hwnd, state),
        super::open_image::REPLACE => super::open_image::start_destination(hwnd, state, false),
        DUPLICATE => {
            state.canvas.borrow_mut().duplicate_selected();
        }
        WIDEN | NARROW | TALLER | SHORTER => {
            super::resize_input::axis_step(
                hwnd,
                state,
                matches!(command, WIDEN | NARROW),
                matches!(command, WIDEN | TALLER),
            );
        }
        ROTATE_LEFT | ROTATE_RIGHT => {
            let degrees = if command == ROTATE_LEFT { -15.0 } else { 15.0 };
            state.canvas.borrow_mut().rotate_selected(degrees);
        }
        ENLARGE => super::resize_input::uniform(hwnd, state, 1.1),
        SHRINK => super::resize_input::uniform(hwnd, state, 1.0 / 1.1),
        REMOVE_LAST => {
            state.canvas.borrow_mut().delete_target();
        }
        APPLY_STYLE => {
            state.canvas.borrow_mut().restyle_target();
        }
        key => {
            if let Some(tool) = tool_for_command(key) {
                state.canvas.borrow_mut().select_tool(tool);
            }
        }
    }
    super::menu_transition::finish(hwnd, state)
}

unsafe fn label(menu: HMENU, id: usize, text: &str, flags: MENU_ITEM_FLAGS) -> Result<()> {
    let text = windows::core::HSTRING::from(text);
    AppendMenuW(menu, flags, id, windows::core::PCWSTR(text.as_ptr()))
}

#[cfg(test)]
#[path = "context_menu_tests.rs"]
mod tests;

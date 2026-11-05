//! 菜单映射、宿主消息路由与 Win32 句柄释放验证；不弹出交互菜单。
use super::*;

#[path = "context_menu_input_tests.rs"]
mod input;

#[test]
fn menu_target_rejects_replacement_even_with_same_revision_and_pixels() {
    let image = crate::desktop::image::Image {
        width: 1,
        height: 1,
        pixels: vec![0; 4].into(),
    };
    let mut canvas = crate::desktop::canvas::Canvas::new(Some(image.clone())).unwrap();
    let target = MenuTarget::capture(&canvas);
    assert!(target.matches(&canvas));
    canvas.replace_image(image).unwrap();
    assert_eq!(canvas.editor.revision(), 0);
    assert!(!target.matches(&canvas));
}

#[test]
fn menu_target_rejects_revision_tool_and_selection_changes() {
    let mut canvas = crate::desktop::canvas::Canvas::new(None).unwrap();
    for x in [1.0, 10.0] {
        canvas
            .editor
            .begin(Tool::Brush, crate::viewport::Point { x, y: 1.0 })
            .unwrap();
        canvas.editor.commit();
    }
    canvas.select_tool(Tool::Select);
    let revision = canvas.editor.revision();
    canvas.selected = Some((0, revision));
    let target = MenuTarget::capture(&canvas);
    canvas.viewport.origin.x = 40.0;
    assert!(target.matches(&canvas)); // 仅窗口呈现变化不使菜单目标失效。
    canvas.selected = Some((1, revision));
    assert!(!target.matches(&canvas));
    canvas.selected = Some((0, revision));
    canvas.multi_selection = Some((vec![0, 1], revision));
    assert!(!target.matches(&canvas));
    canvas.multi_selection = None;
    assert!(target.matches(&canvas));
    canvas.select_tool(Tool::Brush);
    assert!(!target.matches(&canvas));
    let target = MenuTarget::capture(&canvas);
    canvas.editor.undo();
    assert!(!target.matches(&canvas));
}

#[test]
fn menu_target_rejects_style_and_crop_mode_changes_without_revision_increment() {
    let mut canvas = crate::desktop::canvas::Canvas::new(None).unwrap();
    let target = MenuTarget::capture(&canvas);
    canvas.crop_mode = true;
    assert!(!target.matches(&canvas));
    canvas.crop_mode = false;
    assert!(target.matches(&canvas));
    let style = canvas
        .editor
        .style()
        .with_color(crate::stroke_style::Color(1, 2, 3));
    canvas.editor.set_style(style);
    assert_eq!(canvas.editor.revision(), 0);
    assert!(!target.matches(&canvas));
}
#[test]
fn command_mapping_does_not_depend_on_keyboard_modifier_state() {
    assert_eq!(tool_for_command(0x51), Some(Tool::RoundedRectangle));
    assert_eq!(tool_for_command(0x56), Some(Tool::Pan));
    assert_eq!(tool_for_command(EXPORT), None);
    assert_eq!(tool_for_command(0), None);
}
#[test]
fn native_menu_owns_items_and_releases_handle() {
    unsafe {
        let menu = Menu(CreatePopupMenu().unwrap());
        let handle = menu.0;
        label(handle, COLOR, "画笔颜色", MF_STRING).unwrap();
        assert_eq!(GetMenuItemCount(Some(handle)), 1);
        assert_eq!(GetMenuItemID(handle, 0), COLOR as u32);
        let mut text = [0u16; 32];
        let count = GetMenuStringW(handle, COLOR as u32, Some(&mut text), MF_BYCOMMAND);
        assert_eq!(
            String::from_utf16(&text[..count as usize]).unwrap(),
            "画笔颜色"
        );
        drop(menu);
        assert!(!IsMenu(handle).as_bool());
    }
}

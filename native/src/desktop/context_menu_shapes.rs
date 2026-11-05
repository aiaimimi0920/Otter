//! 自定义形状比例/步进的模态编辑事务；释放 Canvas 借用后才显示对话框，取消不写入。
use crate::desktop::{file_dialog, shape_dialog, State};
use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::IsWindow};

// false 表示模态准入被拒绝，调用者必须跳过菜单末尾同步，保留原拒绝语义。
pub(super) unsafe fn edit(hwnd: HWND, state: &State, ratio: bool) -> bool {
    let Some(_dialog) = file_dialog::Guard::begin(&state.file_dialog) else {
        return false;
    };
    let value = {
        let canvas = state.canvas.borrow();
        if ratio {
            canvas.tool_profiles.shapes.ratio_label().to_owned()
        } else {
            canvas.tool_profiles.shapes.step().to_string()
        }
    };
    if let Some(value) = shape_dialog::show(hwnd, value, ratio) {
        if IsWindow(Some(hwnd)).as_bool() {
            let mut canvas = state.canvas.borrow_mut();
            if ratio {
                canvas.tool_profiles.shapes.set_ratio(&value);
            } else {
                canvas.tool_profiles.shapes.set_step(&value);
            }
        }
    }
    true
}

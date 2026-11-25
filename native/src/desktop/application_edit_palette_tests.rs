//! 共享调色板在真实 Application/文档绑定后传播；不启动托盘、全局输入或桌面取色。
use super::*;
use crate::{desktop::Apartment, editor::Tool, stroke_style::Color, viewport::Point};
use neuro_mini_ui::Rgba;
#[test]
fn removal_propagates_defaults_across_documents_without_rewriting_selected_objects_or_new_apply() {
    unsafe {
        let _apartment = Apartment::new().unwrap();
        let instance = crate::desktop::workspace_window_fixture::register();
        let initial =
            DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
        let mut app = Application::new(instance, initial);
        let next = app
            .prepare_window(State::new(None, false).unwrap(), false)
            .unwrap();
        app.documents.push(next);
        let color = Color(16, 32, 48);
        for doc in &app.documents {
            let mut c = doc.state.canvas.borrow_mut();
            c.select_tool(Tool::Rectangle);
            let style = c.editor.style().with_color(color);
            c.editor.set_style(style);
            c.editor
                .begin(Tool::Line, Point { x: 1.0, y: 1.0 })
                .unwrap();
            c.editor.extend(Point { x: 20.0, y: 20.0 }).unwrap();
            assert!(c.editor.commit());
            c.select_tool(Tool::Select);
            c.selected = Some((0, c.editor.revision()));
        }
        let source = &app.documents[0].state;
        let palette = vec![Rgba::rgb(1, 2, 3)];
        app.edit_palette
            .borrow_mut()
            .record(source, palette.clone(), vec![Rgba::rgb(16, 32, 48)]);
        // “删除”之后再“应用同一颜色”应覆盖源默认值，外层结算不能把它重置第二次。
        {
            let mut c = source.canvas.borrow_mut();
            let style = c.editor.style().with_color(color);
            c.editor.set_style(style);
        }
        app.settle_edit_palette();
        assert_eq!(source.canvas.borrow().editor.style().color(), color);
        let second = &app.documents[1].state;
        assert_eq!(
            second.canvas.borrow().editor.style().color(),
            crate::stroke_style::StrokeStyle::default().color()
        );
        for doc in &app.documents {
            assert_eq!(doc.state.canvas.borrow().editor.elements()[0].color, color);
            assert_eq!(doc.state.canvas.borrow().selected_index(), Some(0));
            assert_eq!(doc.state.edit_palette.borrow().colors, palette);
        }
        let new_state = app.bind_window_state(State::new(None, false).unwrap());
        assert!(std::rc::Rc::ptr_eq(
            &new_state.edit_palette,
            &app.edit_palette
        ));
        assert_eq!(new_state.edit_palette.borrow().colors, palette);
        app.settle_edit_palette();
        assert_eq!(source.canvas.borrow().editor.style().color(), color);
    }
}

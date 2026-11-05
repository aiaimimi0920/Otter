//! 复用隐藏 HWND/串行夹具，不启动 Live；CPU 编辑→真实保存 worker→文件恢复→CPU 再呈现。
use super::*;
use crate::{
    desktop::{context_menu, image::Image, render_commands},
    editor::Tool,
    viewport::Point,
};

#[test]
fn cpu_edit_undo_save_file_restore_and_repaint_preserve_pixels_and_vectors() {
    unsafe {
        let environment = Environment::new();
        let pixels = [30, 60, 90, 255].repeat(64);
        let image = Image {
            width: 8,
            height: 8,
            pixels: pixels.clone().into(),
        };
        let mut app = Application::new(
            environment.instance,
            DocumentWindow::create(
                environment.instance,
                State::new(Some(image), false).unwrap(),
                false,
            )
            .unwrap(),
        );
        let doc = &app.documents[0];
        let id = doc.state.unit_id.clone();
        context_menu::execute(doc.window.0, &doc.state, render_commands::SOFTWARE).unwrap();
        {
            let mut canvas = doc.state.canvas.borrow_mut();
            canvas.editor.set_style(
                crate::stroke_style::StrokeStyle::default()
                    .with_opacity(0.5)
                    .unwrap()
                    .with_paint_alpha(crate::stroke_style::PaintAlpha::new(0.25, 1.0).unwrap()),
            );
            canvas
                .editor
                .begin(Tool::Brush, Point { x: 1.0, y: 1.0 })
                .unwrap();
            canvas.editor.extend(Point { x: 6.0, y: 3.0 }).unwrap();
            assert!(canvas.editor.commit());
            assert!(canvas.editor.undo());
            assert!(canvas.editor.elements().is_empty());
            assert!(canvas.editor.redo());
            assert_eq!(canvas.editor.elements().len(), 1);
        }
        SendMessageW(doc.window.0, WM_PAINT, None, None);
        assert!(doc.state.error.borrow().is_none());
        assert!(doc.state.canvas.borrow().software_selected());
        assert!(doc.state.canvas.borrow().confirmed_paints > 0);
        let stroke = doc.state.canvas.borrow().editor.elements()[0].clone();
        let viewport = doc.state.canvas.borrow().viewport;
        let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(doc.window.0);
        let path = save_publish::smoke_destination().unwrap();
        app.request_save_new(path.clone()).unwrap();
        let output = save::prepared(&mut app).unwrap();
        let expected = output.candidate.bytes.clone();
        output
            .commit_ready(&mut app, &AtomicBool::new(false))
            .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), expected.as_ref());
        assert_eq!(app.save_status(), save_state::Status::Unchanged);
        // 保存合同是 Editor→窗口 DIP→物理像素→会话逻辑坐标，不要求恢复 Editor 使用原始坐标框。
        let space = app.restored_session.as_ref().unwrap().space.unwrap();
        let factor = f64::from(dpi) / 96.0 / space.scale;
        let scale = (f64::from(viewport.scale) * factor) as f32;
        let origin = Point {
            x: (f64::from(viewport.origin.x) * factor) as f32,
            y: (f64::from(viewport.origin.y) * factor) as f32,
        };
        let points: Vec<_> = stroke
            .points
            .iter()
            .map(|point| Point {
                x: point.x * scale + origin.x,
                y: point.y * scale + origin.y,
            })
            .collect();
        app.request_restore_input(
            crate::desktop::session_job::Input::File(path.clone()),
            None,
            None::<crate::session_placement::Space>,
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while app.restore_job.is_some() {
            app.process_restore(false).unwrap();
            assert!(Instant::now() < deadline, "CPU saved file restore timeout");
            if app.restore_job.is_some() {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        let restored = app
            .documents
            .iter()
            .find(|doc| doc.state.unit_id == id)
            .unwrap();
        {
            let canvas = restored.state.canvas.borrow();
            assert_eq!(canvas.editor.elements().len(), 1);
            let restored_stroke = &canvas.editor.elements()[0];
            assert_eq!(restored_stroke.tool, stroke.tool);
            assert_eq!(restored_stroke.points, points);
            assert_eq!(restored_stroke.width, stroke.width * scale);
            assert_eq!(restored_stroke.color, stroke.color);
            assert_eq!(restored_stroke.opacity, stroke.opacity);
            assert_eq!(restored_stroke.fill, stroke.fill);
            assert_eq!(restored_stroke.paint_alpha, stroke.paint_alpha);
            assert_eq!(restored_stroke.dash, stroke.dash);
            assert_eq!(restored_stroke.corner_radius, stroke.corner_radius * scale);
            assert_eq!(restored_stroke.sides, stroke.sides);
            assert_eq!(restored_stroke.arrow, stroke.arrow);
            // 恢复的 canvas.image 是投影到窗口逻辑框的编辑底图；原始资源由绑定当前文档的 Source 保管。
            let source = canvas.minified_source.as_ref().unwrap();
            assert!(source.matches(&canvas.editor));
            let image = source.image();
            assert_eq!((image.width, image.height), (8, 8));
            assert_eq!(&*image.pixels, pixels.as_slice());
        }
        context_menu::execute(
            restored.window.0,
            &restored.state,
            render_commands::SOFTWARE,
        )
        .unwrap();
        SendMessageW(restored.window.0, WM_PAINT, None, None);
        assert!(restored.state.error.borrow().is_none());
        assert!(restored.state.canvas.borrow().software_selected());
        assert!(restored.state.canvas.borrow().confirmed_paints > 0);
        assert!(!IsWindowVisible(restored.window.0).as_bool());
        // 独占 linshi GUID 文件保留为业务回执，不触碰真实 profile 或用户文档。
        println!("native-cpu-save-test: file={} pixels=true vectors=true undo=true redo=true restored=true", path.display());
    }
}

//! Verify immediate EDIT appearance and keyed draft interaction.
use super::*;

#[test]
fn draft_and_commit_use_selected_fill_ink_size_and_family() {
    use crate::stroke_style::Color;
    unsafe {
        let f = Fixture::new();
        {
            let mut canvas = f.state.canvas.borrow_mut();
            assert!(canvas.editor.style().fill().is_none());
            let style = canvas
                .editor
                .style()
                .with_color(Color(13, 65, 210))
                .with_fill(Some(Color(240, 221, 115)));
            canvas.editor.set_style(style);
            canvas.tool_profiles.text_font =
                crate::text_annotation::TextFont::new("Consolas", 31.0, true).unwrap();
        }
        let (popup, edit) = f.start();
        let bitmap = crate::software_dib::Bitmap::new(
            windows::Win32::Graphics::Direct2D::Common::D2D_SIZE_U {
                width: 8,
                height: 8,
            },
        )
        .unwrap();
        let dc = bitmap.dc();
        SendMessageW(
            popup,
            WM_CTLCOLOREDIT,
            Some(WPARAM(dc.0 as usize)),
            Some(LPARAM(edit.0 as isize)),
        );
        assert_eq!(GetTextColor(dc), COLORREF(0xd2410d));
        assert_eq!(GetBkColor(dc), COLORREF(0x73ddf0));
        let font = SendMessageW(edit, WM_GETFONT, None, None);
        let mut log = LOGFONTW::default();
        assert!(
            GetObjectW(
                HGDIOBJ(font.0 as *mut _),
                std::mem::size_of::<LOGFONTW>() as i32,
                Some((&mut log as *mut LOGFONTW).cast())
            ) > 0
        );
        assert_eq!(log.lfWeight, FW_BOLD.0 as i32);
        assert_eq!(
            log.lfHeight,
            -(31.0 * GetDpiForWindow(f.hwnd).max(96) as f32 / 96.0).round() as i32
        );
        assert_eq!(String::from_utf16_lossy(&log.lfFaceName[..8]), "Consolas");
        SetWindowTextW(edit, w!("即时样式")).unwrap();
        enter(edit);
        let canvas = f.state.canvas.borrow();
        let stroke = &canvas.editor.elements()[0];
        assert_eq!(stroke.fill, Some(Color(240, 221, 115)));
        assert_eq!(stroke.color, Color(13, 65, 210));
        assert_eq!(stroke.text.as_ref().unwrap().font_size(), 31.0);
    }
}
#[test]
fn transparent_draft_is_color_keyed_and_owned_background_click_does_not_submit() {
    unsafe {
        let f = Fixture::new();
        let (popup, edit) = f.start();
        assert_ne!(
            GetWindowLongW(popup, GWL_EXSTYLE) as u32 & WS_EX_LAYERED.0,
            0
        );
        let mut key = COLORREF::default();
        let mut flags = LAYERED_WINDOW_ATTRIBUTES_FLAGS::default();
        GetLayeredWindowAttributes(popup, Some(&mut key), None, Some(&mut flags)).unwrap();
        assert!(flags.contains(LWA_COLORKEY));
        let mut pt = POINT { x: 10, y: 10 };
        ClientToScreen(edit, &mut pt).ok().unwrap();
        assert!(before_message(
            f.hwnd,
            &f.state,
            &MSG {
                hwnd: f.hwnd,
                message: WM_LBUTTONDOWN,
                pt,
                ..Default::default()
            }
        ));
        SendMessageW(edit, WM_LBUTTONUP, None, None);
        assert_eq!(GetFocus(), edit);
        assert!(f.state.inline_text.borrow().is_some());
        assert!(f.state.canvas.borrow().editor.elements().is_empty());
    }
}

#[test]
fn reediting_translucent_fill_hides_committed_text_and_restores_it_on_cancel() {
    use crate::stroke_style::{Color, PaintAlpha};
    unsafe {
        let f = Fixture::new();
        {
            let mut canvas = f.state.canvas.borrow_mut();
            let style = canvas
                .editor
                .style()
                .with_fill(Some(Color(21, 61, 101)))
                .with_paint_alpha(PaintAlpha::new(1.0, 128.0 / 255.0).unwrap());
            canvas.editor.set_style(style);
        }
        let (_, edit) = f.start();
        SetWindowTextW(edit, w!("底色")).unwrap();
        enter(edit);
        {
            let mut canvas = f.state.canvas.borrow_mut();
            canvas.selected = Some((0, canvas.editor.revision()));
        }
        super::super::super::edit_selected(f.hwnd, &f.state);
        let (popup, _) = f.handles();
        assert_eq!(f.state.canvas.borrow().inline_text_hidden, Some(0));
        let backing = GetWindow(popup, GW_HWNDNEXT).unwrap();
        assert_eq!(GetWindow(backing, GW_OWNER).unwrap(), f.hwnd);
        let mut alpha = 0;
        let mut flags = LAYERED_WINDOW_ATTRIBUTES_FLAGS::default();
        GetLayeredWindowAttributes(backing, None, Some(&mut alpha), Some(&mut flags)).unwrap();
        assert_eq!(alpha, 128);
        assert_eq!(flags, LWA_ALPHA);
        finish(f.hwnd, &f.state, false).unwrap();
        let canvas = f.state.canvas.borrow();
        assert_eq!(canvas.inline_text_hidden, None);
        assert_eq!(canvas.editor.elements()[0].fill, Some(Color(21, 61, 101)));
        assert!(!IsWindow(Some(backing)).as_bool());
    }
}

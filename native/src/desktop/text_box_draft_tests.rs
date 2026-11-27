//! Use a private desktop, real EDIT and a memory DC. Neither the interactive
//! desktop nor the clipboard is touched. A click oracle comes from export's
//! own layout, not the draft controller's selection or its line table.
use super::*;
use crate::{stroke_style::Color, text_annotation::TextBox};
use windows::Win32::Graphics::{Direct2D::Common::D2D_SIZE_U, DirectWrite::*};

#[test]
fn boxed_draft_wrap_pixels_cluster_click_crlf_undo_and_committed_export_agree() {
    desktop::workspace_window_fixture::with_private_input_desktop(|| unsafe {
        let f = Fixture::new();
        let geometry = TextBox::new(113., 5.).unwrap();
        let font = crate::text_annotation::TextFont::new("Segoe UI", 15., false).unwrap();
        {
            let mut c = f.state.canvas.borrow_mut();
            c.tool_profiles.text_box = Some(geometry);
            c.tool_profiles.text_font = font.clone();
            let style = c
                .editor
                .style()
                .with_color(Color(208, 34, 21))
                .with_fill(Some(Color(255, 255, 255)));
            c.editor.set_style(style);
        }
        let (_, edit) = f.start();
        let cover = GetWindow(edit, GW_CHILD).expect("boxed draft has a visible layout");
        let source = "cat 中文 🦦\r\nabcdefghijklmnopqrstuvwxzy\r\n  🦦 fin";
        SetWindowTextW(edit, &HSTRING::from(source)).unwrap();
        let text = TextAnnotation::new(source, font.family(), font.font_size(), false)
            .unwrap()
            .with_geometry(Some(geometry));
        let layout = desktop::annotation_text::layout(&text, 1., 113., 2048.).unwrap();
        let mut lines = [DWRITE_LINE_METRICS::default(); 16];
        let mut count = 0;
        layout.GetLineMetrics(Some(&mut lines), &mut count).unwrap();
        assert!(count > 3);
        let lines = &lines[..count as usize];
        let scale = GetDpiForWindow(f.hwnd).max(96) as f32 / 96.;
        let mut rect = RECT::default();
        GetClientRect(cover, &mut rect).unwrap();
        let bitmap = crate::software_dib::Bitmap::new(D2D_SIZE_U {
            width: rect.right as u32,
            height: rect.bottom as u32,
        })
        .unwrap();
        SendMessageW(
            cover,
            WM_PRINTCLIENT,
            Some(WPARAM(bitmap.dc().0 as usize)),
            None,
        );
        GdiFlush().ok().unwrap();
        let pixels = bitmap.pixels();
        let red = |p: &[u8]| {
            p[2] > 130 && p[2].saturating_sub(p[1]) > 65 && p[2].saturating_sub(p[0]) > 65
        };
        let mut start = 0.;
        for line in lines {
            let (top, bottom) = (
                ((start + 5.) * scale).ceil() as usize,
                ((start + 5. + line.height) * scale).floor() as usize,
            );
            assert!(
                (top..bottom).any(|y| pixels
                    [y * rect.right as usize..(y + 1) * rect.right as usize]
                    .iter()
                    .any(|p| red(p))),
                "no rendered draft ink on row {start}"
            );
            start += line.height;
        }
        let model_index = text
            .content()
            .rsplit_once('🦦')
            .unwrap()
            .0
            .encode_utf16()
            .count() as u32;
        let native_index = source.rsplit_once('🦦').unwrap().0.encode_utf16().count() as u32;
        let mut hit = DWRITE_HIT_TEST_METRICS::default();
        let (mut x, mut y) = (0., 0.);
        layout
            .HitTestTextPosition(model_index, false, &mut x, &mut y, &mut hit)
            .unwrap();
        let x = ((x + geometry.padding() + hit.width * 0.15) * scale).round() as u16;
        let y = ((y + geometry.padding() + hit.height * 0.5) * scale).round() as u16;
        let pt = LPARAM((u32::from(x) | (u32::from(y) << 16)) as isize);
        SendMessageW(edit, WM_LBUTTONDOWN, Some(WPARAM(1)), Some(pt));
        SendMessageW(edit, WM_LBUTTONUP, None, Some(pt));
        let selection = || {
            let (mut a, mut b) = (0u32, 0u32);
            SendMessageW(
                edit,
                EM_GETSEL,
                Some(WPARAM(&mut a as *mut _ as usize)),
                Some(LPARAM(&mut b as *mut _ as isize)),
            );
            (a, b)
        };
        assert_eq!(selection(), (native_index, native_index));
        SendMessageW(edit, WM_KEYDOWN, Some(WPARAM(VK_RIGHT.0 as usize)), None);
        assert_eq!(selection(), (native_index + 2, native_index + 2));
        SendMessageW(
            edit,
            EM_REPLACESEL,
            Some(WPARAM(1)),
            Some(LPARAM(w!("X").as_ptr() as isize)),
        );
        assert_ne!(SendMessageW(edit, EM_UNDO, None, None).0, 0);
        finish(f.hwnd, &f.state, true).unwrap();
        let c = f.state.canvas.borrow();
        let s = &c.editor.elements()[0];
        assert_eq!(s.text.as_ref(), Some(&text));
        let image = desktop::display_composition::compose(
            c.image.as_ref().unwrap(),
            &c.editor,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap();
        let stride = image.width as usize * 4;
        let mut start = s.points[0].y + geometry.padding();
        for line in lines {
            assert!(
                (start.ceil() as usize..(start + line.height).floor() as usize)
                    .any(|y| image.pixels[y * stride..(y + 1) * stride]
                        .chunks_exact(4)
                        .any(red)),
                "no committed ink on row {start}"
            );
            start += line.height;
        }
    });
}

#[test]
fn boxed_draft_mixed_newlines_and_rejected_commit_keep_the_same_edit_and_undo_buffer() {
    desktop::workspace_window_fixture::with_private_input_desktop(|| unsafe {
        // Production starts COM before constructing the first pin. Release its
        // thread-owned windows before detaching this test's private desktop.
        let _apartment = desktop::Apartment::new().unwrap();
        let f = Fixture::new();
        f.state.canvas.borrow_mut().tool_profiles.text_box = TextBox::new(136., 6.);
        let (_, edit) = f.start();
        let initial = "a\nb\rc\r\n🦦";
        SetWindowTextW(edit, &HSTRING::from(initial)).unwrap();
        let session = f.state.inline_text.borrow().clone().unwrap();
        let (normalized, pairs) = session.ui.draft(true).unwrap();
        assert_eq!(normalized, "a\nb\nc\n🦦");
        assert_eq!(pairs, vec![(5, 5)]);
        let revision = f.state.canvas.borrow().editor.revision();
        let over = "x".repeat(crate::text_annotation::MAX_TEXT_UNITS + 1);
        SetWindowTextW(edit, &HSTRING::from(&over)).unwrap();
        assert!(finish(f.hwnd, &f.state, true).is_err());
        assert!(f
            .state
            .inline_text
            .borrow()
            .as_ref()
            .is_some_and(|s| Rc::ptr_eq(s, &session)));
        assert_eq!(GetWindowTextLengthW(edit) as usize, over.len());
        assert_eq!(f.state.canvas.borrow().editor.revision(), revision);
        SetWindowTextW(edit, w!("修正后的文字 🦦")).unwrap();
        finish(f.hwnd, &f.state, true).unwrap();
        let mut c = f.state.canvas.borrow_mut();
        assert_eq!(
            c.editor.elements()[0].text.as_ref().unwrap().content(),
            "修正后的文字 🦦"
        );
        assert!(c.editor.undo());
        assert!(c.editor.elements().is_empty());
        drop(c);
        drop(session);
        drop(f);
        assert!(!IsWindow(Some(edit)).as_bool());
    });
}

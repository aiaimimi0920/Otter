//! Build an editable text card only in the bounded open worker. Image pixels and
//! annotation content keep independent leases; no clipboard or HWND is retained.
use super::{
    annotation_text, clipboard_editable::Content, image::Image, open_image::PreparedDocument,
};
use crate::{
    editor::Editor,
    stroke_style::Color,
    text_annotation::{TextAnnotation, TextBox},
    viewport::Point,
};
use windows::{
    core::{Error, Result, HSTRING},
    Win32::Foundation::*,
};

fn failure() -> Error {
    Error::new(E_INVALIDARG, crate::ui_text::text("paste-card-budget"))
}
fn code_hint(value: &str) -> bool {
    value.lines().any(|s| {
        s.starts_with([' ', '\t'])
            || s.ends_with([';', '{', '}'])
            || ["fn ", "def ", "import ", "#include ", "function "]
                .iter()
                .any(|prefix| s.starts_with(prefix))
    })
}
unsafe fn family(code: bool) -> Result<&'static str> {
    if code {
        let mut fonts = None;
        annotation_text::shared_factory()?.GetSystemFontCollection(&mut fonts, false)?;
        let fonts = fonts.ok_or_else(failure)?;
        for name in ["Cascadia Mono", "Consolas"] {
            let (mut index, mut installed) = (0, Default::default());
            fonts.FindFamilyName(&HSTRING::from(name), &mut index, &mut installed)?;
            if installed.as_bool() {
                return Ok(name);
            }
        }
    }
    Ok("Segoe UI")
}
pub(super) fn prepare(units: Vec<u16>, stopped: impl Fn() -> bool) -> Result<PreparedDocument> {
    let value = super::paste_input::text::normalize(&units)?;
    drop(units);
    let code = code_hint(&value);
    let plain =
        TextAnnotation::new(&value, unsafe { family(code)? }, 16.0, false).ok_or_else(failure)?;
    drop(value);
    let natural = unsafe { annotation_text::measure(&plain)? };
    let geometry = TextBox::new((natural.x + 16.0).clamp(96.0, 560.0), 12.0).ok_or_else(failure)?;
    let text = plain.with_geometry(Some(geometry));
    let size = unsafe { annotation_text::measure(&text)? };
    if !size.x.is_finite() || !size.y.is_finite() {
        return Err(failure());
    }
    let (width, height) = (
        (size.x.ceil() as u32).saturating_add(16),
        (size.y.ceil() as u32).saturating_add(16),
    );
    let bytes = crate::viewport::checked_image_bytes(width, height).ok_or_else(failure)?;
    let mut lease = crate::pixels::reserve(bytes).ok_or_else(failure)?;
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(bytes).map_err(|_| failure())?;
    if !lease.grow_to(pixels.capacity()) {
        return Err(failure());
    }
    let (bg, fg, panel) = if code {
        ([43, 36, 30, 255], Color(235, 241, 248), Color(39, 47, 55))
    } else {
        (
            [235, 242, 245, 255],
            Color(39, 49, 60),
            Color(235, 241, 248),
        )
    };
    for _ in 0..height {
        if stopped() {
            return Err(Error::new(E_ABORT, "Text card cancelled"));
        }
        for _ in 0..width {
            pixels.extend_from_slice(&bg);
        }
    }
    let mut editor = Editor::default();
    let style = editor
        .style()
        .with_color(fg)
        .with_fill(Some(panel))
        .with_corner_radius_value(8.0)
        .expect("constant radius");
    editor
        .insert_text_with_style(Point { x: 8.0, y: 8.0 }, size, text, style)
        .map_err(|_| failure())?
        .ok_or_else(failure)?;
    Content {
        image: Image {
            width,
            height,
            pixels: crate::pixels::Pixels::from_reserved(pixels, lease),
        },
        editor,
        opacity: 1.0,
        frame_style: Default::default(),
        capture_source: None,
        origin: None,
    }
    .prepare(stopped)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn card_roundtrips_box_and_edits_without_flattening_or_entering_capture_history() {
        let _apt = super::super::Apartment::new().unwrap();
        let value = format!("\tfn 你好() {{\r\n{}\r\n}}\r\n🦦", "longUrl".repeat(80));
        let ready = prepare(value.encode_utf16().collect(), || false).unwrap();
        assert!(!ready.record_history && !ready.ordinary_capture);
        assert_eq!(ready.image.width, 576);
        assert_eq!(&ready.image.pixels[..4], &[43, 36, 30, 255]);
        let editor = ready.editable.unwrap().editor;
        let stroke = &editor.elements()[0];
        assert_eq!(stroke.tool, crate::editor::Tool::Text);
        let text = stroke.text.as_ref().unwrap();
        assert_eq!(text.content(), value.replace("\r\n", "\n"));
        assert!(stroke.points[1].y - stroke.points[0].y > 5.0 * text.font_size());
        let saved = crate::session_vector_write::encode(&editor, None, |t| {
            Ok(annotation_text::measure_flip_metrics(t)
                .unwrap()
                .baseline_from_top)
        })
        .unwrap();
        let raw: Box<serde_json::value::RawValue> = serde_json::from_slice(&saved).unwrap();
        let restored = crate::session_vectors::restore_with_text(Some(&raw), |t| {
            Ok(crate::session_vectors::TextMetrics {
                size: unsafe { annotation_text::measure(t).unwrap() },
                baseline: annotation_text::measure_flip_metrics(t)
                    .unwrap()
                    .baseline_from_top,
            })
        })
        .unwrap();
        let other = &restored.editor.elements()[0];
        assert_eq!(other.text, stroke.text);
        assert_eq!(other.points, stroke.points);
        assert_eq!(other.fill, stroke.fill);
        assert!(prepare(vec![65; 100], || true).is_err());
    }
}

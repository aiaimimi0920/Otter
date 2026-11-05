//! 主干默认 PNG 名称：本地日期/百分之一秒/实际导出尺寸；仅在显式导出时取时钟。
use super::{display_snapshot, State};
use windows::Win32::{Foundation::SYSTEMTIME, System::SystemInformation::GetLocalTime};

pub(super) fn suggested(state: &State) -> Option<String> {
    suggested_for(state, Pattern::Save)
}
pub(super) fn suggested_clipboard(state: &State) -> Option<String> {
    suggested_for(state, Pattern::Clipboard)
}
pub(super) fn suggested_drag(state: &State) -> Option<String> {
    suggested_for(state, Pattern::Drag)
}
#[derive(Clone, Copy)]
enum Pattern {
    Save,
    Clipboard,
    Drag,
}
fn suggested_for(state: &State, pattern: Pattern) -> Option<String> {
    let dimensions = display_snapshot::dimensions(
        &state.canvas.borrow(),
        state.resolved_display.borrow().as_ref(),
    )?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or(0)
        .to_string();
    super::naming_context::with_context(state, dimensions, |context| {
        render_pattern(
            unsafe { GetLocalTime() },
            &state.naming.borrow(),
            context,
            &timestamp,
            pattern,
        )
    })?
}
#[cfg(test)]
fn format_name(time: SYSTEMTIME, (width, height): (u32, u32)) -> String {
    render_name(
        time,
        &Default::default(),
        &crate::file_naming::Context {
            width: Some(width),
            height: Some(height),
            ..Default::default()
        },
        "",
    )
    .unwrap()
}
#[cfg(test)]
fn render_name(
    time: SYSTEMTIME,
    settings: &crate::file_naming_settings::Settings,
    context: &crate::file_naming::Context<'_>,
    timestamp: &str,
) -> Option<String> {
    render_pattern(time, settings, context, timestamp, Pattern::Save)
}
fn render_pattern(
    time: SYSTEMTIME,
    settings: &crate::file_naming_settings::Settings,
    context: &crate::file_naming::Context<'_>,
    timestamp: &str,
    pattern: Pattern,
) -> Option<String> {
    let date = format!("{:04}{:02}{:02}", time.wYear, time.wMonth, time.wDay);
    let clock = format!(
        "{:02}{:02}{:02}{:02}",
        time.wHour,
        time.wMinute,
        time.wSecond,
        time.wMilliseconds / 10
    );
    let moment = crate::file_naming::Moment {
        date: &date,
        time: &clock,
        timestamp,
    };
    let stem = crate::file_naming::render(
        match pattern {
            Pattern::Clipboard => &settings.clipboard_file_pattern,
            Pattern::Save => &settings.sticker_save_pattern,
            Pattern::Drag => &settings.drag_export_pattern,
        },
        context,
        &moment,
        settings.title_max_length,
    )
    .ok()?;
    Some(format!("{stem}.png"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runtime_pattern_receives_identity_dimensions_and_timestamp() {
        let settings = crate::file_naming_settings::Settings {
            sticker_save_pattern: "{kind}_{shortId}_{width}x{height}_{timestamp}".into(),
            ..Default::default()
        };
        assert_eq!(
            render_name(
                SYSTEMTIME::default(),
                &settings,
                &crate::file_naming::Context {
                    unit_id: "unit-1234",
                    kind: "art",
                    width: Some(2),
                    height: Some(3),
                    ..Default::default()
                },
                "99"
            ),
            Some("art_1234_2x3_99.png".into())
        );
    }
    #[test]
    fn suggested_dimensions_follow_committed_crop_not_viewport() {
        let image = super::super::image::Image {
            width: 4,
            height: 4,
            pixels: vec![255; 64].into(),
        };
        let state = State::new(Some(image.clone()), false).unwrap();
        let mut editor = crate::editor::Editor::with_layers(crate::erase_layers::Layers {
            size: (4, 4),
            base: image.pixels.clone(),
            annotation_size: None,
            annotations: None,
        })
        .unwrap();
        editor
            .crop_to(crate::crop_geometry::Rect {
                x: 1.0,
                y: 1.0,
                width: 2.0,
                height: 2.0,
            })
            .unwrap();
        {
            let mut canvas = state.canvas.borrow_mut();
            canvas.editor = editor;
            canvas.viewport.scale = 3.0;
            canvas.viewport.origin = crate::viewport::Point { x: -100.0, y: 50.0 };
        }
        assert!(suggested(&state).unwrap().ends_with("_2x2.png"));
        assert_eq!(
            super::super::export::dimensions(&image, Some(&state.canvas.borrow().editor)).unwrap(),
            (2, 2)
        );
    }
    #[test]
    fn default_name_matches_main_pattern_and_centisecond_precision() {
        let time = SYSTEMTIME {
            wYear: 2026,
            wMonth: 10,
            wDay: 18,
            wHour: 3,
            wMinute: 4,
            wSecond: 5,
            wMilliseconds: 987,
            ..Default::default()
        };
        assert_eq!(
            format_name(time, (320, 240)),
            "Otter_20261018_03040598_320x240.png"
        );
        let time = SYSTEMTIME {
            wYear: 2026,
            wMonth: 1,
            wDay: 2,
            wHour: 0,
            wMinute: 0,
            wSecond: 0,
            wMilliseconds: 9,
            ..Default::default()
        };
        assert_eq!(
            format_name(time, (1, 8192)),
            "Otter_20260102_00000000_1x8192.png"
        );
    }
}

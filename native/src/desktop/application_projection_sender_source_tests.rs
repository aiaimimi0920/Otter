//! retained natural source 的生产菜单/TLS 像素回归与弱身份围栏；不占前台。
use super::*;
use crate::{desktop::canvas::minified_source::Source, erase_layers::Mode, viewport::Point};
#[path = "application_projection_sender_png_blur_tests.rs"]
mod png_blur;
#[path = "application_projection_sender_png_cross_crop_tests.rs"]
mod png_cross;
#[path = "application_projection_sender_png_entry_tests.rs"]
mod png_entry;
#[path = "application_projection_sender_png_legacy_tests.rs"]
mod png_legacy;
#[path = "application_projection_sender_png_legacy_owner_tests.rs"]
mod png_legacy_owner;

#[derive(Clone, Copy)]
pub(super) enum Case {
    Current,
    Natural,
    Erased,
    Undone,
    Cropped,
    TargetResolved,
    TargetBeautified,
    TargetLive,
    TargetMini,
    ArtLive,
    NaturalNonuniform,
    NaturalSavedLayout,
    CrossNaturalHistory,
    NaturalLegacyCrop,
    CrossNaturalCrop,
    CrossNaturalReset,
    CrossNaturalCropUndo,
    NaturalPropagatedBlur,
    NaturalLegacyReset,
    NaturalLegacyUndo,
    NaturalLegacySaveReopen,
    NaturalLegacyResetSaveReopen,
    NaturalLegacyNested,
    NaturalLegacyFlipped,
    WorkingLegacy,
    WorkingLegacySaveReopen,
}
fn image(width: u32, height: u32, color: [u8; 4]) -> Image {
    Image {
        width,
        height,
        pixels: color.repeat(width as usize * height as usize).into(),
    }
}
impl Case {
    pub(super) fn pixel_tolerance(self) -> u8 {
        if matches!(self, Self::NaturalPropagatedBlur) {
            2 // 独立f64 Gaussian及8%白wash与D2D/WIC的逐阶段整数舍入。
        } else {
            0
        }
    }
    pub(super) unsafe fn install(self, app: &mut Application) -> Image {
        if matches!(
            self,
            Self::NaturalLegacyNested
                | Self::NaturalLegacyFlipped
                | Self::WorkingLegacy
                | Self::WorkingLegacySaveReopen
        ) {
            return png_legacy_owner::install(app, self);
        }
        if matches!(
            self,
            Self::NaturalLegacyCrop
                | Self::NaturalLegacyReset
                | Self::NaturalLegacyUndo
                | Self::NaturalLegacySaveReopen
                | Self::NaturalLegacyResetSaveReopen
        ) {
            return png_legacy::install(app, self);
        }
        if matches!(self, Self::NaturalPropagatedBlur) {
            return png_blur::install(app);
        }
        if matches!(
            self,
            Self::CrossNaturalCrop | Self::CrossNaturalReset | Self::CrossNaturalCropUndo
        ) {
            return png_cross::install(app, self);
        }
        if matches!(
            self,
            Self::NaturalNonuniform | Self::NaturalSavedLayout | Self::CrossNaturalHistory
        ) {
            return png_entry::install(app, self);
        }
        if matches!(self, Self::TargetLive | Self::TargetMini | Self::ArtLive) {
            return geometry::install(app, self);
        }
        if matches!(self, Self::TargetResolved | Self::TargetBeautified) {
            return target::install(app, matches!(self, Self::TargetBeautified));
        }
        let mut canvas = app.documents[0].state.canvas.borrow_mut();
        if matches!(self, Self::Current) {
            return canvas.image.as_ref().unwrap().clone();
        }
        canvas
            .replace_image(image(32, 16, [0, 255, 0, 255]))
            .unwrap();
        let natural = image(128, 64, [0, 0, 255, 255]);
        canvas.minified_source = Some(Source::new(natural.clone(), &canvas.editor));
        if matches!(self, Self::Erased | Self::Undone) {
            let before = canvas.editor.layers().unwrap().clone();
            let after = before
                .erase(Mode::Content, &[Point { x: 16.0, y: 8.0 }], 8.0, || false)
                .unwrap();
            let revision = canvas.editor.revision();
            canvas
                .editor
                .replace_layers(revision, &before, after)
                .unwrap();
            assert!(!canvas
                .minified_source
                .as_ref()
                .unwrap()
                .matches(&canvas.editor));
            if matches!(self, Self::Erased) {
                let layers = canvas.editor.layers().unwrap();
                assert_eq!(layers.base[(8 * 32 + 16) * 4 + 3], 0);
                // 宿主和 retained 原图都仍存在，但只有新底图是权威像素。
                assert_eq!(
                    canvas.image.as_ref().unwrap().pixels[(8 * 32 + 16) * 4 + 3],
                    255
                );
                return Image {
                    width: 32,
                    height: 16,
                    pixels: layers.base.clone(),
                };
            }
            assert!(canvas.editor.undo());
            assert!(canvas
                .minified_source
                .as_ref()
                .unwrap()
                .matches(&canvas.editor));
        }
        if matches!(self, Self::Cropped) {
            canvas
                .editor
                .crop_to(crate::crop_geometry::Rect {
                    x: 8.0,
                    y: 4.0,
                    width: 16.0,
                    height: 8.0,
                })
                .unwrap();
            return image(64, 32, [0, 0, 255, 255]);
        }
        natural
    }
}
#[test]
fn native_sender_menu_transports_retained_natural_pixels_and_dimensions() {
    tls::run_source(0, Case::Natural);
}
#[test]
fn native_sender_menu_content_erase_cannot_resurrect_retained_source_or_stale_host() {
    tls::run_source(0, Case::Erased);
}
#[test]
fn native_sender_menu_undo_recovers_matching_natural_source() {
    tls::run_source(0, Case::Undone);
}
#[test]
fn native_sender_menu_crop_uses_retained_source_density_not_preview_dimensions() {
    tls::run_source(0, Case::Cropped);
}
#[test]
fn native_sender_retained_replacement_without_editor_revision_invalidates_scope_without_retaining_pixels(
) {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (files, _, _) = Files::new();
    unsafe {
        let (_class, mut app) = application(files.root());
        drop(Case::Natural.install(&mut app));
        let unit = app.documents[0].state.unit_id.clone();
        let (guard, snapshot) = scope::Scope::capture(&app, &unit).unwrap();
        let weak = snapshot.image.pixels.downgrade();
        assert!(guard.matches(&app));
        drop(snapshot);
        let mut canvas = app.documents[0].state.canvas.borrow_mut();
        let revision = canvas.editor.revision();
        canvas.minified_source = Some(Source::new(
            image(128, 64, [0, 0, 255, 255]),
            &canvas.editor,
        ));
        assert_eq!(canvas.editor.revision(), revision);
        drop(canvas);
        assert!(
            !guard.matches(&app),
            "equal pixels in a new allocation are not the captured source"
        );
        assert!(
            weak.upgrade().is_none(),
            "pending scope must not retain decoded image budget"
        );
        app.shutdown();
    }
}
#[test]
fn native_sender_resolved_frame_never_borrows_target_retained_source() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (files, _, _) = Files::new();
    unsafe {
        let (_class, mut app) = application(files.root());
        drop(Case::Natural.install(&mut app));
        let unit = app.documents[0].state.unit_id.clone();
        let resolved = image(8, 4, [255, 0, 0, 255]);
        *app.documents[0].state.resolved_display.borrow_mut() = Some(
            crate::desktop::display_frame::Frame::capture(unit.clone(), resolved.clone(), None),
        );
        let (guard, snapshot) = scope::Scope::capture(&app, &unit).unwrap();
        assert!(snapshot.image.pixels.same_storage(&resolved.pixels));
        app.documents[0].state.canvas.borrow_mut().minified_source = None;
        assert!(
            guard.matches(&app),
            "unused local retained source cannot supersede resolved pixels"
        );
        app.shutdown();
    }
}
#[test]
fn native_sender_retained_pixels_keep_current_vector_and_raster_annotations_independent() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (files, _, _) = Files::new();
    unsafe {
        let (_class, mut app) = application(files.root());
        drop(Case::Natural.install(&mut app));
        let unit = app.documents[0].state.unit_id.clone();
        let mut canvas = app.documents[0].state.canvas.borrow_mut();
        let before = canvas.editor.layers().unwrap().clone();
        let mut next = before.clone();
        let mut raster = vec![0; 32 * 16 * 4];
        for y in 4..8 {
            for x in 4..8 {
                raster[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4].copy_from_slice(&[0, 255, 255, 255]);
            }
        }
        next.annotations = Some(raster.into());
        let revision = canvas.editor.revision();
        canvas
            .editor
            .replace_layers(revision, &before, next)
            .unwrap();
        canvas.editor.set_style(
            crate::stroke_style::StrokeStyle::new(6.0, crate::stroke_style::Color(0, 0, 255))
                .unwrap(),
        );
        canvas
            .editor
            .begin(crate::editor::Tool::Brush, Point { x: 24.0, y: 8.0 })
            .unwrap();
        assert!(canvas.editor.commit());
        canvas.normal_opacity = 0.5;
        drop(canvas);
        let (_, snapshot) = scope::Scope::capture(&app, &unit).unwrap();
        let output = crate::desktop::export::projection::frame(
            &snapshot.image,
            &snapshot.editor,
            snapshot.opacity,
            snapshot.kind,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!((output.width, output.height), (128, 64));
        let decoded = crate::desktop::export::decode_png(output.png.to_vec());
        let pixel = |x: usize, y: usize| &decoded.pixels[(y * 128 + x) * 4..(y * 128 + x) * 4 + 4];
        assert_eq!(pixel(24, 24), &[0, 255, 255, 255]);
        assert_eq!(pixel(96, 32), &[255, 0, 0, 255]);
        let base = pixel(112, 56);
        assert_eq!(&base[..2], &[0, 0]);
        assert!((127..=128).contains(&base[2]) && (127..=128).contains(&base[3]));
        app.shutdown();
    }
}

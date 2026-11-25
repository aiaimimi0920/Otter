//! Transform/group previews must draw transiently, without replacing committed geometry with the same ID.
use super::Cache;
use crate::{
    editor::{Editor, Stroke, Tool},
    viewport::{Point, Viewport},
};
use windows::Win32::{
    Graphics::{
        Direct2D::{Common::*, *},
        Imaging::*,
    },
    System::Com::*,
};

unsafe fn render(
    target: &ID2D1RenderTarget,
    output: &IWICBitmap,
    editor: &Editor,
    replacement: Option<&Stroke>,
    group: bool,
    cache: &mut Cache,
) -> Vec<u8> {
    let brush = target
        .CreateSolidColorBrush(&D2D1_COLOR_F::default(), None)
        .unwrap();
    let viewport = Viewport {
        origin: Point::default(),
        scale: 1.,
    };
    target.BeginDraw();
    target.Clear(Some(&D2D1_COLOR_F::default()));
    if group {
        crate::desktop::annotations::draw_group_preview(
            target,
            &brush,
            editor,
            viewport,
            &[(0, replacement.unwrap().clone())],
            cache,
        )
        .unwrap();
    } else {
        crate::desktop::annotations::draw_preview(
            target,
            &brush,
            editor,
            viewport,
            None,
            replacement.map(|stroke| (0, stroke)),
            cache,
        )
        .unwrap();
    }
    target.EndDraw(None, None).unwrap();
    let mut bytes = vec![0; 32 * 32 * 4];
    output
        .CopyPixels(std::ptr::null(), 32 * 4, &mut bytes)
        .unwrap();
    bytes
}

#[test]
fn replacement_and_group_previews_do_not_evict_committed_paths() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let mut editor = Editor::default();
    editor
        .append_restored(
            Tool::Brush,
            vec![Point { x: 2., y: 2. }, Point { x: 20., y: 18. }],
            0.,
            Vec::new(),
        )
        .unwrap();
    let mut replacement = editor.elements()[0].clone();
    replacement.points[1].x = 30.;
    let mut expected = Editor::default();
    expected
        .append_restored(Tool::Brush, replacement.points.clone(), 0., Vec::new())
        .unwrap();
    let mut cache = Cache::default();
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let output = wic
            .CreateBitmap(32, 32, &GUID_WICPixelFormat32bppPBGRA, WICBitmapCacheOnLoad)
            .unwrap();
        let factory: ID2D1Factory =
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
        let target = factory
            .CreateWicBitmapRenderTarget(
                &output,
                &D2D1_RENDER_TARGET_PROPERTIES {
                    dpiX: 96.,
                    dpiY: 96.,
                    ..Default::default()
                },
            )
            .unwrap();
        let committed = render(&target, &output, &editor, None, false, &mut cache);
        let changed = render(
            &target,
            &output,
            &expected,
            None,
            false,
            &mut Cache::default(),
        );
        assert_ne!(changed, committed);
        assert!(changed.iter().any(|byte| *byte != 0));
        assert_eq!(cache.annotations.path_builds, 1);
        for _ in 0..2 {
            for group in [false, true] {
                assert_eq!(
                    render(
                        &target,
                        &output,
                        &editor,
                        Some(&replacement),
                        group,
                        &mut cache
                    ),
                    changed
                );
            }
            assert_eq!(
                render(&target, &output, &editor, None, false, &mut cache),
                committed
            );
            assert_eq!(cache.annotations.path_builds, 1);
        }
    }
}

//! UI线程独占四份Presentation/COM缓存；后台只移交不可变场景/像素，不创建子窗口。
use crate::{
    surface_presentation::{Images, Presentation},
    tile_shared::Frame,
    tile_surface_owner::Art,
};
use std::{collections::BTreeMap, sync::Arc};
#[path = "tile_surface_focus_render.rs"]
pub(super) mod focus;
#[path = "tile_surface_local_render.rs"]
pub(super) mod local;
#[path = "tile_surface_scroll_render.rs"]
pub(super) mod scroll;
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::E_INVALIDARG,
        Graphics::Direct2D::{Common::*, *},
    },
};
pub(super) struct Prepared {
    pub(super) owner: Arc<Art>,
    presentation: Presentation,
    bitmaps: BTreeMap<String, std::rc::Rc<crate::surface_image_paint::Bitmap>>,
    preview: Option<String>,
    text: BTreeMap<String, std::rc::Rc<str>>,
    local: BTreeMap<String, crate::tile_surface_local::Value>,
}
pub(super) fn text(
    cache: &BTreeMap<String, Prepared>,
    frame: &Frame,
    pixel: [i32; 2],
) -> Option<crate::tile_surface_text::Field> {
    let (placement, _) =
        crate::tile_surface_hit::hit(&frame.layout, &frame.key.endpoint, frame.size, pixel)?;
    let art = frame.arts.get(&placement.source.id)?;
    let p = cache
        .get(&placement.source.id)
        .filter(|p| Arc::ptr_eq(&p.owner, art))?;
    crate::tile_surface_text::hit(frame, art, &p.presentation, pixel)
}
pub(super) fn texts(
    cache: &mut BTreeMap<String, Prepared>,
    rows: &[crate::tile_surface_text::Preview],
) -> Result<()> {
    for p in cache.values_mut() {
        let desired: BTreeMap<_, _> = rows
            .iter()
            .filter(|r| r.field.matches_art(&p.owner, &p.presentation))
            .map(|r| (r.field.node.clone(), r.value.clone()))
            .collect();
        for id in p
            .text
            .keys()
            .filter(|id| !desired.contains_key(*id))
            .cloned()
            .collect::<Vec<_>>()
        {
            p.presentation.preview_text(&id, None)?;
        }
        for (id, value) in &desired {
            if p.text.get(id) != Some(value) {
                p.presentation.preview_text(id, Some(value))?;
            }
        }
        p.text = desired;
    }
    Ok(())
}
pub(super) fn slider(
    cache: &BTreeMap<String, Prepared>,
    frame: &Frame,
    pixel: [i32; 2],
) -> Option<crate::tile_surface_slider::Field> {
    let (placement, _) =
        crate::tile_surface_hit::hit(&frame.layout, &frame.key.endpoint, frame.size, pixel)?;
    let art = frame.arts.get(&placement.source.id)?;
    let p = cache
        .get(&placement.source.id)
        .filter(|p| Arc::ptr_eq(&p.owner, art))?;
    crate::tile_surface_slider::hit(frame, art, &p.presentation, pixel)
}
pub(super) fn preview(
    cache: &mut BTreeMap<String, Prepared>,
    frame: Option<&Frame>,
    drag: Option<&crate::tile_surface_slider::Drag>,
) {
    for p in cache.values_mut() {
        if let Some(id) = p.preview.take() {
            let value = match p.local.get(&id) {
                Some(crate::tile_surface_local::Value::Slider(v)) => Some(*v),
                _ => None,
            };
            p.presentation.preview_slider(&id, value);
        }
        if let Some(drag) = drag.filter(|d| {
            d.edit.active()
                && frame
                    .and_then(|f| {
                        crate::tile_surface_slider::hit(
                            f,
                            &p.owner,
                            &p.presentation,
                            d.field.anchor.pixel,
                        )
                    })
                    .is_some_and(|f| d.field.same(&f))
        }) {
            if p.presentation
                .preview_slider(&drag.field.node, Some(drag.value))
            {
                p.preview = Some(drag.field.node.clone());
            }
        }
    }
}
pub(super) fn value(
    cache: &BTreeMap<String, Prepared>,
    frame: &Frame,
    pixel: [i32; 2],
    modifiers: crate::surface_click_event::Modifiers,
) -> Option<crate::tile_surface_value::Field> {
    let (placement, _) =
        crate::tile_surface_hit::hit(&frame.layout, &frame.key.endpoint, frame.size, pixel)?;
    let art = frame.arts.get(&placement.source.id)?;
    let prepared = cache
        .get(&placement.source.id)
        .filter(|p| Arc::ptr_eq(&p.owner, art))?;
    crate::tile_surface_value::hit(frame, art, &prepared.presentation, pixel, modifiers)
}
pub(super) fn click(
    cache: &BTreeMap<String, Prepared>,
    frame: &Frame,
    pixel: [i32; 2],
    modifiers: crate::surface_click_event::Modifiers,
) -> Result<Vec<crate::tile_surface_actions::Command>> {
    let Some((placement, _)) =
        crate::tile_surface_hit::hit(&frame.layout, &frame.key.endpoint, frame.size, pixel)
    else {
        return Ok(Vec::new());
    };
    let Some(art) = frame.arts.get(&placement.source.id) else {
        return Ok(Vec::new());
    };
    let Some(prepared) = cache
        .get(&placement.source.id)
        .filter(|p| Arc::ptr_eq(&p.owner, art))
    else {
        return Ok(Vec::new());
    };
    crate::tile_surface_click::prepare(frame, art, &prepared.presentation, pixel, modifiers)
        .map_err(|message| Error::new(E_INVALIDARG, message))
}
pub(super) fn prepare(cache: &mut BTreeMap<String, Prepared>, frame: Option<&Frame>) -> Result<()> {
    cache.retain(|id, _| frame.is_some_and(|f| f.arts.contains_key(id)));
    let Some(frame) = frame else {
        return Ok(());
    };
    let mut pool: BTreeMap<_, _> = cache
        .values()
        .flat_map(|p| p.bitmaps.iter().map(|(id, b)| (id.clone(), b.clone())))
        .collect();
    for (id, art) in &frame.arts {
        if cache.get(id).is_some_and(|p| Arc::ptr_eq(&p.owner, art)) {
            continue;
        }
        let scene = crate::surface_scene::Scene::parse(&art.snapshot.scene)
            .map_err(|_| Error::new(E_INVALIDARG, "Tile Art scene"))?;
        let mut bitmaps = BTreeMap::new();
        let mut images = Images::new();
        for (id, image) in &art.images {
            let resource = &art.snapshot.images[id];
            let bitmap = if let Some(bitmap) = pool.get(resource) {
                bitmap.clone()
            } else {
                let b = crate::surface_image_paint::Bitmap::new(image.clone())?;
                pool.insert(resource.clone(), b.clone());
                b
            };
            bitmaps.insert(resource.clone(), bitmap.clone());
            images.insert(id.clone(), bitmap);
        }
        let size = art.size.map(|v| v as f32);
        let mut presentation = Presentation::with_images(
            scene,
            crate::surface_viewport::style(),
            crate::surface_viewport::context(size),
            size,
            images,
        )?;
        if let Some(previous) = cache.get(id).filter(|p| {
            p.owner.view == art.view
                && p.owner.generation == art.generation
                && p.owner.size == art.size
        }) {
            presentation.restore_scroll(&previous.presentation.scroll_state())?;
        }
        presentation.embed_controls()?;
        cache.insert(
            id.clone(),
            Prepared {
                owner: art.clone(),
                presentation,
                bitmaps,
                preview: None,
                text: Default::default(),
                local: Default::default(),
            },
        );
    }
    Ok(())
}
pub(super) unsafe fn draw(
    prepared: &Prepared,
    target: &ID2D1RenderTarget,
    crop: crate::tile_wire::Rect,
    visual: crate::surface_button_visual::State<'_>,
) -> Result<()> {
    // 在unit placement空间先裁剪，再将canonical viewport映射进crop；不栅格化缩小或截图。
    let mut unit = Default::default();
    target.GetTransform(&mut unit);
    target.PushAxisAlignedClip(
        &D2D_RECT_F {
            left: 0.,
            top: 0.,
            right: 1.,
            bottom: 1.,
        },
        D2D1_ANTIALIAS_MODE_ALIASED,
    );
    struct Restore<'a>(&'a ID2D1RenderTarget, [f32; 6]);
    impl Drop for Restore<'_> {
        fn drop(&mut self) {
            unsafe {
                self.0.PopAxisAlignedClip();
                let mut m = Default::default();
                self.0.GetTransform(&mut m);
                [m.M11, m.M12, m.M21, m.M22, m.M31, m.M32] = self.1;
                self.0.SetTransform(&m);
            }
        }
    }
    let _restore = Restore(
        target,
        [unit.M11, unit.M12, unit.M21, unit.M22, unit.M31, unit.M32],
    );
    let [w, h] = prepared.owner.size.map(|v| v as f32);
    let sx = crop.width as f32 * w;
    let sy = crop.height as f32 * h;
    let mut transform = unit;
    transform.M11 /= sx;
    transform.M12 /= sx;
    transform.M21 /= sy;
    transform.M22 /= sy;
    transform.M31 -= crop.x as f32 * w * transform.M11 + crop.y as f32 * h * transform.M21;
    transform.M32 -= crop.x as f32 * w * transform.M12 + crop.y as f32 * h * transform.M22;
    if [transform.M11, transform.M12, transform.M21, transform.M22]
        .iter()
        .any(|n| !n.is_finite() || n.abs() > 65536.)
        || !transform.M31.is_finite()
        || !transform.M32.is_finite()
    {
        return Err(Error::new(E_INVALIDARG, "Tile Art transform limit"));
    }
    target.SetTransform(&transform);
    let brush = target.CreateSolidColorBrush(
        &D2D1_COLOR_F {
            r: 0.,
            g: 0.,
            b: 0.,
            a: 1.,
        },
        None,
    )?;
    prepared
        .presentation
        .draw_with_switches(target, &brush, visual, |id| match prepared.local.get(id) {
            Some(crate::tile_surface_local::Value::Switch(v)) => Some(*v),
            _ => None,
        })
}

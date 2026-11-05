//! 实心边框环及四边分区；共享工厂缓存，透明中心和半透明接角不能重复着色。
use crate::{
    surface_background::Background, surface_box::Insets, surface_clip::Rect,
    surface_round_clip::Shape,
};
use std::{cell::RefCell, rc::Rc};
use windows::{
    core::{Interface, Result},
    Win32::Graphics::Direct2D::{Common::*, *},
};

pub(crate) struct Border {
    geometry: Rc<Geometry>,
    side: Option<usize>,
}
struct Geometry {
    outer: D2D1_ROUNDED_RECT,
    width: Insets,
    clip: Rect,
    cache: RefCell<Option<Cache>>,
}
struct Cache {
    factory: ID2D1Factory,
    ring: ID2D1GeometryGroup,
    sectors: Vec<ID2D1Geometry>,
}
#[path = "surface_border_partition.rs"]
mod partition;
#[path = "surface_border_style.rs"]
pub(crate) mod style;
impl Border {
    pub(crate) fn new(background: &Background, width: Insets) -> Option<Self> {
        if width.total() == [0.; 2] {
            return None;
        }
        let (outer, clip) = background.outline();
        Some(Self {
            geometry: Rc::new(Geometry {
                outer,
                width,
                clip,
                cache: RefCell::new(None),
            }),
            side: None,
        })
    }
    fn part(&self, side: usize) -> Self {
        assert!(side < 4);
        Self {
            geometry: self.geometry.clone(),
            side: Some(side),
        }
    }
    pub(crate) fn draw(
        &self,
        target: &ID2D1RenderTarget,
        brush: &ID2D1SolidColorBrush,
    ) -> Result<()> {
        let owner = &self.geometry;
        let factory = unsafe { target.GetFactory()? };
        let rebuild = owner.cache.borrow().as_ref().is_none_or(|cache| {
            cache.factory.as_raw() != factory.as_raw()
                || (self.side.is_some() && cache.sectors.is_empty())
        });
        if rebuild {
            // 每节点四边共用一份 ring/sector；整批成功后替换，不积累历史工厂。
            *owner.cache.borrow_mut() = Some(owner.build(factory, self.side.is_some())?);
        }
        let cache = owner.cache.borrow();
        let cache = cache.as_ref().unwrap();
        let mask: ID2D1Geometry = cache.ring.cast()?;
        let clip = D2D_RECT_F {
            left: owner.clip.left,
            top: owner.clip.top,
            right: owner.clip.right,
            bottom: owner.clip.bottom,
        };
        unsafe {
            target.PushAxisAlignedClip(&clip, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
            if let Some(side) = self.side {
                let mut parameters = D2D1_LAYER_PARAMETERS {
                    contentBounds: owner.outer.rect,
                    geometricMask: std::mem::ManuallyDrop::new(Some(mask)),
                    opacity: 1.,
                    ..Default::default()
                };
                parameters.maskTransform.M11 = 1.;
                parameters.maskTransform.M22 = 1.;
                target.PushLayer(&parameters, None);
                std::mem::ManuallyDrop::drop(&mut parameters.geometricMask);
                let antialias = target.GetAntialiasMode();
                // 分色接缝只有一个像素 owner；外圆角仍由 ring mask 抗锯齿。
                target.SetAntialiasMode(D2D1_ANTIALIAS_MODE_ALIASED);
                target.FillGeometry(&cache.sectors[side], brush, None);
                target.SetAntialiasMode(antialias);
                target.PopLayer();
            } else {
                target.FillGeometry(&cache.ring, brush, None);
            }
            target.PopAxisAlignedClip();
        }
        Ok(())
    }
}
impl Geometry {
    fn build(&self, factory: ID2D1Factory, partition: bool) -> Result<Cache> {
        let mut shapes = vec![Some(unsafe {
            factory
                .CreateRoundedRectangleGeometry(&self.outer)?
                .cast()?
        })];
        let rect = self.outer.rect;
        if rect.right - rect.left > self.width.total()[0]
            && rect.bottom - rect.top > self.width.total()[1]
        {
            shapes.push(Some(
                Shape::from(self.outer)
                    .inset(self.width)
                    .geometry(&factory)?,
            ));
        }
        let ring = unsafe { factory.CreateGeometryGroup(D2D1_FILL_MODE_ALTERNATE, &shapes)? };
        let sectors = if partition {
            self.sectors(&factory)?
        } else {
            Vec::new()
        };
        Ok(Cache {
            factory,
            ring,
            sectors,
        })
    }
    fn sectors(&self, factory: &ID2D1Factory) -> Result<Vec<ID2D1Geometry>> {
        (0..4)
            .map(|side| {
                let polygon = partition::polygon(self.outer.rect, self.width, side);
                let path = unsafe { factory.CreatePathGeometry()? };
                let sink = unsafe { path.Open()? };
                let point = |[x, y]: [f32; 2]| D2D_POINT_2F { x, y };
                unsafe {
                    if let Some(first) = polygon.first() {
                        sink.BeginFigure(point(*first), D2D1_FIGURE_BEGIN_FILLED);
                        for p in &polygon[1..] {
                            sink.AddLine(point(*p));
                        }
                        sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                    }
                    sink.Close()?;
                }
                path.cast()
            })
            .collect()
    }
}

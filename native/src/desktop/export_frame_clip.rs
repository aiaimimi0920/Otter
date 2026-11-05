//! 工作流圆角覆盖完整合成栈；D2D 层及其预算以同一作用域释放。
use super::*;
use windows::core::Interface;

pub(in crate::desktop) fn radius(edit: &serde_json::Value) -> Result<f32> {
    let radius = match edit.get("cornerRadius") {
        None | Some(serde_json::Value::Null) => 0.0,
        Some(value) => value
            .as_f64()
            .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid corner radius"))?,
    };
    if !radius.is_finite() || !(0.0..=8192.0).contains(&radius) {
        return Err(Error::new(E_INVALIDARG, "Invalid corner radius"));
    }
    Ok(radius as f32)
}

pub(in crate::desktop::export) struct Clip<'a> {
    // PopLayer 必须早于预算释放；调用方还须在 EndDraw 之前释放整个 guard。
    _layer: crate::desktop::annotation_opacity::Layer<'a>,
    _lease: crate::byte_budget::Lease,
}

pub(in crate::desktop::export) unsafe fn push<'a>(
    target: &'a ID2D1RenderTarget,
    factory: &ID2D1Factory,
    layout: Option<&Layout>,
    reserve: impl FnOnce(usize) -> Option<crate::byte_budget::Lease>,
) -> Result<Option<Clip<'a>>> {
    let Some(layout) = layout.filter(|value| value.radius > 0.0) else {
        return Ok(None);
    };
    let bytes = checked_image_bytes(layout.output.0, layout.output.1)
        .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid corner mask dimensions"))?;
    let lease = reserve(bytes).ok_or_else(|| {
        Error::new(
            windows::Win32::Foundation::E_OUTOFMEMORY,
            "Corner mask budget exhausted",
        )
    })?;
    let frame = layout.visible;
    let bounds = D2D_RECT_F {
        left: frame.x as f32,
        top: frame.y as f32,
        right: (frame.x + frame.width) as f32,
        bottom: (frame.y + frame.height) as f32,
    };
    // Canvas roundRect 的统一半径在过大时等比例缩小，而不是分别压扁两轴。
    let radius = layout
        .radius
        .min(frame.width.min(frame.height) as f32 / 2.0);
    let geometry = factory.CreateRoundedRectangleGeometry(&D2D1_ROUNDED_RECT {
        rect: bounds,
        radiusX: radius,
        radiusY: radius,
    })?;
    let layer = crate::desktop::annotation_opacity::push_mask(target, bounds, geometry.cast()?);
    Ok(Some(Clip {
        _layer: layer,
        _lease: lease,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mask_budget_failure_and_success_release_all_transient_leases() {
        let _com = Apartment::new().unwrap();
        let wic: IWICImagingFactory = unsafe {
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap()
        };
        let image = Image {
            width: 10,
            height: 10,
            pixels: vec![255; 400].into(),
        };
        let mut layout = Layout::new(
            &image,
            Size {
                width: 10.0,
                height: 10.0,
            },
            None,
        )
        .unwrap();
        layout.radius = 3.0;
        // 底图上传与输出各 400 字节；第三份圆角层必须独立准入。
        let limited = crate::byte_budget::Budget::new(800);
        assert!(compose_layout(
            &image,
            &Editor::default(),
            &wic,
            1.0,
            |bytes| limited.reserve(bytes),
            Some(&layout)
        )
        .is_err());
        assert_eq!(limited.used(), 0);
        let budget = crate::byte_budget::Budget::new(1200);
        let output = compose_layout(
            &image,
            &Editor::default(),
            &wic,
            1.0,
            |bytes| budget.reserve(bytes),
            Some(&layout),
        )
        .unwrap();
        assert_eq!(budget.used(), 400);
        drop(output);
        assert_eq!(budget.used(), 0);
    }
    #[test]
    fn radius_rejects_unbounded_or_wrong_type_values() {
        for value in [
            serde_json::json!(-1),
            serde_json::json!(8193),
            serde_json::json!("5"),
        ] {
            assert!(radius(&serde_json::json!({"cornerRadius":value})).is_err());
        }
        assert_eq!(radius(&serde_json::json!({})).unwrap(), 0.0);
        assert_eq!(
            radius(&serde_json::json!({"cornerRadius":2.5})).unwrap(),
            2.5
        );
    }
}

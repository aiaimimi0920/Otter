//! 真实QuickJS→软件D2D→PNG；像素oracle手工给出，与新drawImage路径独立。
use super::*;
use crate::surface_javascript_wire::{canvas as pixels, images};
fn large_engine() -> Engine {
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, 2048, 2048);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&vec![255; 2048 * 2048 * 4])
            .unwrap();
    }
    let resources = vec![images::ImageResource {
        id: "input:1".into(),
        width: 2048,
        height: 2048,
        png: png.into(),
    }];
    let mut engine = Engine::new().unwrap();
    engine
        .install_images(
            &images::headers(&resources).unwrap(),
            &mut &resources[0].png[..],
        )
        .unwrap();
    engine
}
#[test]
fn input_and_target_bitmap_share_canvas_budget_and_failed_draw_releases_every_lease() {
    let mut engine = large_engine();
    let budget = engine.canvas.borrow().test_budget();
    assert_eq!(budget.used(), 16 * 1024 * 1024);
    assert!(mount(
        &mut engine,
        "c.width=2048;c.height=2048;x.drawImage(im,0,0)"
    )
    .is_err());
    assert_eq!(budget.used(), pixels::TOTAL_BYTES);
    drop(engine);
    assert_eq!(budget.used(), 0);
}
#[test]
fn zero_area_and_fully_outside_source_do_not_allocate_a_large_target_bitmap() {
    for body in [
        "x.drawImage(im,0,0,0,0)",
        "x.drawImage(im,3000,3000,1,1,0,0,1,1)",
    ] {
        let mut engine = large_engine();
        let output = mount(&mut engine, body).unwrap();
        assert_eq!(
            engine.canvas.borrow().test_budget().used(),
            16 * 1024 * 1024 + 6 * 4 * 4
        );
        assert!(image(output).pixels.iter().all(|v| *v == 0));
    }
}
fn engine() -> Engine {
    let mut engine = Engine::new().unwrap();
    let resources = vec![images::tests::resource()];
    let headers = images::headers(&resources).unwrap();
    let mut bytes = Vec::new();
    images::write(&mut bytes, &resources).unwrap();
    engine.install_images(&headers, &mut &bytes[..]).unwrap();
    engine
}
fn mount(engine: &mut Engine, body: &str) -> io::Result<Output> {
    engine.apply(Command::Mount {
        source:format!("NeuroSurface.define({{mount({{root}}){{const im=NeuroSurface.image('input:1'),c=document.createElement('canvas');c.width=6;c.height=4;c.id='out';root.append(c);const x=c.getContext('2d');x.imageSmoothingEnabled=false;{body}}}}});"),
        snapshot:serde_json::json!({}),resources:serde_json::json!({}),
    },None)
}
fn image(mut output: Output) -> crate::image_decode::Image {
    let scene = pixels::scene(&output.scene).unwrap();
    pixels::images(&scene, std::mem::take(&mut output.canvases))
        .unwrap()
        .into_values()
        .next()
        .unwrap()
}
fn pixel(image: &crate::image_decode::Image, x: usize, y: usize) -> &[u8] {
    let i = (y * image.width as usize + x) * 4;
    &image.pixels[i..i + 4]
}
#[test]
fn draw_image_native_size_crop_nonuniform_scale_transform_and_alpha_have_independent_pixels() {
    for (body, expected) in [
        (
            "x.drawImage(im,1,1)",
            vec![
                (1, 1, [0, 0, 255, 255]),
                (2, 1, [0, 255, 0, 255]),
                (1, 2, [255, 0, 0, 255]),
                (2, 2, [0, 128, 128, 128]),
                (0, 0, [0, 0, 0, 0]),
            ],
        ),
        (
            "x.drawImage(im,2,0,4,4)",
            vec![
                (2, 0, [0, 0, 255, 255]),
                (5, 0, [0, 255, 0, 255]),
                (2, 3, [255, 0, 0, 255]),
            ],
        ),
        (
            "x.drawImage(im,1,0,1,1,0,0,4,2)",
            vec![
                (0, 0, [0, 255, 0, 255]),
                (3, 1, [0, 255, 0, 255]),
                (4, 1, [0, 0, 0, 0]),
            ],
        ),
        (
            "x.setTransform(-1,0,0,1,2,0);x.drawImage(im,0,0)",
            vec![(0, 0, [0, 255, 0, 255]), (1, 0, [0, 0, 255, 255])],
        ),
        (
            "x.globalAlpha=0.5;x.drawImage(im,0,0)",
            vec![(0, 0, [0, 0, 128, 128]), (1, 0, [0, 128, 0, 128])],
        ),
        (
            "x.drawImage(im,-1,0,2,1,0,0,4,2)",
            vec![
                (0, 0, [0, 0, 0, 0]),
                (2, 0, [0, 0, 255, 255]),
                (3, 1, [0, 0, 255, 255]),
            ],
        ),
        (
            "x.drawImage(im,2,2,-2,-2,2,2,-2,-2)",
            vec![(0, 0, [0, 0, 255, 255]), (1, 0, [0, 255, 0, 255])],
        ),
    ] {
        let mut engine = engine();
        let actual = image(mount(&mut engine, body).unwrap());
        for (x, y, expected) in expected {
            assert_eq!(pixel(&actual, x, y), expected, "{body} at {x},{y}");
        }
    }
}
#[test]
fn draw_image_handles_cannot_be_forged_and_no_arbitrary_resource_access_is_available() {
    for body in [
        "x.drawImage({id:'input:1',width:2,height:2},0,0)",
        "x.drawImage('file:///private.png',0,0)",
        "NeuroSurface.image('https://example.test/private.png')",
        "NeuroSurface.image('unknown')",
        "x.drawImage(im,0)",
        "x.drawImage.call({},im,NaN,0)",
        "x.drawImage(im,1048577,0)",
    ] {
        assert!(mount(&mut engine(), body).is_err(), "{body}");
    }
}
#[test]
fn image_handles_reset_state_and_captured_intrinsics_preserve_pixels() {
    let mut engine = engine();
    let output=mount(&mut engine,r#"
        if(im!==NeuroSurface.image('input:1')||im.width!==2||im.height!==2||!Object.isFrozen(im))throw Error('handle');
        x.save();x.imageSmoothingEnabled=true;x.globalAlpha=0.5;x.restore();
        if(x.imageSmoothingEnabled!==false||x.globalAlpha!==1)throw Error('save');
        Array.prototype.slice=()=>{throw Error('slice')};WeakMap.prototype.get=()=>{throw Error('weak')};
        x.drawImage(im,0,0,1,1,0,0,1,1);c.width=c.width;
        if(x.imageSmoothingEnabled!==true)throw Error('reset');x.drawImage(im,0,0);
    "#).unwrap();
    assert_eq!(pixel(&image(output), 0, 0), [0, 0, 255, 255]);
}
#[test]
fn installation_checks_png_dimensions_is_single_use_and_releases_on_dispose() {
    let mut engine = engine();
    let resources = vec![images::tests::resource()];
    let mut headers = images::headers(&resources).unwrap();
    assert!(engine
        .install_images(&headers, &mut &resources[0].png[..])
        .is_err());
    let output = mount(&mut engine, "x.drawImage(im,0,0)").unwrap();
    drop(output);
    let budget = engine.canvas.borrow().test_budget();
    assert_eq!(budget.used(), 16 + 16 + 6 * 4 * 4);
    engine.apply(Command::Dispose, None).unwrap();
    assert_eq!(budget.used(), 0);
    let mut other = Engine::new().unwrap();
    headers[0].size = [1, 4];
    assert!(other
        .install_images(&headers, &mut &resources[0].png[..])
        .is_err());
}
#[test]
fn repeated_draws_share_one_target_bitmap_and_all_allocations_stay_under_original_budget() {
    let mut engine = engine();
    let output = mount(&mut engine, "for(let i=0;i<100;i++)x.drawImage(im,0,0)").unwrap();
    assert_eq!(
        engine.canvas.borrow().test_budget().used(),
        16 + 16 + 6 * 4 * 4
    );
    assert_eq!(pixel(&image(output), 0, 0), [0, 0, 255, 255]);
}

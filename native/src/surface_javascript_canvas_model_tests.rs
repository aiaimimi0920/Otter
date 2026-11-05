//! 真实 QuickJS/WIC/D2D 像素与状态；不以命令数量或 JSON 代替实际 Canvas 绘制。
use super::*;
use crate::{surface_javascript_wire::canvas as pixels, surface_viewport::Viewport};
fn mount(source: &str) -> io::Result<(Engine, Output)> {
    let mut engine = Engine::new()?;
    let result = engine.apply(
        Command::Mount {
            source: source.into(),
            snapshot: crate::surface_javascript::fixture::snapshot(3, 42),
            resources: serde_json::json!({}),
        },
        Some(Viewport::new([200, 80], 96).unwrap()),
    );
    if result.is_err() {
        wire::diagnostics::log(Some(&engine.diagnostic()));
    }
    let output = result?;
    Ok((engine, output))
}
fn image(output: &mut Output) -> crate::image_decode::Image {
    let scene = pixels::scene(&output.scene).unwrap();
    pixels::images(&scene, std::mem::take(&mut output.canvases))
        .unwrap()
        .into_values()
        .next()
        .unwrap()
}
fn pixel(image: &crate::image_decode::Image, x: usize, y: usize) -> &[u8] {
    let start = (y * image.width as usize + x) * 4;
    &image.pixels[start..start + 4]
}
#[test]
fn javascript_canvas_model_real_pixels_text_alpha_paths_and_observer_frame() {
    let (mut engine, mut output) =
        mount(include_str!("../tests/fixtures/javascript-canvas.js")).unwrap();
    let first = image(&mut output);
    assert_eq!([first.width, first.height], [80, 40]);
    assert_eq!(pixel(&first, 2, 2), [0, 255, 0, 255]);
    assert_eq!(pixel(&first, 12, 2), [0, 127, 128, 255]);
    assert_eq!(pixel(&first, 20, 2), [0, 255, 255, 255]);
    assert_eq!(pixel(&first, 76, 2), [0, 0, 0, 0]);
    assert_eq!(pixel(&first, 54, 16), [0, 255, 255, 255]);
    assert!((25..39).any(|y| (2..70).any(|x| pixel(&first, x, y)[..3].iter().all(|v| *v > 220))));
    assert!(output.animation_pending && output.resize_observed && output.layout_read);
    let mut next = engine
        .apply(
            Command::AnimationFrame,
            Some(Viewport::new([200, 80], 96).unwrap()),
        )
        .unwrap();
    assert_eq!(pixel(&image(&mut next), 2, 2), [255, 0, 0, 255]);
    assert!(!next.animation_pending);
}
#[test]
fn javascript_canvas_model_same_dimension_reset_clears_state_path_stack_and_bitmap() {
    let (mut engine,mut output)=mount("let c,x;NeuroSurface.define({mount({root}){c=document.createElement('canvas');c.width=8;c.height=8;x=c.getContext('2d');x.fillStyle='red';x.fillRect(0,0,8,8);x.save();x.beginPath();x.rect(0,0,8,8);root.append(c)},update(){const old=x;c.width=c.width;if(c.getContext('2d')!==old||x.fillStyle!=='#000000')throw Error('reset');x.restore();x.fill();x.fillRect(0,0,2,2)}})").unwrap();
    assert_eq!(pixel(&image(&mut output), 4, 4), [0, 0, 255, 255]);
    let mut output = engine
        .apply(
            Command::Update {
                snapshot: serde_json::json!({}),
                resources: serde_json::json!({}),
            },
            Some(Viewport::new([200, 80], 96).unwrap()),
        )
        .unwrap();
    let next = image(&mut output);
    assert_eq!(pixel(&next, 4, 4), [0, 0, 0, 0]);
    assert_eq!(pixel(&next, 0, 0), [0, 0, 0, 255]);
}
#[test]
fn javascript_canvas_model_persists_pixels_and_captured_intrinsics_without_exposing_private_arrays()
{
    let (mut engine,mut output)=mount("let c,x;NeuroSurface.define({mount({root}){c=document.createElement('canvas');c.width=8;c.height=8;x=c.getContext('2d');x.fillStyle='red';x.fillRect(0,0,8,8);root.append(c);x.setLineDash([1,2,3]);const a=x.getLineDash();a[0]=99;if(x.getLineDash()[0]!==1)throw Error('leak')},update(){Array.from=()=>{throw Error('from')};Map.prototype.forEach=()=>{throw Error('map')};Object.prototype.toJSON=()=>{throw Error('json')};x.getLineDash();x.fillStyle='blue';x.fillRect(0,0,2,2)}})").unwrap();
    assert_eq!(pixel(&image(&mut output), 4, 4), [0, 0, 255, 255]);
    let mut output = engine
        .apply(
            Command::Update {
                snapshot: serde_json::json!({}),
                resources: serde_json::json!({}),
            },
            Some(Viewport::new([200, 80], 96).unwrap()),
        )
        .unwrap();
    let next = image(&mut output);
    assert_eq!(pixel(&next, 4, 4), [0, 0, 255, 255]);
    assert_eq!(pixel(&next, 0, 0), [255, 0, 0, 255]);
}
#[test]
fn javascript_canvas_model_limits_receivers_zero_and_explicit_unsupported_operations() {
    for body in [
        "c.width=4097",
        "x.arc(0,0,-1,0,1)",
        "x.clearRect.call({},0,0,1,1)",
        "for(let i=0;i<33;i++)x.save()",
        "x.setTransform(1,1,0,1,0,0);x.clearRect(0,0,1,1)",
        "x.moveTo(0,0);x.setTransform(2,0,0,2,0,0);x.lineTo(1,1)",
        "c.getContext('2d',{alpha:false})",
    ] {
        let source=format!("NeuroSurface.define({{mount({{root}}){{const c=document.createElement('canvas'),x=c.getContext('2d');root.append(c);{body}}}}})");
        assert!(mount(&source).is_err(), "{body}");
    }
    let (_,output)=mount("NeuroSurface.define({mount({root}){const c=document.createElement('canvas');if(c.width!==300||c.height!==150||c.getContext('webgl')!==null)throw Error('default');c.width=0;root.append(c)}})").unwrap();
    assert!(output.canvases[0].png.is_empty());
    assert_eq!(output.canvases[0].width, 0);
}
#[test]
fn javascript_canvas_model_png_frames_reject_wrong_identity_dimensions_and_total_budget() {
    let (_, mut output) = mount(
        "NeuroSurface.define({mount({root}){root.append(document.createElement('canvas'))}})",
    )
    .unwrap();
    let scene = pixels::scene(&output.scene).unwrap();
    let frames = serde_json::to_value(&output.canvases).unwrap();
    let mut unknown = frames.clone();
    unknown[0]["id"] = serde_json::json!("forged");
    assert!(pixels::images(&scene, serde_json::from_value(unknown).unwrap()).is_err());
    assert!(pixels::images(&scene, Vec::new()).is_err());
    let public = crate::surface_scene::Scene::parse(&wire::encode(&output.scene).unwrap()).unwrap();
    assert!(pixels::images(&public, serde_json::from_value(frames.clone()).unwrap()).is_err());
    let mut duplicate: Vec<pixels::CanvasFrame> = serde_json::from_value(frames).unwrap();
    duplicate.push(pixels::CanvasFrame {
        id: duplicate[0].id.clone(),
        width: duplicate[0].width,
        height: duplicate[0].height,
        png: duplicate[0].png.clone(),
    });
    assert!(pixels::images(&scene, duplicate).is_err());
    output.canvases[0].width += 1;
    assert!(pixels::images(&scene, output.canvases).is_err());
    assert!(pixels::bytes([4097, 1]).is_err());
    assert!(pixels::bytes([4096, 4096]).is_err());
}
#[test]
fn javascript_canvas_model_receiver_checks_precede_ignored_invalid_values() {
    for body in [
        "Object.getOwnPropertyDescriptor(CanvasRenderingContext2D.prototype,'globalAlpha').set.call({},NaN)",
        "Object.getOwnPropertyDescriptor(CanvasRenderingContext2D.prototype,'fillStyle').set.call({},'invalid')",
        "x.setTransform.call({},NaN,0,0,1,0,0)",
        "x.fillRect.call({},NaN,0,1,1)",
        "x.arc.call({},NaN,0,1,0,1)",
        "x.setLineDash.call({},[-1])",
    ] {
        let source = format!("NeuroSurface.define({{mount(){{const x=document.createElement('canvas').getContext('2d');{body}}}}})");
        assert!(mount(&source).is_err(), "{body}");
    }
}
#[test]
fn javascript_canvas_model_identity_total_backing_and_path_command_limits() {
    let (_, output) = mount("NeuroSurface.define({mount({root}){for(let i=0;i<8;i++){const c=document.createElement('canvas');c.width=1;c.height=1;root.append(c)}}})").unwrap();
    assert_eq!(output.canvases.len(), 8);
    for body in [
        "for(let i=0;i<9;i++)document.createElement('canvas').width=1",
        "for(let i=0;i<3;i++){const c=document.createElement('canvas');c.width=2048;c.height=2048}",
        "for(let i=0;i<4097;i++)x.lineTo(0,0)",
        "for(let i=0;i<8193;i++)x.fillRect(0,0,1,1)",
    ] {
        let source = format!("NeuroSurface.define({{mount(){{const x=document.createElement('canvas').getContext('2d');{body}}}}})");
        assert!(mount(&source).is_err(), "{body}");
    }
}
#[test]
fn javascript_canvas_model_attribute_reset_detach_and_reconnect_keep_identity() {
    let (mut engine, mut output) = mount(
        r#"
        let c,x,host;
        NeuroSurface.define({
          mount({root}) {
            host=root;
            c=document.createElement('canvas');c.setAttribute('width','8');c.height=8;
            x=c.getContext('2d');x.fillStyle='red';x.fillRect(0,0,8,8);root.append(c);
          },
          update() {
            c.remove();c.setAttribute('width','8');
            if(c.getContext('2d')!==x||x.fillStyle!=='#000000')throw Error('reset');
            c.removeAttribute('height');
            if(c.height!==150||c.hasAttribute('height'))throw Error('remove');
            host.append(c);x.fillRect(0,0,2,2);
          }
        });
    "#,
    )
    .unwrap();
    assert_eq!(pixel(&image(&mut output), 4, 4), [0, 0, 255, 255]);
    let mut output = engine
        .apply(
            Command::Update {
                snapshot: serde_json::json!({}),
                resources: serde_json::json!({}),
            },
            Some(Viewport::new([200, 80], 96).unwrap()),
        )
        .unwrap();
    let next = image(&mut output);
    assert_eq!([next.width, next.height], [8, 150]);
    assert_eq!(pixel(&next, 4, 4), [0, 0, 0, 0]);
    assert_eq!(pixel(&next, 0, 0), [0, 0, 0, 255]);
}
#[test]
fn javascript_canvas_model_markup_reflects_dimensions_and_rasterizes_without_replacing_siblings() {
    let (_, mut output) = mount(r#"
        NeuroSurface.define({mount({root}) {
          root.innerHTML='<span id="keep">original</span><canvas width="8" height="6"></canvas>';
          const first=root.firstChild,c=root.lastElementChild;
          if(!(c instanceof HTMLCanvasElement)||c.width!==8||c.height!==6||c.getAttribute('width')!=='8')throw Error('canvas markup');
          const x=c.getContext('2d');x.fillStyle='red';x.fillRect(0,0,8,6);
          if(root.firstChild!==first||first.textContent!=='original')throw Error('sibling identity');
        }});
    "#).unwrap();
    assert_eq!(output.scene["children"][0]["props"]["text"], "original");
    let next = image(&mut output);
    assert_eq!([next.width, next.height], [8, 6]);
    assert_eq!(pixel(&next, 7, 5), [0, 0, 255, 255]);
}
#[test]
fn javascript_canvas_model_raster_failure_is_sticky_across_begin() {
    let mut store = canvas::Store::default();
    store.begin(Instant::now() + std::time::Duration::from_millis(250));
    let mut frames = [pixels::CanvasFrame {
        id: "unknown".into(),
        width: 1,
        height: 1,
        png: String::new(),
    }];
    assert!(store.snapshots(&mut frames).is_err());
    store.begin(Instant::now() + std::time::Duration::from_millis(250));
    assert!(!store.valid());
    assert!(!store.submit(r#"{"id":"unknown","size":[1,1],"reset":true,"commands":[]}"#));
    store.dispose();
}

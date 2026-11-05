//! 签名许可的显式结果权限与真实 child 像素交付；不代替 UI/Save 集成门。
use super::{
    tests::{grant, lease, mount},
    *,
};
const SOURCE: &str = r#"
NeuroSurface.define({mount({root}) {
 const canvas=document.createElement('canvas');canvas.id='output';canvas.width=2;canvas.height=1;
 root.appendChild(canvas);const ctx=canvas.getContext('2d');ctx.fillStyle='red';ctx.fillRect(0,0,2,1);
 NeuroSurface.commitResult({image:{kind:'canvas',nodeId:'output'},data:{kind:'value',value:null}});
}});
"#;
#[test]
fn signed_result_requires_explicit_permission_and_shares_verified_canvas_pixels() {
    let _serial = super::super::fixture::SERIAL.lock().unwrap();
    let mut g = grant(SOURCE);
    let (_owner, device) = lease();
    assert!(mount(&g, SOURCE, device, &|| false).is_err());
    g.permissions.push(wire::Permission::WorkspaceResult);
    let (_owner, device) = lease();
    let (mut local, frame) = mount(&g, SOURCE, device, &|| false).unwrap();
    let result = frame.result.unwrap();
    use crate::surface_javascript_wire::result::ResolvedOutput;
    let ResolvedOutput::Image(image) = &result.outputs["image"] else {
        panic!("missing image")
    };
    assert_eq!(image.pixels.as_ref(), &[0, 0, 255, 255, 0, 0, 255, 255]);
    assert!(image
        .pixels
        .same_storage(&frame.images.values().next().unwrap().pixels));
    assert!(matches!(&result.outputs["data"], ResolvedOutput::Value(value) if value.is_null()));
    local.stop();
    assert_eq!(image.pixels.len(), 8);
}
#[test]
fn signed_result_rejects_missing_canvas_and_revoked_operation() {
    let _serial = super::super::fixture::SERIAL.lock().unwrap();
    let source="NeuroSurface.define({mount(){NeuroSurface.commitResult({image:{kind:'canvas',nodeId:'missing'}});}});";
    let mut g = grant(source);
    g.permissions.push(wire::Permission::WorkspaceResult);
    let (_owner, device) = lease();
    assert!(mount(&g, source, device, &|| false).is_err());
    let source="NeuroSurface.define({mount({root}){const b=document.createElement('button');b.textContent='commit';b.addEventListener('click',()=>NeuroSurface.commitResult({}));root.appendChild(b)}});";
    let mut g = grant(source);
    g.permissions.push(wire::Permission::WorkspaceResult);
    let (_owner, device) = lease();
    let (mut local, frame) = mount(&g, source, device.clone(), &|| false).unwrap();
    let button = frame.scene.root().children()[0].id().to_owned();
    device.revoke();
    assert!(local.click(&g.binding, true, &button, &|| false).is_err());
}

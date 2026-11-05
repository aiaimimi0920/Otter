//! 默认原生安装经真实 Application 菜单、认证取码和缓存重渲染；不配置 ANGLE。
use super::*;

#[test]
#[ignore = "explicit hidden Application / native GPU integration gate"]
fn native_shader_default_application_fetch_commit_and_cached_parameters() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap();
    let _input = crate::desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
    let _apartment = Apartment::new().unwrap();
    unsafe {
        for (own, texture) in [
            (false, TextureSource::Inline),
            (true, TextureSource::Inline),
            (false, TextureSource::Graph),
            (false, TextureSource::Unit),
        ] {
            run_mode(true, Backend::D3d11, own, texture, true);
        }
        assert!(GetModuleHandleW(w!("libEGL.dll")).is_err());
        assert!(GetModuleHandleW(w!("libGLESv2.dll")).is_err());
    }
}

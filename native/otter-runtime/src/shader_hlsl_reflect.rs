//! Recheck active bindings against real bytecode, rejecting extra resources/registers declared by source and preserving both backends' semantics.
use super::*;
use windows::{
    core::Interface,
    Win32::Graphics::{Direct3D::*, Direct3D11::*},
};
pub(super) fn validate(
    blob: &ID3DBlob,
    request: &Request<'_>,
    reflect: super::compiler_api::Reflect,
) -> Result<()> {
    let fail = |e| api(Stage::Pipeline, e);
    unsafe {
        let mut pointer = std::ptr::null_mut();
        reflect(
            blob.GetBufferPointer(),
            blob.GetBufferSize(),
            &ID3D11ShaderReflection::IID,
            &mut pointer,
        )
        .ok()
        .map_err(fail)?;
        if pointer.is_null() {
            return Err(Error::Layout);
        }
        let reflection = ID3D11ShaderReflection::from_raw(pointer);
        let mut desc = D3D11_SHADER_DESC::default();
        reflection.GetDesc(&mut desc).map_err(fail)?;
        if desc.BoundResources as usize > request.textures.len() + 2 || desc.ConstantBuffers > 1 {
            return Err(Error::Layout);
        }
        for i in 0..desc.BoundResources {
            let mut resource = D3D11_SHADER_INPUT_BIND_DESC::default();
            reflection
                .GetResourceBindingDesc(i, &mut resource)
                .map_err(fail)?;
            if resource.Name.is_null() || resource.BindCount != 1 {
                return Err(Error::Layout);
            }
            let name = resource.Name.as_bytes();
            let valid = match resource.Type {
                D3D_SIT_TEXTURE => request
                    .textures
                    .get(resource.BindPoint as usize)
                    .is_some_and(|t| {
                        name == format!("u_{}", t.name).as_bytes()
                            && resource.Dimension == D3D_SRV_DIMENSION_TEXTURE2D
                    }),
                D3D_SIT_SAMPLER => resource.BindPoint == 0 && name == b"otter_sampler",
                D3D_SIT_CBUFFER => resource.BindPoint == 0 && name == b"OtterConstants",
                _ => false,
            };
            if !valid {
                return Err(Error::Layout);
            }
        }
    }
    Ok(())
}

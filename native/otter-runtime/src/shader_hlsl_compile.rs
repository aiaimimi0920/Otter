//! System HLSL compilation with fixed vertex/pixel ABI; no include handler or path/macro argument entry points.
use super::*;
use std::fmt::Write;
use windows::{
    core::s,
    Win32::Graphics::Direct3D::{Fxc::*, ID3DBlob},
};
#[path = "shader_hlsl_compiler_api.rs"]
pub mod compiler_api;
#[path = "shader_hlsl_reflect.rs"]
pub mod reflect;
const VERTEX: &str = "float4 main(uint id:SV_VertexID):SV_Position { float2 p=float2((id<<1)&2,id&2); return float4(p*float2(2,-2)+float2(-1,1),0,1); }";
pub(super) struct Program {
    pub vertex: ID3DBlob,
    pub pixel: ID3DBlob,
    // Blob vtables come from the compiler DLL; release both Blobs before releasing the module lease.
    _compiler: compiler_api::Compiler,
}
pub(super) struct CompilerSession {
    vertex: ID3DBlob,
    cached: Option<(String, Program)>,
    // cached remains empty on first-compilation failure; the session must also release Blobs before unloading the DLL.
    compiler: compiler_api::Compiler,
}
impl CompilerSession {
    pub fn new() -> Result<Self> {
        let compiler = compiler_api::Compiler::new()?;
        let vertex = compile(&compiler, VERTEX, true)?;
        Ok(Self {
            vertex,
            cached: None,
            compiler,
        })
    }
    pub fn compile_request(
        &mut self,
        request: &Request<'_>,
        stopped: &impl Fn() -> bool,
        deadline: Instant,
    ) -> Result<&Program> {
        check(stopped, deadline)?;
        let full_source = source(request);
        if self
            .cached
            .as_ref()
            .is_none_or(|(previous, _)| previous != &full_source)
        {
            let pixel = compile(&self.compiler, &full_source, false)?;
            reflect::validate(&pixel, request, self.compiler.reflect)?;
            check(stopped, deadline)?;
            let program = Program {
                vertex: self.vertex.clone(),
                pixel,
                _compiler: self.compiler.clone(),
            };
            self.cached = Some((full_source, program));
        } else if let Some((_, program)) = &self.cached {
            reflect::validate(&program.pixel, request, self.compiler.reflect)?;
        }
        Ok(&self.cached.as_ref().unwrap().1)
    }
}
impl Program {
    pub fn new(
        request: &Request<'_>,
        stopped: &impl Fn() -> bool,
        deadline: Instant,
    ) -> Result<Self> {
        check(stopped, deadline)?;
        let compiler = compiler_api::Compiler::new()?;
        let vertex = compile(&compiler, VERTEX, true)?;
        check(stopped, deadline)?;
        let source = source(request);
        let pixel = compile(&compiler, &source, false)?;
        reflect::validate(&pixel, request, compiler.reflect)?;
        check(stopped, deadline)?;
        Ok(Self {
            vertex,
            pixel,
            _compiler: compiler,
        })
    }
}
pub(super) fn source(request: &Request<'_>) -> String {
    let mut text = String::with_capacity(request.source.len() + 16384);
    text.push_str("cbuffer OtterConstants:register(b0) { float2 otter_size:packoffset(c0);\n");
    for (i, uniform) in request.uniforms.iter().enumerate() {
        writeln!(text, "float u_{}:packoffset(c{}.x);", uniform.name, i + 1).unwrap();
    }
    text.push_str("};\nSamplerState otter_sampler:register(s0);\n");
    for (i, texture) in request.textures.iter().enumerate() {
        writeln!(
            text,
            "Texture2D<float4> u_{}:register(t{});",
            texture.name, i
        )
        .unwrap();
    }
    // Fixed entry names prevent remote command-line/path injection. #line supplies only synthetic diagnostic names and opens no files.
    text.push_str("#line 1 \"otter.hlsl\"\n");
    text.push_str(request.source);
    text.push_str("\n#line 1 \"otter.host\"\nfloat4 main(float4 p:SV_Position):SV_Target { return otter_main(p.xy/otter_size); }\n");
    text
}
fn compile(compiler: &compiler_api::Compiler, source: &str, vertex: bool) -> Result<ID3DBlob> {
    let mut code: Option<ID3DBlob> = None;
    let mut diagnostic: Option<ID3DBlob> = None;
    let result = unsafe {
        (compiler.compile)(
            source.as_ptr().cast(),
            source.len(),
            s!("otter.hlsl"),
            std::ptr::null(),
            std::ptr::null_mut(),
            s!("main"),
            if vertex { s!("vs_5_0") } else { s!("ps_5_0") },
            D3DCOMPILE_ENABLE_STRICTNESS
                | D3DCOMPILE_IEEE_STRICTNESS
                | D3DCOMPILE_OPTIMIZATION_LEVEL3,
            0,
            (&mut code as *mut Option<ID3DBlob>).cast(),
            (&mut diagnostic as *mut Option<ID3DBlob>).cast(),
        )
        .ok()
    };
    if let Err(error) = result {
        let mut message = diagnostic
            .as_ref()
            .map(|blob| unsafe {
                let bytes = std::slice::from_raw_parts(
                    blob.GetBufferPointer().cast::<u8>(),
                    blob.GetBufferSize().min(4096),
                );
                String::from_utf8_lossy(bytes)
                    .chars()
                    .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
                    .collect()
            })
            .unwrap_or_else(|| format!("HLSL compiler failed: 0x{:08x}", error.code().0));
        let mut end = message.len().min(4096);
        while !message.is_char_boundary(end) {
            end -= 1;
        }
        message.truncate(end);
        return Err(Error::Compile(message));
    }
    code.ok_or_else(|| Error::Compile("HLSL compiler returned no bytecode".into()))
}
pub(super) fn bytes(blob: &ID3DBlob) -> &[u8] {
    unsafe { std::slice::from_raw_parts(blob.GetBufferPointer().cast(), blob.GetBufferSize()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_compiler_session_releases_vertex_before_library() {
        drop(CompilerSession::new().unwrap());
    }

    #[test]
    fn first_compile_failure_releases_vertex_before_library() {
        let mut session = CompilerSession::new().unwrap();
        crate::shader_hlsl::contract_tests::with_request("not valid HLSL", |request| {
            assert!(matches!(
                session.compile_request(
                    request,
                    &|| false,
                    Instant::now() + std::time::Duration::from_secs(10),
                ),
                Err(Error::Compile(_))
            ));
        });
        assert!(session.cached.is_none());
        drop(session);
    }
}

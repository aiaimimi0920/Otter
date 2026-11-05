//! 桌面任务持有 HLSL 输入的适配层；驱动、隔离、容量与私有协议仍完全归 shader_hlsl。
use crate::{pixels::Pixels, shader_hlsl as hlsl};

pub struct Texture {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Pixels,
}
pub struct Invocation {
    pub source: String,
    pub width: u32,
    pub height: u32,
    pub textures: Vec<Texture>,
    pub uniforms: Vec<(String, f32)>,
}
impl Invocation {
    fn with_request<T>(&self, use_request: impl FnOnce(&hlsl::Request<'_>) -> T) -> T {
        let textures: Vec<_> = self
            .textures
            .iter()
            .map(|texture| hlsl::Texture {
                name: &texture.name,
                width: texture.width,
                height: texture.height,
                rgba: &texture.rgba,
            })
            .collect();
        let uniforms: Vec<_> = self
            .uniforms
            .iter()
            .map(|(name, value)| hlsl::Uniform {
                name,
                value: *value,
            })
            .collect();
        use_request(&hlsl::Request {
            abi: hlsl::ABI,
            source: &self.source,
            width: self.width,
            height: self.height,
            textures: &textures,
            uniforms: &uniforms,
        })
    }
    pub fn validate(&self) -> hlsl::Result<()> {
        if self.textures.len() > 32 || self.uniforms.len() > 256 {
            return Err(hlsl::Error::Budget);
        }
        self.with_request(hlsl::validate)
    }
    pub fn render(
        &self,
        stopped: &impl Fn() -> bool,
    ) -> Result<hlsl::Frame, hlsl::process::Failure> {
        self.validate().map_err(hlsl::process::Failure::Backend)?;
        // 默认只派发一个 Auto child；是否退到 D3D11 由原生设备创建阶段决定。
        self.with_request(|request| hlsl::process::render(request, hlsl::Backend::Auto, stopped))
    }
}

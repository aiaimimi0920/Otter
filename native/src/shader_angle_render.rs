//! 单次编译/纹理/读回事务；资源先注册 owner，所有失败及取消路径都释放 GL 对象。
use super::{api::Gl, image_bytes, name, Error, Frame, Renderer, Request, Result};
use crate::byte_budget::Lease;
use std::{
    ffi::CString,
    sync::atomic::{AtomicBool, Ordering},
};
struct Resources<'a> {
    gl: &'a Gl,
    shaders: Vec<u32>,
    program: u32,
    vao: u32,
    buffer: u32,
    textures: Vec<u32>,
    leases: Vec<Lease>,
}
impl Drop for Resources<'_> {
    fn drop(&mut self) {
        unsafe {
            (self.gl.bind_vertex_array)(0);
            if self.buffer != 0 {
                (self.gl.delete_buffers)(1, &self.buffer);
            }
            if self.vao != 0 {
                (self.gl.delete_vertex_arrays)(1, &self.vao);
            }
            for texture in &self.textures {
                (self.gl.delete_textures)(1, texture);
            }
            (self.gl.use_program)(0);
            if self.program != 0 {
                (self.gl.delete_program)(self.program);
            }
            for shader in &self.shaders {
                (self.gl.delete_shader)(*shader);
            }
        }
        // 字节租约在上述实际 GPU 对象释放之后随字段归还。
    }
}
fn cancelled(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
fn diagnostic(bytes: &[u8], written: i32) -> String {
    let length = usize::try_from(written).unwrap_or(0).min(bytes.len());
    String::from_utf8_lossy(&bytes[..length])
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect()
}
impl Resources<'_> {
    unsafe fn compile(&mut self, kind: u32, source: &str) -> Result<u32> {
        let shader = (self.gl.create_shader)(kind);
        if shader == 0 {
            return Err(Error::Context);
        }
        self.shaders.push(shader);
        let pointer = source.as_ptr().cast();
        let length = source.len() as i32;
        (self.gl.shader_source)(shader, 1, &pointer, &length);
        (self.gl.compile_shader)(shader);
        let mut status = 0;
        (self.gl.shader_parameter)(shader, 0x8B81, &mut status);
        if status == 0 {
            let mut log = [0u8; 4096];
            let mut written = 0;
            (self.gl.shader_log)(
                shader,
                log.len() as i32,
                &mut written,
                log.as_mut_ptr().cast(),
            );
            return Err(Error::Compile(diagnostic(&log, written)));
        }
        Ok(shader)
    }
}
impl Renderer {
    fn validate(&self, request: &Request<'_>) -> Result<(Vec<CString>, Vec<CString>)> {
        if request.vertex.is_empty()
            || request.fragment.is_empty()
            || request.vertex.len() > 1024 * 1024
            || request.fragment.len() > 1024 * 1024
            || request.vertex.contains('\0')
            || request.fragment.contains('\0')
            || request.textures.len() > self.max_units.min(32)
            || request.uniforms.len() > 256
            || !request.textures.iter().any(|texture| {
                texture.name == "input"
                    && texture.width == self.width
                    && texture.height == self.height
            })
        {
            return Err(Error::Layout);
        }
        let mut textures = Vec::new();
        let mut uniforms = Vec::new();
        for texture in request.textures {
            if image_bytes(texture.width, texture.height)? != texture.rgba.len()
                || texture.width > self.max_texture
                || texture.height > self.max_texture
            {
                return Err(Error::Layout);
            }
            let key = name(texture.name)?;
            if textures.contains(&key) {
                return Err(Error::Layout);
            }
            textures.push(key);
        }
        for uniform in request.uniforms {
            let key = name(uniform.name)?;
            if !uniform.value.is_finite() || uniforms.contains(&key) || textures.contains(&key) {
                return Err(Error::Layout);
            }
            uniforms.push(key);
        }
        Ok((textures, uniforms))
    }
    /// 不改写 GLSL 源码/UV，不添加时间或帧循环；取消只能在系统调用间观察。
    /// 返回 raw RGBA，不擅自猜测 WebGL canvas 的预乘/PNG 语义。
    pub fn render(&mut self, request: Request<'_>, cancel: &AtomicBool) -> Result<Frame> {
        cancelled(cancel)?;
        let (texture_names, uniform_names) = self.validate(&request)?;
        self.activate()?;
        let bytes = image_bytes(self.width, self.height)?;
        let mut lease = crate::pixels::reserve(bytes).ok_or(Error::Budget)?;
        let mut output = Vec::new();
        output.try_reserve_exact(bytes).map_err(|_| Error::Budget)?;
        if !lease.grow_to(output.capacity()) {
            return Err(Error::Budget);
        }
        output.resize(bytes, 0);
        let gl = &self.gl;
        let mut objects = Resources {
            gl,
            shaders: Vec::with_capacity(2),
            program: 0,
            vao: 0,
            buffer: 0,
            textures: Vec::with_capacity(request.textures.len()),
            leases: Vec::with_capacity(request.textures.len()),
        };
        unsafe {
            let vertex = objects.compile(0x8B31, request.vertex)?;
            cancelled(cancel)?;
            let fragment = objects.compile(0x8B30, request.fragment)?;
            objects.program = (gl.create_program)();
            if objects.program == 0 {
                return Err(Error::Context);
            }
            (gl.attach_shader)(objects.program, vertex);
            (gl.attach_shader)(objects.program, fragment);
            (gl.link_program)(objects.program);
            let mut status = 0;
            (gl.program_parameter)(objects.program, 0x8B82, &mut status);
            if status == 0 {
                let mut log = [0u8; 4096];
                let mut written = 0;
                (gl.program_log)(
                    objects.program,
                    log.len() as i32,
                    &mut written,
                    log.as_mut_ptr().cast(),
                );
                return Err(Error::Link(diagnostic(&log, written)));
            }
            (gl.use_program)(objects.program);
            (gl.gen_vertex_arrays)(1, &mut objects.vao);
            (gl.bind_vertex_array)(objects.vao);
            (gl.gen_buffers)(1, &mut objects.buffer);
            if objects.vao == 0 || objects.buffer == 0 {
                return Err(Error::Context);
            }
            (gl.bind_buffer)(0x8892, objects.buffer);
            let vertices = [-1.0f32, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0];
            (gl.buffer_data)(
                0x8892,
                std::mem::size_of_val(&vertices) as isize,
                vertices.as_ptr().cast(),
                0x88E4,
            );
            let position = (gl.attribute_location)(objects.program, c"a_position".as_ptr());
            if position < 0 {
                return Err(Error::Layout);
            }
            (gl.enable_attribute)(position as u32);
            (gl.attribute_pointer)(position as u32, 2, 0x1406, 0, 0, std::ptr::null());
            for (unit, (texture, key)) in request
                .textures
                .iter()
                .zip(texture_names.iter())
                .enumerate()
            {
                cancelled(cancel)?;
                objects
                    .leases
                    .push(crate::pixels::reserve(texture.rgba.len()).ok_or(Error::Budget)?);
                let mut handle = 0;
                (gl.gen_textures)(1, &mut handle);
                if handle == 0 {
                    return Err(Error::Context);
                }
                objects.textures.push(handle);
                (gl.active_texture)(0x84C0 + unit as u32);
                (gl.bind_texture)(0x0DE1, handle);
                (gl.texture_image)(
                    0x0DE1,
                    0,
                    0x1908,
                    texture.width as i32,
                    texture.height as i32,
                    0,
                    0x1908,
                    0x1401,
                    texture.rgba.as_ptr().cast(),
                );
                for (parameter, value) in [
                    (0x2802, 0x812F),
                    (0x2803, 0x812F),
                    (0x2801, 0x2601),
                    (0x2800, 0x2601),
                ] {
                    (gl.texture_parameter)(0x0DE1, parameter, value);
                }
                (gl.uniform_i)(
                    (gl.uniform_location)(objects.program, key.as_ptr()),
                    unit as i32,
                );
                self.check()?;
            }
            for (uniform, key) in request.uniforms.iter().zip(uniform_names.iter()) {
                (gl.uniform_f)(
                    (gl.uniform_location)(objects.program, key.as_ptr()),
                    uniform.value,
                );
            }
            cancelled(cancel)?;
            (gl.viewport)(0, 0, self.width as i32, self.height as i32);
            (gl.clear_color)(0.0, 0.0, 0.0, 0.0);
            (gl.clear)(0x4000);
            (gl.draw_arrays)(0x0005, 0, 4);
            self.check()?;
            (gl.read_pixels)(
                0,
                0,
                self.width as i32,
                self.height as i32,
                0x1908,
                0x1401,
                output.as_mut_ptr().cast(),
            );
            self.check()?;
        }
        cancelled(cancel)?;
        let stride = self.width as usize * 4;
        // 只把 readPixels bottom-up 变为 top-down；不改变 shader 自己的 UV 变换。
        for y in 0..self.height as usize / 2 {
            cancelled(cancel)?;
            let opposite = (self.height as usize - 1 - y) * stride;
            let (top, bottom) = output.split_at_mut(opposite);
            top[y * stride..(y + 1) * stride].swap_with_slice(&mut bottom[..stride]);
        }
        Ok(Frame {
            width: self.width,
            height: self.height,
            rgba: crate::pixels::Pixels::from_reserved(output, lease),
        })
    }
}

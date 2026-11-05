//! 已核验 ANGLE 的固定 ABI；不从 shader 字符串、环境 PATH 或任意符号名解析函数。
use super::{library::Library, Result};
use std::ffi::{c_char, c_void, CStr};
pub(super) type Handle = *mut c_void;
macro_rules! api {
    ($kind:ident { $($field:ident: $symbol:literal ($($argument:ty),*) -> $result:ty;)* }) => {
        pub(super) struct $kind { $(pub $field: unsafe extern "system" fn($($argument),*) -> $result,)* }
        impl $kind {
            pub unsafe fn load(library: &Library) -> Result<Self> {
                Ok(Self {$($field: std::mem::transmute::<*const c_void,
                    unsafe extern "system" fn($($argument),*) -> $result>(
                    library.symbol(CStr::from_bytes_with_nul_unchecked(concat!($symbol,"\0").as_bytes()))?),)*})
            }
        }
    }
}
api!(Egl {
    get_proc_address: "eglGetProcAddress" (*const c_char) -> *const c_void;
    initialize: "eglInitialize" (Handle, *mut i32, *mut i32) -> u32;
    choose_config: "eglChooseConfig" (Handle, *const i32, *mut Handle, i32, *mut i32) -> u32;
    bind_api: "eglBindAPI" (u32) -> u32;
    create_surface: "eglCreatePbufferSurface" (Handle, Handle, *const i32) -> Handle;
    create_context: "eglCreateContext" (Handle, Handle, Handle, *const i32) -> Handle;
    make_current: "eglMakeCurrent" (Handle, Handle, Handle, Handle) -> u32;
    destroy_context: "eglDestroyContext" (Handle, Handle) -> u32;
    destroy_surface: "eglDestroySurface" (Handle, Handle) -> u32;
    terminate: "eglTerminate" (Handle) -> u32;
    release_thread: "eglReleaseThread" () -> u32;
    query_string: "eglQueryString" (Handle, i32) -> *const c_char;
});
api!(Gl {
    get_string: "glGetString" (u32) -> *const c_char;
    get_error: "glGetError" () -> u32;
    get_integer: "glGetIntegerv" (u32, *mut i32) -> ();
    create_shader: "glCreateShader" (u32) -> u32;
    shader_source: "glShaderSource" (u32, i32, *const *const c_char, *const i32) -> ();
    compile_shader: "glCompileShader" (u32) -> ();
    shader_parameter: "glGetShaderiv" (u32, u32, *mut i32) -> ();
    shader_log: "glGetShaderInfoLog" (u32, i32, *mut i32, *mut c_char) -> ();
    delete_shader: "glDeleteShader" (u32) -> ();
    create_program: "glCreateProgram" () -> u32;
    attach_shader: "glAttachShader" (u32, u32) -> ();
    link_program: "glLinkProgram" (u32) -> ();
    program_parameter: "glGetProgramiv" (u32, u32, *mut i32) -> ();
    program_log: "glGetProgramInfoLog" (u32, i32, *mut i32, *mut c_char) -> ();
    use_program: "glUseProgram" (u32) -> ();
    delete_program: "glDeleteProgram" (u32) -> ();
    gen_vertex_arrays: "glGenVertexArrays" (i32, *mut u32) -> ();
    bind_vertex_array: "glBindVertexArray" (u32) -> ();
    delete_vertex_arrays: "glDeleteVertexArrays" (i32, *const u32) -> ();
    gen_buffers: "glGenBuffers" (i32, *mut u32) -> ();
    bind_buffer: "glBindBuffer" (u32, u32) -> ();
    buffer_data: "glBufferData" (u32, isize, *const c_void, u32) -> ();
    delete_buffers: "glDeleteBuffers" (i32, *const u32) -> ();
    attribute_location: "glGetAttribLocation" (u32, *const c_char) -> i32;
    enable_attribute: "glEnableVertexAttribArray" (u32) -> ();
    attribute_pointer: "glVertexAttribPointer" (u32, i32, u32, u8, i32, *const c_void) -> ();
    gen_textures: "glGenTextures" (i32, *mut u32) -> ();
    active_texture: "glActiveTexture" (u32) -> ();
    bind_texture: "glBindTexture" (u32, u32) -> ();
    texture_image: "glTexImage2D" (u32, i32, i32, i32, i32, i32, u32, u32, *const c_void) -> ();
    texture_parameter: "glTexParameteri" (u32, u32, i32) -> ();
    delete_textures: "glDeleteTextures" (i32, *const u32) -> ();
    uniform_location: "glGetUniformLocation" (u32, *const c_char) -> i32;
    uniform_i: "glUniform1i" (i32, i32) -> ();
    uniform_f: "glUniform1f" (i32, f32) -> ();
    viewport: "glViewport" (i32, i32, i32, i32) -> ();
    clear_color: "glClearColor" (f32, f32, f32, f32) -> ();
    clear: "glClear" (u32) -> ();
    draw_arrays: "glDrawArrays" (u32, i32, i32) -> ();
    read_pixels: "glReadPixels" (i32, i32, i32, i32, u32, u32, *mut c_void) -> ();
});

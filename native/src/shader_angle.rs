//! 原样 GLSL ES3 离屏后端；只允许专属 worker 线程拥有，尚未接入应用能力声明。
//! 编译/驱动调用不能可靠地在线程内中断；应用接线前必须由有期限的隔离 child 承载。
use crate::pixels::Pixels;
use std::{
    cell::Cell,
    ffi::{CStr, CString},
    marker::PhantomData,
    path::Path,
    ptr::null_mut,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};
#[path = "shader_angle_api.rs"]
mod api;
#[path = "shader_angle_library.rs"]
mod library;
#[path = "shader_angle_render.rs"]
mod render;
pub use library::{Fingerprint, Identity};
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Library,
    Context,
    ContextBusy,
    Poisoned,
    Layout,
    Budget,
    Cancelled,
    Api(u32),
    Compile(String),
    Link(String),
}
#[derive(Clone, Copy, Debug)]
pub enum Backend {
    D3d11,
    Warp,
}
pub struct Texture<'a> {
    pub name: &'a str,
    pub width: u32,
    pub height: u32,
    pub rgba: &'a [u8],
}
pub struct Uniform<'a> {
    pub name: &'a str,
    pub value: f32,
}
pub struct Request<'a> {
    pub vertex: &'a str,
    pub fragment: &'a str,
    pub textures: &'a [Texture<'a>],
    pub uniforms: &'a [Uniform<'a>],
}
/// top-down 原始 RGBA，与 GL framebuffer 一致；不是可直接交给 D2D 的 PBGRA。
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Pixels,
}
#[derive(Debug)]
pub struct Description {
    pub egl: String,
    pub renderer: String,
    pub version: String,
}
pub struct Renderer {
    egl: api::Egl,
    gl: api::Gl,
    display: api::Handle,
    context: api::Handle,
    surface: api::Handle,
    width: u32,
    height: u32,
    max_texture: u32,
    max_units: usize,
    poisoned: Cell<bool>,
    _libraries: library::Libraries,
    _surface_budget: crate::byte_budget::Lease,
    _thread: ContextLease,
}
static CONTEXT_OWNED: AtomicBool = AtomicBool::new(false);
// 每个隔离进程只准一个 owner：同线程会替换 current，跨线程 eglTerminate 会影响共享 display。
struct ContextLease(PhantomData<Rc<()>>);
impl ContextLease {
    fn acquire() -> Result<Self> {
        CONTEXT_OWNED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| Self(PhantomData))
            .map_err(|_| Error::ContextBusy)
    }
}
impl Drop for ContextLease {
    fn drop(&mut self) {
        CONTEXT_OWNED.store(false, Ordering::Release);
    }
}
pub(super) fn image_bytes(width: u32, height: u32) -> Result<usize> {
    crate::viewport::checked_image_bytes(width, height).ok_or(Error::Layout)
}
pub(super) fn name(value: &str) -> Result<CString> {
    if value.is_empty()
        || value.len() > 128
        || !value.bytes().enumerate().all(|(index, c)| {
            c.is_ascii_alphabetic() || c == b'_' || (index > 0 && c.is_ascii_digit())
        })
    {
        return Err(Error::Layout);
    }
    CString::new(format!("u_{value}")).map_err(|_| Error::Layout)
}
impl Renderer {
    /// identity 必须是可信发行目录的固定指纹，不得采用远端参数或环境 PATH 推断。
    /// EGL 上下文由专属线程独占；此函数不应在 UI 线程调用。
    pub fn new(
        directory: &Path,
        identity: Identity,
        width: u32,
        height: u32,
        backend: Backend,
    ) -> Result<Self> {
        let surface_budget =
            crate::pixels::reserve(image_bytes(width, height)?).ok_or(Error::Budget)?;
        let libraries = library::Libraries::load(directory, identity)?;
        let thread = ContextLease::acquire()?;
        unsafe {
            let egl = api::Egl::load(&libraries.egl)?;
            let gl = api::Gl::load(&libraries.gles)?;
            let mut renderer = Self {
                egl,
                gl,
                display: null_mut(),
                context: null_mut(),
                surface: null_mut(),
                width,
                height,
                max_texture: 0,
                max_units: 0,
                poisoned: Cell::new(false),
                _libraries: libraries,
                _surface_budget: surface_budget,
                _thread: thread,
            };
            renderer.initialize(backend)?;
            Ok(renderer)
        }
    }
    unsafe fn initialize(&mut self, backend: Backend) -> Result<()> {
        let address = (self.egl.get_proc_address)(c"eglGetPlatformDisplayEXT".as_ptr());
        if address.is_null() {
            return Err(Error::Context);
        }
        let platform = std::mem::transmute::<
            *const std::ffi::c_void,
            unsafe extern "system" fn(u32, api::Handle, *const i32) -> api::Handle,
        >(address);
        let device = match backend {
            Backend::D3d11 => 0x320A,
            Backend::Warp => 0x320B,
        };
        self.display = platform(
            0x3202,
            null_mut(),
            [0x3203, 0x3208, 0x3209, device, 0x3038].as_ptr(),
        );
        let mut major = 0;
        let mut minor = 0;
        if self.display.is_null()
            || (self.egl.initialize)(self.display, &mut major, &mut minor) == 0
            || (self.egl.bind_api)(0x30A0) == 0
        {
            return Err(Error::Context);
        }
        let mut config = null_mut();
        let mut count = 0;
        let attrs = [
            0x3033, 1, 0x3040, 0x40, 0x3024, 8, 0x3023, 8, 0x3022, 8, 0x3021, 8, 0x3025, 0, 0x3026,
            0, 0x3038,
        ];
        if (self.egl.choose_config)(self.display, attrs.as_ptr(), &mut config, 1, &mut count) == 0
            || count != 1
        {
            return Err(Error::Context);
        }
        self.context = (self.egl.create_context)(
            self.display,
            config,
            null_mut(),
            [0x3098, 3, 0x33AC, 1, 0x3453, 1, 0x3038].as_ptr(),
        );
        self.surface = (self.egl.create_surface)(
            self.display,
            config,
            [
                0x3057,
                self.width as i32,
                0x3056,
                self.height as i32,
                0x3038,
            ]
            .as_ptr(),
        );
        if self.context.is_null()
            || self.surface.is_null()
            || (self.egl.make_current)(self.display, self.surface, self.surface, self.context) == 0
        {
            return Err(Error::Context);
        }
        let mut maximum = 0;
        let mut units = 0;
        (self.gl.get_integer)(0x0D33, &mut maximum);
        (self.gl.get_integer)(0x8872, &mut units);
        if maximum <= 0 || units <= 0 {
            return Err(Error::Context);
        }
        self.max_texture = maximum as u32;
        self.max_units = units as usize;
        self.check()
    }
    pub fn description(&self) -> Result<Description> {
        self.activate()?;
        unsafe fn text(value: *const std::ffi::c_char) -> Result<String> {
            if value.is_null() {
                return Err(Error::Context);
            }
            Ok(CStr::from_ptr(value)
                .to_string_lossy()
                .chars()
                .take(1024)
                .collect())
        }
        unsafe {
            Ok(Description {
                egl: text((self.egl.query_string)(self.display, 0x3054))?,
                renderer: text((self.gl.get_string)(0x1F01))?,
                version: text((self.gl.get_string)(0x1F02))?,
            })
        }
    }
    fn check(&self) -> Result<()> {
        if self.poisoned.get() {
            return Err(Error::Poisoned);
        }
        let mut first = 0;
        // 不把上一事务的 GL 错误留给下一请求；任何 API 错误都永久退休本 context。
        for _ in 0..16 {
            let code = unsafe { (self.gl.get_error)() };
            if code == 0 {
                break;
            }
            if first == 0 {
                first = code;
            }
        }
        if first == 0 {
            return Ok(());
        }
        self.poisoned.set(true);
        Err(Error::Api(first))
    }
    fn activate(&self) -> Result<()> {
        if self.poisoned.get() {
            return Err(Error::Poisoned);
        }
        if unsafe {
            (self.egl.make_current)(self.display, self.surface, self.surface, self.context)
        } == 0
        {
            self.poisoned.set(true);
            return Err(Error::Context);
        }
        self.check()
    }
}
impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            if !self.display.is_null() {
                (self.egl.make_current)(self.display, null_mut(), null_mut(), null_mut());
                if !self.context.is_null() {
                    (self.egl.destroy_context)(self.display, self.context);
                }
                if !self.surface.is_null() {
                    (self.egl.destroy_surface)(self.display, self.surface);
                }
                (self.egl.terminate)(self.display);
                (self.egl.release_thread)();
            }
        }
    }
}
#[cfg(test)]
#[path = "shader_angle_contract_tests.rs"]
pub(crate) mod contract_tests;
#[cfg(test)]
#[path = "shader_angle_tests.rs"]
pub(crate) mod tests;

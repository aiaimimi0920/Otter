//! 编辑栏字体全集：迁移原 GDI 枚举，精确去重并按中文、忽略大小写/重音和数字值排序。
//! 只枚举名称，不读取字体文件；UI 线程成功缓存，失败不缓存，后续可重新展开。
use std::{cell::RefCell, collections::BTreeSet, rc::Rc};
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, Graphics::Gdi::*},
};
#[path = "installed_font_order.rs"]
mod order;
const PRESETS: [&str; 9] = [
    "微软雅黑",
    "宋体",
    "黑体",
    "楷体",
    "仿宋",
    "Arial",
    "Times New Roman",
    "Consolas",
    "Segoe UI",
];
const MAX_FAMILIES: usize = 16_384;
#[derive(Default)]
struct Catalog(Option<Rc<[String]>>);
impl Catalog {
    fn get(&mut self, load: impl FnOnce() -> Result<Vec<String>>) -> Result<Rc<[String]>> {
        if let Some(value) = &self.0 {
            return Ok(value.clone());
        }
        let value: Rc<[String]> = merge(load()?)?.into();
        self.0 = Some(value.clone());
        Ok(value)
    }
}
thread_local! { static CATALOG: RefCell<Catalog> = RefCell::new(Catalog::default()); }
pub(super) fn list() -> Result<Rc<[String]>> {
    CATALOG.with(|c| c.borrow_mut().get(enumerate))
}
struct Scan {
    names: BTreeSet<String>,
    overflow: bool,
}
struct Dc(HDC);
impl Drop for Dc {
    fn drop(&mut self) {
        unsafe {
            ReleaseDC(None, self.0);
        }
    }
}
unsafe extern "system" fn collect(
    font: *const LOGFONTW,
    _: *const TEXTMETRICW,
    _: u32,
    lp: LPARAM,
) -> i32 {
    // GDI 在本同步枚举栈内调用，指针与 Scan 都由 enumerate 持有。
    let scan = &mut *(lp.0 as *mut Scan);
    let face = &(*font).lfFaceName;
    let len = face.iter().position(|v| *v == 0).unwrap_or(face.len());
    let name = String::from_utf16_lossy(&face[..len]);
    let name = name.trim();
    if !name.is_empty() && !name.starts_with('@') {
        scan.names.insert(name.to_owned());
    }
    if scan.names.len() > MAX_FAMILIES {
        scan.overflow = true;
        return 0;
    }
    1
}
fn enumerate() -> Result<Vec<String>> {
    unsafe {
        let dc = Dc(GetDC(None));
        if dc.0.is_invalid() {
            return Err(Error::from_win32());
        }
        let filter = LOGFONTW {
            lfCharSet: DEFAULT_CHARSET,
            ..Default::default()
        };
        let mut scan = Scan {
            names: BTreeSet::new(),
            overflow: false,
        };
        EnumFontFamiliesExW(
            dc.0,
            &filter,
            Some(collect),
            LPARAM(&mut scan as *mut Scan as isize),
            0,
        );
        if scan.overflow {
            return Err(Error::new(
                E_BOUNDS,
                "Installed font catalog exceeds 16384 families",
            ));
        }
        Ok(scan.names.into_iter().collect())
    }
}
fn merge(installed: Vec<String>) -> Result<Vec<String>> {
    let mut seen: BTreeSet<_> = PRESETS.map(str::to_owned).into();
    let mut names = Vec::new();
    for value in installed {
        let name = value.trim();
        if !name.is_empty() && seen.insert(name.to_owned()) {
            names.push(name.to_owned());
        }
    }
    let mut result = PRESETS.map(str::to_owned).to_vec();
    if result.len() + names.len() > MAX_FAMILIES {
        return Err(Error::new(E_BOUNDS, "Font catalog exceeds 16384 families"));
    }
    result.extend(order::sorted(names)?);
    Ok(result)
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "installed_fonts_tests.rs"]
mod tests;

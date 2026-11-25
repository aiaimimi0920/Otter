//! On-demand system-font sources for SVG children, connected to usvg through `surface_svg_text_fonts`.
use std::{collections::HashMap, io, path::PathBuf, sync::Arc};
use windows::{
    core::{Interface, BOOL, PCWSTR},
    Win32::Graphics::DirectWrite::*,
};
#[path = "surface_svg_font_fallback.rs"]
pub mod fallback;
#[path = "surface_svg_font_file.rs"]
pub mod file;
pub use fallback::{Mapped, Properties};
const MAX_QUERIES: usize = 256;
const MAX_BYTES: usize = 48 * 1024 * 1024;
const MAX_FILES: usize = 16;

/// Source for one face; callers must retain collection indexes and simulation flags instead of loading only face zero.
pub struct Face {
    pub bytes: Arc<Vec<u8>>,
    pub index: u32,
    pub simulations: u32,
}
/// Never enumerate/preload system fonts or accept font-file paths; each SVG request owns this budget independently.
pub struct Fonts {
    collection: IDWriteFontCollection,
    fallback: IDWriteFontFallback,
    root: file::Root,
    files: HashMap<PathBuf, Arc<Vec<u8>>>,
    bytes: usize,
    queries: usize,
}
impl Fonts {
    /// Creation is allowed only inside existing production-grade image Jobs; parents still own deadlines and termination/cleanup after cancellation.
    pub fn new() -> io::Result<Self> {
        crate::surface_image_worker_host::require_limits()?;
        let root = file::Root::system()?;
        let (collection, fallback) = unsafe {
            let factory: IDWriteFactory2 =
                DWriteCreateFactory(DWRITE_FACTORY_TYPE_ISOLATED).map_err(io::Error::other)?;
            let mut collection = None;
            factory
                .GetSystemFontCollection(&mut collection, false)
                .map_err(io::Error::other)?;
            (
                collection.ok_or_else(file::invalid)?,
                factory.GetSystemFontFallback().map_err(io::Error::other)?,
            )
        };
        Ok(Self {
            collection,
            fallback,
            root,
            files: HashMap::new(),
            bytes: 0,
            queries: 0,
        })
    }
    /// style: 0 normal, 1 oblique, 2 italic; stretch follows DirectWrite 1..=9.
    /// Uninstalled families return None; this does not mean missing-glyph fallback is complete.
    pub fn select(
        &mut self,
        family: &str,
        weight: u16,
        stretch: u8,
        style: u8,
    ) -> io::Result<Option<Face>> {
        self.begin_query(family, weight, stretch, style)?;
        let name: Vec<u16> = family.encode_utf16().chain(Some(0)).collect();
        unsafe {
            let (mut index, mut exists) = (0, BOOL(0));
            self.collection
                .FindFamilyName(PCWSTR(name.as_ptr()), &mut index, &mut exists)
                .map_err(io::Error::other)?;
            if !exists.as_bool() {
                return Ok(None);
            }
            let family = self
                .collection
                .GetFontFamily(index)
                .map_err(io::Error::other)?;
            let font = family
                .GetFirstMatchingFont(
                    DWRITE_FONT_WEIGHT(i32::from(weight)),
                    DWRITE_FONT_STRETCH(i32::from(stretch)),
                    DWRITE_FONT_STYLE(i32::from(style)),
                )
                .map_err(io::Error::other)?;
            self.load_font(font).map(Some)
        }
    }
    fn begin_query(&mut self, family: &str, weight: u16, stretch: u8, style: u8) -> io::Result<()> {
        if self.queries >= MAX_QUERIES {
            return Err(file::invalid());
        }
        self.queries += 1;
        if family.is_empty()
            || family.len() > 1024
            || family.chars().any(char::is_control)
            || !(1..=999).contains(&weight)
            || !(1..=9).contains(&stretch)
            || style > 2
        {
            return Err(file::invalid());
        }
        Ok(())
    }
    fn load_font(&mut self, font: IDWriteFont) -> io::Result<Face> {
        unsafe {
            // Obtain only the reference without creating/parsing the face first; the OS still manages collection metadata.
            let font: IDWriteFont3 = font.cast().map_err(io::Error::other)?;
            let face = font.GetFontFaceReference().map_err(io::Error::other)?;
            let path = face_path(&face)?;
            let bytes = match self.files.get(&path) {
                Some(bytes) => Arc::clone(bytes),
                None => {
                    if self.files.len() >= MAX_FILES {
                        return Err(file::invalid());
                    }
                    let bytes = Arc::new(self.root.read(&path, MAX_BYTES - self.bytes)?);
                    self.bytes += bytes.len();
                    self.files.insert(path, Arc::clone(&bytes));
                    bytes
                }
            };
            Ok(Face {
                bytes,
                index: face.GetFontFaceIndex(),
                simulations: face.GetSimulations().0 as u32,
            })
        }
    }
}
unsafe fn face_path(face: &IDWriteFontFaceReference) -> io::Result<PathBuf> {
    let file = face.GetFontFile().map_err(io::Error::other)?;
    let loader: IDWriteLocalFontFileLoader = file
        .GetLoader()
        .and_then(|l| l.cast())
        .map_err(io::Error::other)?;
    let (mut key, mut size) = (std::ptr::null_mut(), 0);
    file.GetReferenceKey(&mut key, &mut size)
        .map_err(io::Error::other)?;
    if key.is_null() || size == 0 || size > 65536 {
        return Err(file::invalid());
    }
    let length = loader
        .GetFilePathLengthFromKey(key, size)
        .map_err(io::Error::other)? as usize;
    if length == 0 || length > 32767 {
        return Err(file::invalid());
    }
    let mut path = vec![0u16; length + 1];
    loader
        .GetFilePathFromKey(key, size, &mut path)
        .map_err(io::Error::other)?;
    if path[length] != 0 || path[..length].contains(&0) {
        return Err(file::invalid());
    }
    use std::os::windows::ffi::OsStringExt;
    Ok(PathBuf::from(std::ffi::OsString::from_wide(
        &path[..length],
    )))
}
#[cfg(test)]
#[path = "surface_svg_fonts_tests.rs"]
mod tests;

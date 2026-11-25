//! Immutable image snapshots, atomic batch preparation, and pre-delivery freshness checks; resource descriptors borrow the entire batch owner.
use super::super::loom_input_stamp::SourceStamp;
#[cfg(test)]
#[path = "loom_input_output_source_tests.rs"]
mod output_tests;
use super::*;
#[path = "loom_input_data.rs"]
mod data;
use data::Data;
#[path = "loom_input_fingerprint.rs"]
mod fingerprint;
pub struct Source {
    fingerprint: bool,
    dependencies: Vec<(UnitId, SourceStamp)>,
    port: String,
    unit: UnitId,
    stamp: SourceStamp,
    data: Data,
}
impl Source {
    pub(crate) fn fingerprint_for(&mut self, port: &str) {
        self.fingerprint |= self.port == port;
    }
    pub(crate) fn capture_live(
        port: String,
        unit: UnitId,
        identity: Arc<()>,
        frame: Arc<crate::live_frames::Frame<crate::live_texture::Texture>>,
    ) -> Result<Self, Error> {
        if port.is_empty() || port.len() > 256 || port.chars().any(char::is_control) {
            return Err(Error::InvalidInput);
        }
        let data = Data::Live(frame);
        data.bytes()?;
        Ok(Self {
            fingerprint: false,
            dependencies: Vec::new(),
            port,
            unit,
            stamp: SourceStamp::Live(identity),
            data,
        })
    }
    pub fn capture_inline(
        port: String,
        unit: UnitId,
        uri: &str,
        editor: &Editor,
    ) -> Result<Self, Error> {
        if port.is_empty()
            || port.len() > 256
            || port.chars().any(char::is_control)
            || editor.draft().is_some()
        {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            fingerprint: false,
            dependencies: Vec::new(),
            port,
            unit,
            stamp: SourceStamp::Document(editor.document_stamp()),
            data: Data::inline(uri)?,
        })
    }
    pub(crate) fn depend_on(&mut self, state: &super::super::State) -> Result<(), Error> {
        let canvas = state.canvas.borrow();
        let editor = &canvas.editor;
        if self.dependencies.len() >= crate::workspace::MAX_UNITS || editor.draft().is_some() {
            return Err(Error::InvalidInput);
        }
        self.dependencies
            .try_reserve(1)
            .map_err(|_| Error::Allocation)?;
        self.dependencies.push((
            state.unit_id.clone(),
            SourceStamp::Node {
                document: editor.document_stamp(),
                configuration: super::super::configuration_stamp::Stamp::capture(state),
            },
        ));
        Ok(())
    }
    pub fn capture(
        port: String,
        unit: UnitId,
        width: u32,
        height: u32,
        pixels: Pixels,
        editor: &Editor,
    ) -> Result<Self, Error> {
        Self::capture_raster(port, unit, width, height, pixels, editor, true)
    }
    /// Shader's own src uses base images only; display owners separately own previews/annotations, which must not be rebaked into u_input.
    pub(crate) fn capture_base(port: String, unit: UnitId, editor: &Editor) -> Result<Self, Error> {
        // Canvas.image may be only a restored-session preview; only the editing owner's real base image may serve as src.
        let base = editor.layers().ok_or(Error::InvalidInput)?;
        Self::capture_raster(
            port,
            unit,
            base.size.0,
            base.size.1,
            base.base.clone(),
            editor,
            false,
        )
    }
    fn capture_raster(
        port: String,
        unit: UnitId,
        width: u32,
        height: u32,
        pixels: Pixels,
        editor: &Editor,
        compose: bool,
    ) -> Result<Self, Error> {
        if port.is_empty()
            || port.len() > 256
            || port.chars().any(char::is_control)
            || editor.draft().is_some()
        {
            return Err(Error::InvalidInput);
        }
        if crate::viewport::checked_image_bytes(width, height) != Some(pixels.len()) {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            fingerprint: false,
            dependencies: Vec::new(),
            port,
            unit,
            stamp: SourceStamp::Document(editor.document_stamp()),
            data: Data::Raster {
                width,
                height,
                pixels,
                editor: if compose {
                    Box::new(editor.export_snapshot())
                } else {
                    Box::default()
                },
            },
        })
    }
    /// Manual inputs do not bake source-canvas annotations; share the same document/configuration freshness boundary with original images.
    pub(crate) fn capture_sticker(
        port: String,
        unit: UnitId,
        image: &crate::desktop::image::Image,
        editor: &Editor,
    ) -> Result<Self, Error> {
        Self::capture_raster(
            port,
            unit,
            image.width,
            image.height,
            image.pixels.clone(),
            editor,
            false,
        )
    }
    /// Official outputs already contain complete images; never overlay source-window annotations or use previews, and transfer original PNGs too.
    pub fn capture_output(
        port: String,
        unit: UnitId,
        source: super::super::loom_result_state::ImageSource,
    ) -> Result<Self, Error> {
        let image = source.image;
        let mut result = Self::capture(
            port,
            unit,
            image.width,
            image.height,
            image.pixels.clone(),
            &Editor::default(),
        )?;
        result.data = Data::Formal(image);
        result.stamp = SourceStamp::Output(source.stamp);
        Ok(result)
    }
}
struct Resource {
    fingerprint: Option<[u8; 32]>,
    dependencies: Vec<(UnitId, SourceStamp)>,
    port: String,
    unit: UnitId,
    stamp: SourceStamp,
    image: Inline,
}
pub struct PreparedBatch {
    dependencies: Vec<(UnitId, SourceStamp)>,
    pub target: UnitId,
    target_stamp: DocumentStamp,
    resources: Vec<Resource>,
}
/// Retain bounded identity evidence only, without extending encoded-input/pixel-snapshot lifetimes.
pub(crate) struct Freshness {
    documents: Vec<(UnitId, SourceStamp)>,
}
impl Freshness {
    pub(crate) fn is_current(&self, mut check: impl FnMut(&UnitId, &SourceStamp) -> bool) -> bool {
        self.documents
            .iter()
            .all(|(unit, stamp)| check(unit, stamp))
    }
}
impl PreparedBatch {
    pub(crate) fn pngs(&self) -> impl ExactSizeIterator<Item = crate::pixels::Pixels> + '_ {
        self.resources.iter().map(|resource| resource.image.png())
    }
    pub(crate) fn fingerprint(&self, port: &str) -> Option<[u8; 32]> {
        self.resources
            .iter()
            .find(|resource| resource.port == port)?
            .fingerprint
    }
    pub(crate) fn set_dependencies(
        &mut self,
        dependencies: Vec<(UnitId, SourceStamp)>,
    ) -> Result<(), Error> {
        if dependencies.len() > crate::workspace::MAX_UNITS * 2 {
            return Err(Error::InvalidInput);
        }
        self.dependencies = dependencies;
        Ok(())
    }
    pub(crate) fn freshness(&self) -> Result<Freshness, Error> {
        let mut documents = Vec::new();
        documents
            .try_reserve(self.resources.len() + 1)
            .map_err(|_| Error::Allocation)?;
        documents.push((
            self.target.clone(),
            SourceStamp::Document(self.target_stamp.clone()),
        ));
        documents
            .try_reserve(self.dependencies.len())
            .map_err(|_| Error::Allocation)?;
        documents.extend(self.dependencies.iter().cloned());
        for resource in &self.resources {
            documents
                .try_reserve(resource.dependencies.len())
                .map_err(|_| Error::Allocation)?;
            documents.extend(resource.dependencies.iter().cloned());
        }
        documents
            .try_reserve(self.resources.len())
            .map_err(|_| Error::Allocation)?;
        documents.extend(
            self.resources
                .iter()
                .map(|resource| (resource.unit.clone(), resource.stamp.clone())),
        );
        Ok(Freshness { documents })
    }
    #[cfg(test)]
    pub(crate) fn empty_for_test(target: UnitId, editor: &Editor) -> Self {
        Self {
            dependencies: Vec::new(),
            target,
            target_stamp: editor.document_stamp(),
            resources: Vec::new(),
        }
    }
    pub fn descriptors(
        &self,
    ) -> impl ExactSizeIterator<Item = (&str, crate::loom_art_wire::Input<'_>)> {
        self.resources
            .iter()
            .map(|resource| (resource.port.as_str(), resource.image.descriptor()))
    }
    pub fn is_current<'a>(&self, mut lookup: impl FnMut(&UnitId) -> Option<&'a Editor>) -> bool {
        let current = |stamp: &DocumentStamp, editor: &Editor| {
            stamp.matches(editor) && editor.draft().is_none()
        };
        lookup(&self.target).is_some_and(|editor| current(&self.target_stamp, editor))
            && self.dependencies.iter().all(|(unit, stamp)| {
                lookup(unit).is_some_and(|editor| stamp.matches(editor) && editor.draft().is_none())
            })
            && self.resources.iter().all(|resource| {
                lookup(&resource.unit).is_some_and(|editor| {
                    resource.stamp.matches(editor) && editor.draft().is_none()
                }) && resource.dependencies.iter().all(|(unit, stamp)| {
                    lookup(unit)
                        .is_some_and(|editor| stamp.matches(editor) && editor.draft().is_none())
                })
            })
    }
}
pub(super) fn validate(sources: &[Source]) -> Result<(), Error> {
    if sources.len() > 32 {
        return Err(Error::InvalidInput);
    }
    let mut total = 0usize;
    for (index, source) in sources.iter().enumerate() {
        if sources[..index]
            .iter()
            .any(|previous| previous.port == source.port)
        {
            return Err(Error::InvalidInput);
        }
        total = total
            .checked_add(source.data.bytes()?)
            .ok_or(Error::Budget)?;
        if total > 256 * 1024 * 1024 {
            return Err(Error::Budget);
        }
    }
    Ok(())
}
pub(super) fn prepare(
    target: UnitId,
    target_stamp: DocumentStamp,
    sources: Vec<Source>,
    cancel: &AtomicBool,
) -> Result<PreparedBatch, Error> {
    validate(&sources)?;
    let mut resources = Vec::new();
    resources
        .try_reserve(sources.len())
        .map_err(|_| Error::Allocation)?;
    let mut encoded = 0usize;
    let mut actual = 0usize;
    for source in sources {
        if cancel.load(Ordering::Acquire) {
            return Err(Error::Cancelled);
        }
        let image = source.data.prepare(cancel, &mut actual)?;
        let crate::loom_art_wire::Input::InlinePng { base64, .. } = image.descriptor() else {
            return Err(Error::InvalidInput);
        };
        encoded = encoded.checked_add(base64.len()).ok_or(Error::Budget)?;
        if encoded > (64usize * 1024 * 1024).div_ceil(3) * 4 * 2 {
            return Err(Error::Budget);
        }
        resources.push(Resource {
            fingerprint: source
                .fingerprint
                .then(|| fingerprint::capture(&image, cancel))
                .transpose()?,
            dependencies: source.dependencies,
            port: source.port,
            unit: source.unit,
            stamp: source.stamp,
            image,
        });
    }
    if cancel.load(Ordering::Acquire) {
        return Err(Error::Cancelled);
    }
    Ok(PreparedBatch {
        dependencies: Vec::new(),
        target,
        target_stamp,
        resources,
    })
}

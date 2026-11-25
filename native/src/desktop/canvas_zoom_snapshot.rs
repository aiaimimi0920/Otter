//! Zoom previews own independent software targets and reuse official Canvas drawing; never replace host GPU resources or documents.
use super::*;

impl Canvas {
    pub(in crate::desktop) fn zoom_snapshot(
        &self,
        frame: Option<&crate::desktop::display_frame::Frame>,
    ) -> Result<Option<Self>> {
        let (image, editor) = match crate::desktop::display_snapshot::capture(self, frame) {
            Some((image, editor)) => (Some(image), editor),
            None if frame.is_none() && self.image.is_none() && self.editor.draft().is_none() => {
                (None, self.editor.export_snapshot())
            }
            None => return Ok(None),
        };
        let mut copy = Self::new(None)?;
        copy.image = image;
        copy.editor = editor;
        copy.pinned = true;
        copy.software_renderer = true;
        copy.viewport = self.viewport;
        copy.window_opacity = self.window_opacity;
        copy.frame_style = self.frame_style;
        copy.extension_views = self
            .extension_views
            .as_ref()
            .and_then(|views| views.rebind(self, &copy, frame));
        copy.tool = self.tool;
        if frame.is_none() {
            copy.selected = self.selected;
            copy.multi_selection = self.multi_selection.clone();
            copy.transform_own = self.transform_own;
        }
        if let Some(minified) = &self.minified {
            let (views, size) = minified.geometry();
            copy.minified = Some(minified::Presentation::restored(
                minified.image().cloned(),
                views,
                size,
                &copy.editor,
            )?);
        }
        Ok(Some(copy))
    }
}

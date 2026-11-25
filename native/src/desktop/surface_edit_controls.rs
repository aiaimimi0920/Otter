//! Mounted-field HWND identity and focus routing; access existing owners only without another window-index cache.
use super::*;
impl Mount {
    pub fn owns(&self, hwnd: HWND) -> bool {
        self.id(hwnd).is_some()
    }
    pub(in crate::desktop) fn hwnd(&self, id: &str) -> Option<HWND> {
        self.fields
            .get(id)
            .and_then(|field| field.session.control().hwnd().ok())
            .or_else(|| {
                self.selects
                    .get(id)
                    .and_then(|field| field.control.hwnd().ok())
            })
            .or_else(|| {
                self.sliders
                    .get(id)
                    .and_then(|field| field.control.hwnd().ok())
            })
    }
    pub fn id(&self, hwnd: HWND) -> Option<&str> {
        self.fields
            .iter()
            .find(|(_, f)| f.session.control().hwnd().ok() == Some(hwnd))
            .map(|(id, _)| id.as_str())
            .or_else(|| {
                self.selects
                    .iter()
                    .find(|(_, f)| f.control.hwnd().ok() == Some(hwnd))
                    .map(|(id, _)| id.as_str())
            })
            .or_else(|| {
                self.sliders
                    .iter()
                    .find(|(_, f)| f.control.hwnd().ok() == Some(hwnd))
                    .map(|(id, _)| id.as_str())
            })
    }
    pub fn focus(&mut self, id: &str) -> Result<bool> {
        if let Some(field) = self.sliders.get(id) {
            if !self.editing || field.disabled || !field.visible {
                return Ok(false);
            }
            field.control.focus()?;
            return Ok(true);
        }
        if let Some(field) = self.selects.get(id) {
            if !self.editing || field.disabled || !field.visible {
                return Ok(false);
            }
            field.control.focus()?;
            return Ok(true);
        }
        let Some(field) = self.fields.get(id) else {
            return Ok(false);
        };
        if !self.editing || field.disabled {
            return Ok(false);
        }
        field.session.control().focus()?;
        Ok(true)
    }
}

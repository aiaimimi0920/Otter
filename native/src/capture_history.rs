//! 截图/取色便利历史；与系统剪贴板历史和工作区恢复完全独立。
use crate::stroke_style::Color;
use serde::{Deserialize, Serialize};

#[path = "capture_history_image.rs"]
pub(crate) mod image;
#[path = "capture_history_store.rs"]
pub(crate) mod store;
pub(crate) const LIMIT: usize = 24;
pub(crate) const MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Picked {
    pub hex: String,
    pub rgb: Rgb,
    pub at: u64,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Screenshot {
    pub id: String,
    pub thumbnail: String,
    pub width: u32,
    pub height: u32,
    pub at: u64,
    #[serde(skip)]
    pub preview: Option<image::Preview>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub(crate) struct Data {
    pub colors: Vec<Picked>,
    pub screenshots: Vec<Screenshot>,
    // 保留文件根的未知扩展；不从其中执行路径、网络或业务动作。
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}
impl Data {
    pub fn color(&mut self, color: Color, at: u64) {
        let hex = format!("#{:02x}{:02x}{:02x}", color.0, color.1, color.2);
        self.colors.retain(|old| old.hex != hex);
        self.colors.insert(
            0,
            Picked {
                hex,
                rgb: Rgb {
                    r: color.0,
                    g: color.1,
                    b: color.2,
                },
                at,
            },
        );
        self.colors.truncate(LIMIT);
    }
    pub fn screenshot(&mut self, entry: Screenshot) {
        self.screenshots.retain(|old| old.id != entry.id);
        self.screenshots.insert(0, entry);
        self.screenshots.truncate(LIMIT);
    }
    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.screenshots.len();
        self.screenshots.retain(|entry| entry.id != id);
        before != self.screenshots.len()
    }
    /// 后台调用；JSON根损坏是错误，不允许把原文件当空缓存覆盖。
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        Self::decode_with(bytes, image::decode)
    }
    fn decode_with(
        bytes: &[u8],
        prepare: impl Fn(&str) -> Result<image::Preview, String>,
    ) -> Result<Self, String> {
        if bytes.len() > MAX_BYTES {
            return Err("History exceeds 8 MiB".into());
        }
        let mut root: serde_json::Map<String, serde_json::Value> =
            serde_json::from_slice(bytes).map_err(|_| "History contains malformed JSON")?;
        let mut data = Self::default();
        if let Some(serde_json::Value::Array(entries)) = root.remove("colors") {
            for value in entries {
                let Ok(mut entry) = serde_json::from_value::<Picked>(value) else {
                    continue;
                };
                let hex = entry
                    .hex
                    .trim()
                    .trim_start_matches('#')
                    .to_ascii_lowercase();
                if hex.len() != 6 || !hex.bytes().all(|ch| ch.is_ascii_hexdigit()) {
                    continue;
                }
                entry.hex = format!("#{hex}");
                // 当前色由hex定义；不让手改文件的不一致rgb改变点击复用语义。
                entry.rgb = Rgb {
                    r: u8::from_str_radix(&hex[..2], 16).unwrap(),
                    g: u8::from_str_radix(&hex[2..4], 16).unwrap(),
                    b: u8::from_str_radix(&hex[4..], 16).unwrap(),
                };
                if data.colors.iter().any(|old| old.hex == entry.hex) {
                    continue;
                }
                data.colors.push(entry);
                if data.colors.len() == LIMIT {
                    break;
                }
            }
        }
        if let Some(serde_json::Value::Array(entries)) = root.remove("screenshots") {
            for value in entries {
                let Ok(mut entry) = serde_json::from_value::<Screenshot>(value) else {
                    continue;
                };
                if entry.id.is_empty()
                    || entry.id.len() > 256
                    || entry.id.chars().any(char::is_control)
                    || entry.width == 0
                    || entry.height == 0
                    || data.screenshots.iter().any(|old| old.id == entry.id)
                {
                    continue;
                }
                // 旧版可保存大图fallback；解码器先缩放，绝不完整分配该大图。
                if !entry.thumbnail.starts_with("data:image/") {
                    continue;
                }
                // 资源/COM暂态失败不等于坏数据；保留原条目，后续取色不能永久删掉它。
                if let Ok(preview) = prepare(&entry.thumbnail) {
                    if let Ok(encoded) = image::encode(&preview.image) {
                        entry.thumbnail = encoded;
                    }
                    entry.preview = Some(preview);
                }
                data.screenshots.push(entry);
                if data.screenshots.len() == LIMIT {
                    break;
                }
            }
        }
        data.extra = root;
        Ok(data)
    }
    /// 启动读取期间的新提交优先，不能被迟到磁盘结果覆盖。
    pub fn merge_recent(&mut self, recent: Self) {
        for entry in recent.colors.into_iter().rev() {
            self.color(Color(entry.rgb.r, entry.rgb.g, entry.rgb.b), entry.at);
        }
        for entry in recent.screenshots.into_iter().rev() {
            self.screenshot(entry);
        }
    }
}

#[cfg(test)]
#[path = "capture_history_tests.rs"]
mod tests;

//! 宿主在来源身份校验后捕获的待应用操作；像素大小不能证明逻辑框或物化状态。
use crate::crop_geometry::{Rect, Size};

/// 由宿主验证过的自然像素恢复原文；普通保存的新canonical不能替换它。
#[derive(Clone)]
pub(in crate::desktop) struct Baseline {
    pub record: crate::session_source::Source,
    pub unit: crate::workspace_id::UnitId,
}

#[derive(Clone, Default)]
pub(in crate::desktop) struct Input {
    /// 完整 Editor 坐标框，不是当前裁剪可见框或自然像素尺寸。
    pub logical: Option<Size>,
    /// 自然来源原始逻辑sourceSize；非等比恢复后它不等于当前Editor完整框。
    pub source_logical: Option<Size>,
    /// 只解释自然来源的历史/布局，不提供目标当前crop、标注或外观。
    pub baseline: Option<Baseline>,
    /// 仅尚未物化的目标轴翻转；已翻转像素必须传 false。
    pub flip: (bool, bool),
    /// 自然来源尚未包含baseline的历史擦除；当前layers.base不得重放。
    pub historical_erase: bool,
    /// 自然来源尚未物化传播 placement；不影响独立标注的坐标。
    pub content: bool,
    /// 只有宿主证明仍有效的旧式像素 crop 才可重放；reset 后必须 false。
    pub legacy_crop: bool,
    /// 当前像素owner已物化flip后的有效区域；独立于content，优先于旧原文crop。
    pub legacy_pixels: Option<Rect>,
}

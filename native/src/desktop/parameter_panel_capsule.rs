//! 单字段capsule复用原行、真实控件和草稿；不挂完整面板的设置/执行/分页按钮。
use super::*;

pub(super) unsafe fn build(ui: &Ui) -> Result<()> {
    let model = ui.model.borrow().clone();
    let mut b = Builder {
        ui,
        bindings: Vec::new(),
        placements: Vec::new(),
        y: 4,
        next: 1000,
    };
    sticker::build(&mut b, &model)?;
    if let Some(snapshot) = &model.snapshot {
        for row in &model.rows {
            rows::build(&mut b, snapshot, row)?;
        }
    }
    let message = ui.message.borrow().clone();
    ui.status.set(b.label(&message)?);
    b.y += 26;
    ui.content_height.set(b.y);
    *ui.placements.borrow_mut() = b.placements;
    *ui.bindings.borrow_mut() = b.bindings;
    Ok(())
}

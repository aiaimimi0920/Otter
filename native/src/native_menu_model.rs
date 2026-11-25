//! 手写/异步菜单树与 HMENU 快照使用同等预算；替换在修改模型前验证整棵树。
use super::{invalid, Item};
use windows::core::Result;

pub(super) const MAX_GALLERY_ITEMS: usize = 4096;

/// 图库的逻辑数据与静态菜单树分开；每页仍只向 HWND 提交最多15张卡片。
pub(super) fn validate_gallery(items: &[Item]) -> Result<()> {
    if items.len() > MAX_GALLERY_ITEMS {
        return Err(invalid());
    }
    let mut units = 0usize;
    for item in items {
        let actions = item.children.as_deref().unwrap_or(&[]);
        if actions.len() > 2 || actions.iter().any(|action| action.children.is_some()) {
            return Err(invalid());
        }
        for row in std::iter::once(item).chain(actions) {
            let count = row.label.encode_utf16().take(4097).count();
            units += count;
            if row.label.contains('\0') || count > 4096 || units > 2 * 1024 * 1024 {
                return Err(invalid());
            }
        }
    }
    Ok(())
}

pub(super) fn validate(tree: &[Item], replacement: Option<(usize, &[Item])>) -> Result<()> {
    let mut budget = (0usize, 0usize);
    for (index, item) in tree.iter().enumerate() {
        let children = match replacement {
            Some((target, children)) if index == target => Some(children),
            _ => item.children.as_deref(),
        };
        visit(item, children, 0, &mut budget)?;
    }
    Ok(())
}

fn visit(
    item: &Item,
    children: Option<&[Item]>,
    depth: usize,
    budget: &mut (usize, usize),
) -> Result<()> {
    if depth >= 16 || item.label.contains('\0') {
        return Err(invalid());
    }
    let units = item.label.encode_utf16().take(4097).count();
    budget.0 += 1;
    budget.1 += units;
    if units > 4096 || budget.0 > 1024 || budget.1 > 128 * 1024 {
        return Err(invalid());
    }
    if let Some(children) = children {
        if depth + 1 >= 16 {
            return Err(invalid());
        }
        for child in children {
            visit(child, child.children.as_deref(), depth + 1, budget)?;
        }
    }
    Ok(())
}

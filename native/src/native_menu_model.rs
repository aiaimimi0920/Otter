//! 手写/异步菜单树与 HMENU 快照使用同等预算；替换在修改模型前验证整棵树。
use super::{invalid, Item};
use windows::core::Result;

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

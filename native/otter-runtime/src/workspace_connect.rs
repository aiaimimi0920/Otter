//! 用户连线事务：先验证和预留，再替换单输入边；导入仍可用 add_link 保留旧图语义。
use super::*;

impl Workspace {
    /// 对齐主干 useLinking 的自连接/循环拒绝和单输入规则，不承担端口能力类型验证。
    /// 与先删除旧边再添加不同，此处任何失败都保持原图完整。
    pub fn connect(&mut self, link: Link) -> Result<Option<Link>, Error> {
        if !self.contains(&link.from_unit) || !self.contains(&link.to_unit) {
            return Err(Error::MissingUnit);
        }
        if !valid_port(&link.from_port) || !valid_port(&link.to_port) {
            return Err(Error::InvalidPort);
        }
        if link.from_unit == link.to_unit {
            return Err(Error::SelfConnection);
        }
        if self.links.iter().any(|existing| existing.id == link.id) {
            return Err(Error::DuplicateId);
        }
        // 主干先对现有图检查 target -> source，再替换输入。已有有向环也必须终止遍历。
        if self.reachable(&link.to_unit, &link.from_unit) {
            return Err(Error::Cycle);
        }
        let replaced = self.links.iter().position(|existing| {
            existing.to_unit == link.to_unit && existing.to_port == link.to_port
        });
        if replaced.is_none() && self.links.len() >= MAX_LINKS {
            return Err(Error::Capacity);
        }
        if replaced.is_none() {
            self.links.try_reserve(1).map_err(|_| Error::Allocation)?;
        }
        // 主干移除第一个占用边后追加新边；保留已有导入图中其余边的顺序和内容。
        self.topology = Arc::new(());
        let previous = replaced.map(|index| self.links.remove(index));
        self.links.push(link);
        Ok(previous)
    }

    fn reachable(&self, from: &UnitId, to: &UnitId) -> bool {
        let Some(start) = self.units.iter().position(|unit| unit == from) else {
            return false;
        };
        // 对象集合最多 MAX_UNITS；固定栈缓冲，不为每次鼠标落点分配 BFS 容器。
        let mut queue = [0usize; MAX_UNITS];
        let mut visited = [false; MAX_UNITS];
        let (mut head, mut tail) = (0, 1);
        queue[0] = start;
        visited[start] = true;
        while head < tail {
            let unit = &self.units[queue[head]];
            head += 1;
            if unit == to {
                return true;
            }
            for link in self.links.iter().filter(|link| link.from_unit == *unit) {
                let Some(next) = self.units.iter().position(|unit| *unit == link.to_unit) else {
                    continue;
                };
                if !visited[next] {
                    visited[next] = true;
                    queue[tail] = next;
                    tail += 1;
                }
            }
        }
        false
    }
}

#[cfg(test)]
#[path = "workspace_connect_tests.rs"]
mod tests;

//! 执行输入解析对齐 executionImageInputs；主输入缺失不能用 Art 预览冒充就绪。
use super::*;
type Images<'a> = Vec<(&'a str, &'a str)>;
fn primary(name: &str) -> bool {
    ["input", "input_image", "image"]
        .iter()
        .any(|alias| name.eq_ignore_ascii_case(alias))
}
fn manual_value<'a>(manual: &[(&str, Output<'a>)], name: &str) -> Option<&'a str> {
    match manual.iter().find(|(key, _)| *key == name)?.1 {
        Output::Image(value) if !value.is_empty() => Some(value),
        _ => None,
    }
}
fn validate_manual(manual: &[(&str, Output<'_>)]) -> Result<(), Error> {
    if manual.len() > MAX_PORTS || manual.iter().any(|(name, _)| name.len() > 256) {
        Err(Error::Capacity)
    } else {
        Ok(())
    }
}
fn insert<'a>(images: &mut Images<'a>, name: &'a str, value: &'a str) -> Result<(), Error> {
    if let Some(entry) = images.iter_mut().find(|(key, _)| *key == name) {
        entry.1 = value;
    } else {
        images.try_reserve(1).map_err(|_| Error::Allocation)?;
        images.push((name, value));
    }
    Ok(())
}
impl<'a> Graph<'a> {
    /// 有连接但输出未就绪时返回 None；仅无连接的钉图允许回退到自己的图像解析。
    pub fn execution_image(&self, id: &UnitId) -> Option<&'a str> {
        let index = self.index(id)?;
        match self.input(index) {
            Some(link) => self.connected(link, &mut [false; MAX_UNITS]),
            None if self.nodes[index].kind == Kind::Sticker => self.image(id),
            _ => None,
        }
    }
    /// 所有声明图片输入：连接优先，其次同名手动参数。借用值，不复制载荷。
    pub fn execution_images(
        &self,
        id: &UnitId,
        manual: &[(&str, Output<'a>)],
    ) -> Result<Images<'a>, Error> {
        validate_manual(manual)?;
        let Some(index) = self.index(id) else {
            return Ok(Vec::new());
        };
        let node = &self.nodes[index];
        let ports = if node.kind == Kind::Art {
            node.capability_inputs.unwrap_or(node.inputs)
        } else {
            node.inputs
        };
        let mut images = Vec::new();
        let mut declared = false;
        for port in ports
            .iter()
            .filter(|port| !port.name.is_empty() && image_like(**port))
        {
            declared = true;
            if let Some(value) = self
                .connected_image(id, port.name)
                .or_else(|| manual_value(manual, port.name))
            {
                insert(&mut images, port.name, value)?;
            }
        }
        if !declared && node.kind != Kind::Art {
            if let Some(value) = self
                .connected_image(id, "image")
                .or_else(|| manual_value(manual, "image"))
            {
                insert(&mut images, "image", value)?;
            }
        }
        Ok(images)
    }
    /// 主干辅助图片执行只使用 Art 能力声明，不把任意对象字段自动升级为执行端口。
    pub fn auxiliary_execution_images(
        &self,
        id: &UnitId,
        manual: &[(&str, Output<'a>)],
    ) -> Result<Images<'a>, Error> {
        validate_manual(manual)?;
        let Some(index) = self.index(id) else {
            return Ok(Vec::new());
        };
        let node = &self.nodes[index];
        if node.kind != Kind::Art {
            return Ok(Vec::new());
        }
        let mut images = Vec::new();
        for port in node
            .capability_inputs
            .unwrap_or(&[])
            .iter()
            .filter(|port| !port.name.is_empty() && image_like(**port) && !primary(port.name))
        {
            if let Some(value) = self
                .connected_image(id, port.name)
                .or_else(|| manual_value(manual, port.name))
            {
                insert(&mut images, port.name, value)?;
            }
        }
        Ok(images)
    }
    pub fn missing_execution_images(
        &self,
        id: &UnitId,
        manual: &[(&str, Output<'a>)],
    ) -> Result<Vec<&'a str>, Error> {
        validate_manual(manual)?;
        let Some(index) = self.index(id) else {
            return Ok(Vec::new());
        };
        let node = &self.nodes[index];
        let ports = if node.kind == Kind::Art {
            node.capability_inputs.unwrap_or(node.inputs)
        } else {
            node.inputs
        };
        let mut declared = Vec::new();
        for port in ports
            .iter()
            .filter(|port| !port.name.is_empty() && image_like(**port))
        {
            if !declared.contains(&port.name) {
                declared.try_reserve(1).map_err(|_| Error::Allocation)?;
                declared.push(port.name);
            }
        }
        let Some(first) = declared
            .iter()
            .find(|name| primary(name))
            .or(declared.first())
            .copied()
        else {
            return Ok(Vec::new());
        };
        let value = if primary(first) {
            self.execution_image(id)
        } else {
            self.connected_image(id, first)
        };
        let auxiliary = self.auxiliary_execution_images(id, manual)?;
        let mut missing = Vec::new();
        missing
            .try_reserve(declared.len())
            .map_err(|_| Error::Allocation)?;
        if value.is_none() {
            missing.push(first);
        }
        for name in declared.into_iter().filter(|name| *name != first) {
            if !auxiliary.iter().any(|(key, _)| *key == name) {
                missing.push(name);
            }
        }
        Ok(missing)
    }
}
#[cfg(test)]
#[path = "graph_execution_images_tests.rs"]
mod tests;

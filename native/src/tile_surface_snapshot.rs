//! Tighten existing Scene contracts and index resources; validate all references before downloading/delivering complete frames.
use super::*;
use crate::surface_scene::{Kind, Node, Scene};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
pub(crate) struct Snapshot {
    pub revision: u64,
    pub digest: [u8; 32],
    pub scene: Vec<u8>,
    pub nodes: usize,
    pub images: BTreeMap<String, String>,
    pub dimensions: BTreeMap<String, [Option<u32>; 2]>,
}
pub(super) fn parse(v: &Value, view: &View) -> Result<Snapshot> {
    require(
        v["protocolVersion"] == "loom.surface.v1"
            && v["instanceId"] == view.instance_id
            && v["attachmentId"] == view.attachment_id
            && v["runtime"] == "declarative",
    )?;
    id(&v["artId"])?;
    id(&v["artVersion"])?;
    if !v["viewId"].is_null() {
        id(&v["viewId"])?;
    }
    require(
        v.get("resourceLeases")
            .is_none_or(|a| a.as_array().is_some_and(Vec::is_empty)),
    )?;
    let revision = integer(&v["revision"])?;
    let resources = v["resources"].as_array().ok_or("tile_surface_resources")?;
    require(resources.len() <= 64)?;
    let mut seen = BTreeSet::new();
    let mut descriptors = BTreeMap::new();
    for r in resources {
        let resource = id(&r["resourceId"])?;
        require(wall::image_id(resource) && seen.insert(resource))?;
        require(
            r["mime"].as_str().is_some_and(|s| s.len() <= 128)
                && ["image", "audio", "video", "file", "binary"]
                    .contains(&r["kind"].as_str().unwrap_or("")),
        )?;
        integer(&r["size"])?;
        let dimension = |key| -> Result<Option<u32>> {
            if r[key].is_null() {
                return Ok(None);
            }
            let n = integer(&r[key])?;
            require((1..=16384).contains(&n))?;
            Ok(Some(n as u32))
        };
        let dims = [dimension("width")?, dimension("height")?];
        if r["kind"] == "image" {
            descriptors.insert(resource.to_owned(), dims);
        }
    }
    let scene = serde_json::to_vec(&v["scene"]).map_err(|_| "tile_surface_scene")?;
    let model = Scene::parse(&scene).map_err(|_| "tile_surface_scene")?;
    let mut images = BTreeMap::new();
    fn visit(
        n: &Node,
        depth: usize,
        count: &mut usize,
        images: &mut BTreeMap<String, String>,
        descriptors: &BTreeMap<String, [Option<u32>; 2]>,
    ) -> Result<()> {
        *count += 1;
        require(*count <= 512 && depth <= 16)?;
        if n.kind() == Kind::Image {
            // URLs/inline data must not bypass view authorization. Sourceless images may retain empty placeholders but cannot falsely claim pixels.
            require(
                n.props()
                    .get("src")
                    .is_none_or(|v| v.is_null() || v.as_str() == Some("")),
            )?;
            if let Some(resource) = n.props().get("resourceId") {
                let resource = id(resource)?;
                require(descriptors.contains_key(resource))?;
                images.insert(n.id().to_owned(), resource.to_owned());
            }
        }
        for child in n.children() {
            visit(child, depth + 1, count, images, descriptors)?;
        }
        Ok(())
    }
    let mut nodes = 0;
    visit(model.root(), 0, &mut nodes, &mut images, &descriptors)?;
    let wanted: BTreeSet<_> = images.values().cloned().collect();
    descriptors.retain(|id, _| wanted.contains(id));
    require(descriptors.len() <= 16)?;
    let raw = serde_json::to_vec(v).map_err(|_| "tile_surface_snapshot")?;
    Ok(Snapshot {
        revision,
        digest: Sha256::digest(raw).into(),
        scene,
        nodes,
        images,
        dimensions: descriptors,
    })
}

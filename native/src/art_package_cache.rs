//! 资源以原1MiB暂存上限分块，和源码共享32项/32MiB；不另建预算、不驱逐旧内容。
use crate::{
    art_execution_grant::package::Resource,
    art_source_cache::{Store, MAX_SOURCE},
};
use sha2::{Digest, Sha256};
use std::io;
fn key(source_key: &[u8; 32], resource: &Resource, chunk: usize) -> [u8; 32] {
    Sha256::digest(
        serde_json::to_vec(&("art-package-resource.v1", source_key, resource, chunk))
            .expect("bounded resource key"),
    )
    .into()
}
pub(crate) fn load(
    store: &Store,
    source_key: &[u8; 32],
    resource: &Resource,
) -> io::Result<Option<Vec<u8>>> {
    if resource.size == 0 || resource.size > 4 * 1024 * 1024 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let size = resource.size as usize;
    let mut bytes = Vec::with_capacity(size);
    for chunk in 0..size.div_ceil(MAX_SOURCE) {
        let Some(part) = store.load_blob(&key(source_key, resource, chunk))? else {
            return Ok(None);
        };
        if part.len() != (size - bytes.len()).min(MAX_SOURCE) {
            return Err(io::ErrorKind::InvalidData.into());
        }
        bytes.extend_from_slice(&part);
    }
    if format!("{:x}", Sha256::digest(&bytes)) != resource.digest {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(Some(bytes))
}
pub(crate) fn put(
    store: &Store,
    source_key: &[u8; 32],
    resource: &Resource,
    bytes: &[u8],
    stopped: &impl Fn() -> bool,
) -> io::Result<()> {
    if bytes.len() as u64 != resource.size
        || format!("{:x}", Sha256::digest(bytes)) != resource.digest
    {
        return Err(io::ErrorKind::InvalidData.into());
    }
    for (chunk, part) in bytes.chunks(MAX_SOURCE).enumerate() {
        store.put_blob(&key(source_key, resource, chunk), part, stopped)?;
    }
    Ok(())
}
#[cfg(test)]
#[path = "art_package_cache_tests.rs"]
mod tests;

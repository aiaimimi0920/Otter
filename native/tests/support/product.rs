//! 集成测试可固定待验收的实际包；显式路径必须绑定摘要，并固定文件到子进程创建后。
use sha2::{Digest, Sha256};
use std::{
    ffi::OsString,
    fs::File,
    io::{self, Read},
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::{Component, Path, PathBuf, Prefix},
};

pub struct Product {
    path: PathBuf,
    _file: File,
}
impl Product {
    pub fn resolve(default: &Path) -> io::Result<Self> {
        Self::from_options(
            default,
            std::env::var_os("OTTER_NATIVE_TEST_PRODUCT_EXE"),
            std::env::var_os("OTTER_NATIVE_TEST_PRODUCT_SHA256"),
        )
    }
    fn from_options(
        default: &Path,
        explicit: Option<OsString>,
        digest: Option<OsString>,
    ) -> io::Result<Self> {
        let invalid = || io::Error::new(io::ErrorKind::InvalidInput, "Invalid pinned test product");
        if explicit.is_some() != digest.is_some() {
            return Err(invalid());
        }
        let expected = digest
            .map(|value| {
                value
                    .into_string()
                    .ok()
                    .filter(|s| s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit()))
                    .ok_or_else(invalid)
            })
            .transpose()?;
        let path = explicit.map_or_else(|| default.to_owned(), PathBuf::from);
        if !path.is_absolute()
            || !matches!(path.components().next(), Some(Component::Prefix(prefix))
                if matches!(prefix.kind(), Prefix::Disk(_)))
            || path.components().any(|c| matches!(c, Component::ParentDir))
        {
            return Err(invalid());
        }
        let mut current = PathBuf::new();
        for component in path.components() {
            current.push(component);
            if matches!(component, Component::Prefix(_)) {
                continue;
            }
            if std::fs::symlink_metadata(&current)?.file_attributes() & 0x400 != 0 {
                return Err(invalid());
            }
        }
        // 只共享读取；hash 完成后仍持有文件，禁止启动前写入/替换候选。
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&path)?;
        if !file.metadata()?.is_file() || file.metadata()?.len() > 128 * 1024 * 1024 {
            return Err(invalid());
        }
        if let Some(expected) = expected {
            let mut hash = Sha256::new();
            let mut buffer = [0; 64 * 1024];
            loop {
                let count = file.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                hash.update(&buffer[..count]);
            }
            if !format!("{:x}", hash.finalize()).eq_ignore_ascii_case(&expected) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Test product SHA256 mismatch",
                ));
            }
        }
        Ok(Self { path, _file: file })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> PathBuf {
        let root = PathBuf::from(
            "C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-pinned-product-tests",
        );
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join(format!("{:?}.bin", unsafe {
            windows::Win32::System::Com::CoCreateGuid().unwrap()
        }));
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        std::fs::write(&path, b"pinned product fixture").unwrap();
        path
    }
    #[test]
    fn explicit_product_requires_valid_pair_before_opening_any_path() {
        let missing = Path::new("C:/not-a-test-product.exe");
        for (path, hash) in [
            (Some(missing.as_os_str().to_owned()), None),
            (None, Some("0".repeat(64).into())),
            (Some("relative.exe".into()), Some("0".repeat(64).into())),
            (Some(missing.as_os_str().to_owned()), Some("invalid".into())),
        ] {
            assert_eq!(
                Product::from_options(missing, path, hash)
                    .err()
                    .unwrap()
                    .kind(),
                io::ErrorKind::InvalidInput
            );
        }
    }
    #[test]
    fn explicit_product_matches_digest_and_remains_locked_until_spawn_owner_releases() {
        let path = fixture();
        let expected = format!("{:X}", Sha256::digest(b"pinned product fixture"));
        let product = Product::from_options(
            &path,
            Some(path.clone().into_os_string()),
            Some(expected.into()),
        )
        .unwrap();
        assert_eq!(product.path(), path);
        assert!(std::fs::OpenOptions::new().write(true).open(&path).is_err());
        drop(product);
        assert!(std::fs::OpenOptions::new().write(true).open(&path).is_ok());
    }
    #[test]
    fn explicit_product_rejects_mismatch_without_changing_file() {
        let path = fixture();
        let actual = std::fs::read(&path).unwrap();
        assert_eq!(
            Product::from_options(
                &path,
                Some(path.clone().into_os_string()),
                Some("0".repeat(64).into())
            )
            .err()
            .unwrap()
            .kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(std::fs::read(path).unwrap(), actual);
    }
}

//! Receiving side of a checkpoint handoff: claim the existing exclusive lease
//! first. Never create a profile or substitute a recovery file for a receipt.
use super::{invalid, Profile};
use sha2::{Digest, Sha256};
use std::{fs::File, io, path::Path, sync::atomic::AtomicBool};

#[derive(Clone, Copy, PartialEq)]
pub struct Expected {
    revision: u64,
    digest: [u8; 32],
}
impl Expected {
    pub fn parse(revision: &str, sha256: &str) -> io::Result<Self> {
        if revision.len() > 20
            || revision.starts_with('0')
            || !revision.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(invalid("Expected a positive decimal checkpoint revision"));
        }
        let revision = revision
            .parse()
            .map_err(|_| invalid("Invalid checkpoint revision"))?;
        if sha256.len() != 64
            || !sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid("Expected a lowercase SHA-256 checkpoint digest"));
        }
        let mut digest = [0; 32];
        for (i, byte) in digest.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&sha256[2 * i..2 * i + 2], 16).expect("validated hex pair");
        }
        Ok(Self { revision, digest })
    }
}

pub(in crate::desktop) struct Checkpoint {
    pub(in crate::desktop) revision: u64,
    pub(in crate::desktop) bytes: Vec<u8>,
    // Preserve the file's sharing restrictions through restoration and revision
    // readback; retaining bytes alone would not pin its on-disk identity.
    _baseline: File,
}
impl Profile {
    pub(in crate::desktop) fn claim(root: &Path, expected: Expected) -> io::Result<Self> {
        // Only fixed local paths. This pins names, not publisher/user identity.
        let directories = crate::install_file_leases::shared_directories(root)?;
        let lease = Self::lock(root, false)?;
        let path = root.join("workspace.json");
        let baseline = crate::install_file_leases::binary(&path)?;
        let length = usize::try_from(baseline.metadata()?.len())
            .ok()
            .filter(|n| (1..=crate::session_document::MAX_SESSION_BYTES).contains(n))
            .ok_or_else(|| invalid("Restart checkpoint exceeds session budget or is empty"))?;
        let bytes = crate::desktop::session_job::file::read_file(
            &baseline,
            length,
            &AtomicBool::new(false),
        )?;
        if Sha256::digest(&bytes)[..] != expected.digest {
            return Err(invalid(
                "Restart checkpoint differs from the published digest",
            ));
        }
        Ok(Self {
            path,
            restore: true,
            claimed: Some(Checkpoint {
                revision: expected.revision,
                bytes,
                _baseline: baseline,
            }),
            _lease: lease,
            _directories: directories,
        })
    }
}

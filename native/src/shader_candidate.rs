//! Explicit internal candidate admission; code pins are not license/release approval, and digests from CLI, environment, or Art are rejected.
use crate::{
    shader_angle::{Fingerprint, Identity},
    shader_process::TrustedBundle,
};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};
#[path = "shader_candidate_acl.rs"]
mod acl;
use otter_media::install_file_leases as files;

// Reviewed local build: v332 build.json ae24c4b8..., v333 materials.json 95322473....
// Updates must re-review the build/materials and developer-side execution together; never import new pins automatically from the candidate directory.
fn identity() -> Identity {
    fn fingerprint(bytes: u64, hex: &str) -> Fingerprint {
        let mut sha256 = [0; 32];
        for (index, byte) in sha256.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).expect("static pin");
        }
        Fingerprint { bytes, sha256 }
    }
    Identity {
        egl: fingerprint(
            199680,
            "fb538bb38dae68c612cfc309881143cdbd3ace9498e207f4cb2f6d38c3721639",
        ),
        gles: fingerprint(
            5295616,
            "beeb45725d5ef558eaaa1e3a222df31e9d53103c0ee154f6853380e2a362e5ff",
        ),
    }
}
fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "Shader candidate path, ACL, file set or pinned identity rejected",
    )
}
fn verify(file: &mut File, pin: Fingerprint) -> io::Result<()> {
    if file.metadata()?.len() != pin.bytes {
        return Err(invalid());
    }
    let mut hash = Sha256::new();
    let mut buffer = [0; 16384];
    let mut remaining = pin.bytes;
    while remaining != 0 {
        let count = file.read(&mut buffer[..(remaining as usize).min(16384)])?;
        if count == 0 {
            return Err(invalid());
        }
        hash.update(&buffer[..count]);
        remaining -= count as u64;
    }
    if hash.finalize().as_slice() != pin.sha256 {
        return Err(invalid());
    }
    Ok(())
}
/// Does not install, copy, or change permissions; the caller explicitly selects a prepared two-file directory writable only by trusted users.
/// Protects against other local users; malicious same-user processes, administrators, and the system are not considered isolatable attackers.
pub fn open(directory: &Path) -> io::Result<TrustedBundle> {
    let mut leases = files::directories(directory, |file, path| {
        // The developer-only shader sandbox trusts the volume root and the
        // interactive user. Elevation admission must not use this policy.
        if path.parent().is_none() {
            Ok(())
        } else {
            acl::ancestor(file)
        }
    })?;
    acl::check(leases.last().ok_or_else(invalid)?)?;
    let entries: Vec<_> = std::fs::read_dir(directory)?
        .take(3)
        .collect::<io::Result<_>>()?;
    if entries.len() != 2
        || entries.iter().any(|entry| {
            !matches!(
                entry.file_name().to_str(),
                Some("libEGL.dll" | "libGLESv2.dll")
            )
        })
    {
        return Err(invalid());
    }
    let identity = identity();
    for (name, pin) in [
        ("libEGL.dll", identity.egl),
        ("libGLESv2.dll", identity.gles),
    ] {
        let mut file = files::binary(&directory.join(name))?;
        acl::check(&file)?;
        verify(&mut file, pin)?;
        leases.push(file);
    }
    Ok(TrustedBundle::new(directory, identity)?.with_files(leases))
}

// The fixed candidate reacquires leases in the child; abnormal parent exit must not rely on a cross-process Arc remaining alive.
// Other existing trusted-manifest callers retain the original wire contract; this compatibility branch is not a candidate authorization entry.
#[cfg(test)]
pub(crate) fn from_worker(directory: &Path, supplied: Identity) -> io::Result<TrustedBundle> {
    let pinned = identity();
    if [(supplied.egl, pinned.egl), (supplied.gles, pinned.gles)]
        .iter()
        .all(|(a, b)| a.bytes == b.bytes && a.sha256 == b.sha256)
    {
        let plain = directory.to_str().and_then(|s| s.strip_prefix(r"\\?\"));
        let directory = plain.map(Path::new).unwrap_or(directory);
        open(directory)
    } else {
        TrustedBundle::new(directory, supplied)
    }
}

#[cfg(test)]
#[path = "shader_candidate_tests.rs"]
mod tests;

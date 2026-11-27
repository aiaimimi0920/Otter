//! Session-local directory capability. A capture retains the very directory
//! confirmed by the user, not a path selected again when the encoder finishes.
use std::{
    fs::{File, OpenOptions},
    io,
    os::windows::{
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::{Path, PathBuf},
    sync::Arc,
};
use windows::Win32::{Foundation::HANDLE, Storage::FileSystem::*};

pub(super) type DirectoryId = (u64, [u8; 16]);

pub(in crate::desktop) struct Target {
    pub(super) path: PathBuf,
    pub(super) id: DirectoryId,
    _identity: File,
}

fn directory_id(leaf: &File) -> io::Result<DirectoryId> {
    let mut info = FILE_ID_INFO::default();
    unsafe {
        GetFileInformationByHandleEx(
            HANDLE(leaf.as_raw_handle()),
            FileIdInfo,
            (&mut info as *mut FILE_ID_INFO).cast(),
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        )
    }?;
    if leaf.metadata()?.file_attributes()
        & (FILE_ATTRIBUTE_REPARSE_POINT.0 | FILE_ATTRIBUTE_DIRECTORY.0)
        != FILE_ATTRIBUTE_DIRECTORY.0
        || info.FileId.Identifier == [0; 16]
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Cannot identify the archive directory",
        ));
    }
    Ok((info.VolumeSerialNumber, info.FileId.Identifier))
}

fn finish(path: &Path, pins: Vec<File>) -> io::Result<Arc<Target>> {
    let leaf = pins.last().expect("directory leases include the leaf");
    let id = directory_id(leaf)?;
    // Keep identity, not a session-long deny-write on the user's profile,
    // volume root or selected folder. Holding the identity handle also keeps
    // its file index from being recycled while requests are queued.
    let identity = OpenOptions::new()
        .read(true)
        .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE).0)
        .custom_flags((FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT).0)
        .open(path)?;
    if directory_id(&identity)? != id {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Archive selection changed",
        ));
    }
    Ok(Arc::new(Target {
        path: path.to_owned(),
        id,
        _identity: identity,
    }))
}

impl Target {
    pub(super) fn begin_file(&self) -> io::Result<super::archive_file::Staged> {
        // Recheck the same object without following a newly installed junction.
        // Deny concurrent reparse mutation only for the relative staging open;
        // the remaining operations use the exclusive staged file handle.
        let pins = crate::install_file_leases::directories(&self.path, |_, _| Ok(()))?;
        let leaf = pins.last().expect("directory leases include the leaf");
        if directory_id(leaf)? != self.id {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Archive directory no longer matches the selection",
            ));
        }
        super::archive_file::Staged::new(leaf)
    }
}

/// A shell picker only grants an *existing* fixed-local directory. Never
/// create a missing selection, follow a junction, or contact a UNC target.
pub(in crate::desktop) fn grant(path: &Path) -> Result<Arc<Target>, String> {
    crate::install_file_leases::directories(path, |_, _| Ok(()))
        .and_then(|pins| finish(path, pins))
        .map_err(|e| e.to_string())
}

/// Only after affirmative consent may the fixed `saved` child be created.
/// The leased profile must already exist; never create or rediscover it here.
pub(super) fn grant_default(profile: &Path) -> Result<Arc<Target>, String> {
    let mut pins =
        crate::install_file_leases::shared_directories(profile).map_err(|e| e.to_string())?;
    let path = profile.join("saved");
    match std::fs::create_dir(&path) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e.to_string()),
    }
    pins.push(crate::install_file_leases::directory(&path).map_err(|e| e.to_string())?);
    finish(&path, pins).map_err(|e| e.to_string())
}

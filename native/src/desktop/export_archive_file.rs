//! Stage under the consented directory object, then rename the same open file
//! by basename. No part of publication or cleanup resolves a DOS path again.
use std::{
    fs::File,
    io::{self, Write},
    mem::{offset_of, size_of},
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle},
    },
    path::Path,
    sync::atomic::AtomicBool,
};
use windows::{
    core::PWSTR,
    Wdk::{Foundation::OBJECT_ATTRIBUTES, Storage::FileSystem::*},
    Win32::{
        Foundation::*,
        Storage::FileSystem::*,
        System::{Com::CoCreateGuid, IO::IO_STATUS_BLOCK},
    },
};

fn status(s: NTSTATUS) -> io::Result<()> {
    if s.0 >= 0 {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(
            unsafe { RtlNtStatusToDosError(s) } as i32,
        ))
    }
}

pub(super) struct Staged {
    file: File,
    published: bool,
}
impl Staged {
    pub(super) fn new(directory: &File) -> io::Result<Self> {
        let id = unsafe { CoCreateGuid() }?;
        let mut name: Vec<_> = format!(".otter-archive-{id:?}.tmp")
            .encode_utf16()
            .collect();
        let length = u16::try_from(name.len() * 2).expect("GUID filename fits UNICODE_STRING");
        let name = UNICODE_STRING {
            Buffer: PWSTR(name.as_mut_ptr()),
            Length: length,
            MaximumLength: length,
        };
        let attributes = OBJECT_ATTRIBUTES {
            Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
            RootDirectory: HANDLE(directory.as_raw_handle()),
            ObjectName: &name,
            Attributes: OBJ_CASE_INSENSITIVE | OBJ_DONT_REPARSE,
            ..Default::default()
        };
        let mut handle = HANDLE::default();
        let mut ios = IO_STATUS_BLOCK::default();
        status(unsafe {
            NtCreateFile(
                &mut handle,
                FILE_GENERIC_WRITE | DELETE,
                &attributes,
                &mut ios,
                None,
                FILE_ATTRIBUTE_NORMAL,
                FILE_SHARE_MODE(0),
                FILE_CREATE,
                FILE_NON_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
                None,
                0,
            )
        })?;
        Ok(Self {
            file: unsafe { File::from_raw_handle(handle.0) },
            published: false,
        })
    }

    pub(super) fn publish(
        mut self,
        bytes: &[u8],
        stem: &str,
        cancel: &AtomicBool,
    ) -> io::Result<()> {
        self.file.write_all(bytes)?;
        self.file.sync_all()?;
        crate::file_naming::allocate::allocate_with(
            Path::new(""),
            stem,
            Some("png"),
            cancel,
            |name| self.rename(name),
        )?;
        self.published = true;
        Ok(())
    }

    fn rename(&self, name: &Path) -> io::Result<()> {
        if name.file_name() != Some(name.as_os_str()) || name.as_os_str().is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Expected an archive basename",
            ));
        }
        let wide: Vec<_> = name.as_os_str().encode_wide().collect();
        let offset = offset_of!(FILE_RENAME_INFORMATION, FileName);
        let size = (offset + 2 * wide.len()).max(size_of::<FILE_RENAME_INFORMATION>());
        let mut buf = vec![0usize; size.div_ceil(size_of::<usize>())];
        let info = buf.as_mut_ptr().cast::<FILE_RENAME_INFORMATION>();
        // Null RootDirectory + a single basename retains the source's parent;
        // a zero ReplaceIfExists reserves the first available name atomically.
        unsafe {
            (*info).FileNameLength = (2 * wide.len()) as u32;
            let tail = buf.as_mut_ptr().cast::<u8>().add(offset).cast::<u16>();
            std::ptr::copy_nonoverlapping(wide.as_ptr(), tail, wide.len());
            status(NtSetInformationFile(
                HANDLE(self.file.as_raw_handle()),
                &mut IO_STATUS_BLOCK::default(),
                info.cast(),
                size as u32,
                FileRenameInformation,
            ))
        }
    }
}
impl Drop for Staged {
    fn drop(&mut self) {
        if !self.published {
            // Never remove by name: another process might have reused it.
            let _ = otter_media::session_file_rename::retire(&self.file);
        }
    }
}

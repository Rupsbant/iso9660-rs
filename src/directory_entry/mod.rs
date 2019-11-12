// SPDX-License-Identifier: (MIT OR Apache-2.0)

pub use self::isodirectory::{ISODirectory, ISODirectoryIterator};
pub use self::isofile::{ISOFile, ISOFileReader};

use crate::parse::{DirectoryEntryHeader, FileFlags};
use crate::{FileRef, ISO9660Reader, Result, Recovered};

mod isodirectory;
mod isofile;

#[derive(Clone, Debug)]
pub enum DirectoryEntry<T: ISO9660Reader> {
    Directory(ISODirectory<T>),
    File(ISOFile<T>),
}

impl<T: ISO9660Reader> DirectoryEntry<T> {
    pub(crate) fn new(
        header: DirectoryEntryHeader,
        identifier: String,
        file: FileRef<T>,
    ) -> Recovered<Self> {
        if header.file_flags.contains(FileFlags::DIRECTORY) {
            let dir = ISODirectory::new(header, identifier, file);
            Recovered::ok(DirectoryEntry::Directory(dir))
        } else {
            ISOFile::new(header, identifier, file).map(DirectoryEntry::File)
        }
    }

    pub fn header(&self) -> &DirectoryEntryHeader {
        match *self {
            DirectoryEntry::Directory(ref dir) => &dir.header,
            DirectoryEntry::File(ref file) => &file.header,
        }
    }

    pub fn identifier(&self) -> &str {
        match *self {
            DirectoryEntry::Directory(ref dir) => &dir.identifier,
            DirectoryEntry::File(ref file) => &file.identifier,
        }
    }
}

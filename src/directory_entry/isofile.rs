// SPDX-License-Identifier: (MIT OR Apache-2.0)

use std::cmp::min;
use std::fmt;
use std::io::{self, Initializer, Read, Seek, SeekFrom, Write};
use std::mem;
use std::str::FromStr;

use time::Tm;

use super::DirectoryEntryHeader;
use crate::{FileRef, ISO9660Reader, ISOError, Result, Recovered};

#[derive(Clone)]
pub struct ISOFile<T: ISO9660Reader> {
    pub(crate) header: DirectoryEntryHeader,
    pub identifier: String,
    // File version; ranges from 1 to 32767
    pub version: u16,
    file: FileRef<T>,
}

impl<T: ISO9660Reader> fmt::Debug for ISOFile<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("ISOFile")
            .field("header", &self.header)
            .field("identifier", &self.identifier)
            .field("version", &self.version)
            .finish()
    }
}

impl<T: ISO9660Reader> ISOFile<T> {
    pub(crate) fn new(
        header: DirectoryEntryHeader,
        mut identifier: String,
        file: FileRef<T>,
    ) -> Recovered<ISOFile<T>> {
        // Files (not directories) in ISO 9660 have a version number, which is
        // provided at the end of the identifier, seperated by ';'
        let (version, error) = match identifier.rfind(';') {
            None => (0, Err(ISOError::InvalidFs("File indentifier missing ';'"))),
            Some(idx) => {
                let version = u16::from_str(&identifier[idx + 1..]);
                identifier.truncate(idx);
                match version {
                    Ok(version) => (version, Ok(())),
                    Err(err) => (0, Err(err.into())),
                }
            }
        };

        // Files without an extension have a '.' at the end
        if identifier.ends_with('.') {
            identifier.pop();
        }

        Recovered(ISOFile {
            header,
            identifier,
            version,
            file,
        }, error)
    }

    pub fn size(&self) -> u32 {
        self.header.extent_length
    }

    pub fn time(&self) -> Tm {
        self.header.time
    }

    pub fn read(&self) -> ISOFileReader<T> {
        ISOFileReader {
            buf: unsafe { mem::uninitialized() },
            buf_lba: None,
            seek: 0,
            start_lba: self.header.extent_loc,
            size: self.size() as usize,
            file: self.file.clone(),
        }
    }
}

pub struct ISOFileReader<T: ISO9660Reader> {
    buf: [u8; 2048],
    buf_lba: Option<u64>,
    seek: usize,
    start_lba: u32,
    size: usize,
    file: FileRef<T>,
}

impl<T: ISO9660Reader> Read for ISOFileReader<T> {
    unsafe fn initializer(&self) -> Initializer {
        Initializer::nop()
    }

    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        let mut seek = self.seek;
        while !buf.is_empty() && seek < self.size {
            let lba = self.start_lba as u64 + (seek as u64 / 2048);
            if self.buf_lba != Some(lba) {
                self.file.read_at(&mut self.buf, lba)?;
                self.buf_lba = Some(lba);
            }

            let start = seek % 2048;
            let end = min(self.size - (seek / 2048) * 2048, 2048);
            seek += buf.write(&self.buf[start..end]).unwrap();
        }

        let bytes = seek - self.seek;
        self.seek = seek;
        Ok(bytes)
    }
}

impl<T: ISO9660Reader> Seek for ISOFileReader<T> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let seek = match pos {
            SeekFrom::Start(pos) => pos as i64,
            SeekFrom::End(pos) => self.size as i64 + pos,
            SeekFrom::Current(pos) => self.seek as i64 + pos,
        };

        if seek < 0 {
            Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid seek"))
        } else {
            self.seek = seek as usize;
            Ok(seek as u64)
        }
    }
}

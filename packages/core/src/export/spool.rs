//! Finished pages on disk instead of in memory: the page pipeline appends each JPEG as soon as
//! it is encoded and the format writers read them back one at a time, so an export holds at
//! most a handful of pages in RAM however long the book is.

use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};

use super::epub::PageImage;
use super::{space, spool_file};
use crate::domain::{io_at, Error, Result};

/// What a writer needs to know about a page without reading its bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageInfo {
    pub width: u32,
    pub height: u32,
    /// Size of the JPEG in bytes.
    pub bytes: usize,
}

/// Pages in reading order, backed by an unlinked temporary file (freed when dropped).
#[derive(Debug)]
pub struct PageSpool {
    file: File,
    offsets: Vec<u64>,
    info: Vec<PageInfo>,
    end: u64,
    /// Where the file lives, for [`Error::NoSpace`].
    folder: PathBuf,
}

/// A spool failure is an IO failure in the export's folder, like the book's own writes.
fn io(folder: &Path) -> impl Fn(std::io::Error) -> Error + '_ {
    move |e| match space::is_out_of_space(&e) {
        true => space::no_space(folder),
        false => io_at(folder)(e),
    }
}

impl PageSpool {
    pub fn new() -> Result<PageSpool> {
        PageSpool::new_in(None)
    }

    /// Spooled in `dir` when given: the export's own folder, which has to hold the book anyway,
    /// rather than a system temp dir that may be small or RAM-backed. Created if missing. The
    /// file is unlinked right after it is created ([`spool_file`]); stale `.mangas-spool-*.tmp`
    /// files already in the folder are left alone.
    pub fn new_in(dir: Option<&Path>) -> Result<PageSpool> {
        let folder = dir.map_or_else(std::env::temp_dir, Path::to_path_buf);
        let file = match dir {
            Some(d) => spool_file::create_in(d),
            None => tempfile::tempfile(),
        };
        Ok(PageSpool {
            file: file.map_err(io(&folder))?,
            offsets: Vec::new(),
            info: Vec::new(),
            end: 0,
            folder,
        })
    }

    pub fn from_pages(pages: impl IntoIterator<Item = PageImage>) -> Result<PageSpool> {
        let mut spool = PageSpool::new()?;
        for page in pages {
            spool.push(&page)?;
        }
        Ok(spool)
    }

    pub fn push(&mut self, page: &PageImage) -> Result<()> {
        let io = io(&self.folder);
        self.file.seek(SeekFrom::Start(self.end)).map_err(&io)?;
        self.file.write_all(&page.jpeg).map_err(&io)?;
        self.offsets.push(self.end);
        self.end += page.jpeg.len() as u64;
        self.info.push(PageInfo {
            width: page.width,
            height: page.height,
            bytes: page.jpeg.len(),
        });
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.info.len()
    }

    pub fn is_empty(&self) -> bool {
        self.info.is_empty()
    }

    pub fn info(&self) -> &[PageInfo] {
        &self.info
    }

    /// Pages `range` copied one at a time into a new spool in `dir` (see [`PageSpool::new_in`]).
    pub(crate) fn copy_range(&self, range: Range<usize>, dir: Option<&Path>) -> Result<PageSpool> {
        let mut part = PageSpool::new_in(dir)?;
        for i in range {
            part.push(&self.read(i)?)?;
        }
        Ok(part)
    }

    /// Page `index` read back from disk.
    pub fn read(&self, index: usize) -> Result<PageImage> {
        let (Some(&offset), Some(&info)) = (self.offsets.get(index), self.info.get(index)) else {
            return Err(Error::Other(format!("page spool: no page {index}")));
        };
        let mut jpeg = vec![0; info.bytes];
        read_exact_at(&self.file, &mut jpeg, offset).map_err(io(&self.folder))?;
        Ok(PageImage {
            jpeg,
            width: info.width,
            height: info.height,
        })
    }
}

#[cfg(unix)]
fn read_exact_at(file: &File, buf: &mut [u8], offset: u64) -> std::io::Result<()> {
    std::os::unix::fs::FileExt::read_exact_at(file, buf, offset)
}

#[cfg(windows)]
fn read_exact_at(file: &File, mut buf: &mut [u8], mut offset: u64) -> std::io::Result<()> {
    use std::os::windows::fs::FileExt;
    while !buf.is_empty() {
        match file.seek_read(buf, offset)? {
            0 => return Err(std::io::ErrorKind::UnexpectedEof.into()),
            n => {
                buf = &mut buf[n..];
                offset += n as u64;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(fill: u8, len: usize) -> PageImage {
        PageImage {
            jpeg: vec![fill; len],
            width: len as u32,
            height: 2,
        }
    }

    #[test]
    fn reads_back_every_page_in_any_order() {
        let pages = vec![page(1, 10), page(2, 0), page(3, 70_000)];
        let mut spool = PageSpool::from_pages(pages.clone()).unwrap();
        assert_eq!(spool.len(), 3);
        assert_eq!(spool.read(2).unwrap(), pages[2]);
        assert_eq!(spool.read(0).unwrap(), pages[0]);
        spool.push(&page(4, 5)).unwrap();
        assert_eq!(spool.read(3).unwrap(), page(4, 5));
        assert_eq!(spool.read(1).unwrap(), pages[1]);
        assert_eq!(spool.info()[2].bytes, 70_000);
        assert!(spool.read(4).is_err());
    }
}

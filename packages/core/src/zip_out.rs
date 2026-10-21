//! Zip archives that fail quietly: the `zip` crate finishes an unfinished archive when the
//! writer is dropped and prints `ZipWriter::drop: failed to finalize archive` when that fails
//! too (the disk that refused the archive refuses its end as well). Here the underlying writer
//! turns into a sink after its first error or when the caller gives up, so the drop finishes
//! into nothing and the caller's error is the only one shown.

use std::cell::Cell;
use std::io::{self, Seek, SeekFrom, Write};
use std::rc::Rc;

use zip::ZipWriter;

use crate::domain::{Error, Result};

/// `inner` until it fails or the archive is abandoned, then a sink that accepts everything.
/// The sink keeps a position as if the bytes were written: `zip` checks offsets as it finishes.
pub(crate) struct Guarded<W> {
    inner: W,
    dead: Rc<Cell<bool>>,
    pos: u64,
    len: u64,
}

impl<W> Guarded<W> {
    fn check<T>(&self, result: io::Result<T>) -> io::Result<T> {
        if result.is_err() {
            self.dead.set(true);
        }
        result
    }

    fn moved_to(&mut self, pos: u64) -> u64 {
        self.pos = pos;
        self.len = self.len.max(pos);
        pos
    }
}

impl<W: Write> Write for Guarded<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = if self.dead.get() {
            buf.len()
        } else {
            let result = self.inner.write(buf);
            self.check(result)?
        };
        self.moved_to(self.pos + n as u64);
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.dead.get() {
            return Ok(());
        }
        let result = self.inner.flush();
        self.check(result)
    }
}

impl<W: Seek> Seek for Guarded<W> {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        if !self.dead.get() {
            let result = self.inner.seek(to);
            let pos = self.check(result)?;
            return Ok(self.moved_to(pos));
        }
        let pos = match to {
            SeekFrom::Start(n) => Some(n),
            SeekFrom::Current(d) => self.pos.checked_add_signed(d),
            SeekFrom::End(d) => self.len.checked_add_signed(d),
        };
        let pos = pos.ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
        Ok(self.moved_to(pos))
    }
}

/// Writes one archive into `out` with `fill`, then finishes it and flushes `out`. On any error
/// the half-written archive is dropped without a word; the error is returned, prefixed `what`.
pub(crate) fn write_zip<W: Write + Seek>(
    out: W,
    what: &str,
    fill: impl FnOnce(&mut ZipWriter<Guarded<W>>) -> Result<()>,
) -> Result<()> {
    let dead = Rc::new(Cell::new(false));
    let mut zip = ZipWriter::new(Guarded {
        inner: out,
        dead: dead.clone(),
        pos: 0,
        len: 0,
    });
    if let Err(e) = fill(&mut zip) {
        dead.set(true);
        return Err(e);
    }
    let mut out = zip.finish().map_err(|e| match e {
        zip::result::ZipError::Io(io) => Error::Io(io),
        e => Error::Other(format!("{what}: {e}")),
    })?;
    out.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use zip::write::SimpleFileOptions;

    /// Accepts `room` bytes, then fails like a full disk.
    struct Full {
        room: usize,
        data: Cursor<Vec<u8>>,
    }

    impl Write for Full {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            if self.data.get_ref().len() + buf.len() > self.room {
                return Err(io::Error::other("no space left on device"));
            }
            self.data.write(buf)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Seek for Full {
        fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
            self.data.seek(pos)
        }
    }

    fn fill_big(zip: &mut ZipWriter<Guarded<&mut Full>>) -> Result<()> {
        zip.start_file("a", SimpleFileOptions::default())
            .map_err(|e| Error::Other(e.to_string()))?;
        zip.write_all(&[7; 4096])?;
        Ok(())
    }

    #[test]
    fn a_full_disk_fails_with_one_error_and_writes_nothing_after_it() {
        let mut out = Full {
            room: 100,
            data: Cursor::new(Vec::new()),
        };
        assert!(write_zip(&mut out, "test", fill_big).is_err());
        assert!(out.data.get_ref().len() <= 100);
    }

    #[test]
    fn an_archive_that_fits_is_complete() {
        let mut out = Full {
            room: 1 << 20,
            data: Cursor::new(Vec::new()),
        };
        write_zip(&mut out, "test", fill_big).unwrap();
        let archive = zip::ZipArchive::new(Cursor::new(out.data.into_inner())).unwrap();
        assert_eq!(archive.len(), 1);
    }
}

//! A PDF written object by object straight into its file, byte for byte what `pdf_writer::Pdf`
//! builds in memory: the header, the objects in the order written, the xref table, the trailer.

use std::fmt::Write as _;
use std::io::Write;

use pdf_writer::{Chunk, Ref};

use crate::domain::Result;

const HEADER: &[u8] = b"%PDF-1.7\n%\x80\x80\x80\x80\n\n";

/// A batch of objects and where each starts inside the batch.
pub(super) struct Objects {
    chunk: Chunk,
    starts: Vec<(i32, usize)>,
}

impl Objects {
    pub fn new() -> Objects {
        Objects::with_capacity(0)
    }

    /// Room for `bytes` up front, so a page's image isn't copied again as the chunk grows.
    pub fn with_capacity(bytes: usize) -> Objects {
        Objects {
            chunk: Chunk::with_capacity(bytes),
            starts: Vec::new(),
        }
    }

    /// The chunk to write object `id` into next.
    pub fn at(&mut self, id: Ref) -> &mut Chunk {
        self.starts.push((id.get(), self.chunk.len()));
        &mut self.chunk
    }
}

pub(super) struct PdfStream<W: Write> {
    out: W,
    pos: usize,
    offsets: Vec<(i32, usize)>,
}

impl<W: Write> PdfStream<W> {
    pub fn new(mut out: W) -> Result<PdfStream<W>> {
        out.write_all(HEADER)?;
        Ok(PdfStream {
            out,
            pos: HEADER.len(),
            offsets: Vec::new(),
        })
    }

    pub fn write(&mut self, objects: Objects) -> Result<()> {
        let base = self.pos;
        self.offsets
            .extend(objects.starts.iter().map(|&(id, at)| (id, base + at)));
        self.out.write_all(objects.chunk.as_bytes())?;
        self.pos += objects.chunk.len();
        Ok(())
    }

    /// Writes the xref table and the trailer (catalog `root`, document info `info`).
    pub fn finish(mut self, root: Ref, info: Ref) -> Result<W> {
        let (len, mut tail) = xref(self.offsets);
        let _ = write!(
            tail,
            "trailer\n<<\n  /Size {len}\n  /Root {} 0 R\n  /Info {} 0 R\n>>\nstartxref\n{}\n%%EOF",
            root.get(),
            info.get(),
            self.pos
        );
        self.out.write_all(tail.as_bytes())?;
        Ok(self.out)
    }
}

/// The classic xref table: one entry per id up to the largest, unused ids chained as free.
fn xref(mut offsets: Vec<(i32, usize)>) -> (i32, String) {
    offsets.sort_unstable();
    let len = 1 + offsets.last().map_or(0, |p| p.0);
    let mut out = format!("xref\n0 {len}\n");
    if offsets.is_empty() {
        out.push_str("0000000000 65535 f\r\n");
    }
    let mut free = 0;
    for (i, &(id, offset)) in offsets.iter().enumerate() {
        while free < id {
            let mut next = free + 1;
            if next == id {
                for &(used, _) in &offsets[i..] {
                    if next < used {
                        break;
                    }
                    next = used + 1;
                }
            }
            let gen = if free == 0 { 65535 } else { 0 };
            let _ = write!(out, "{:010} {gen:05} f\r\n", next % len);
            free += 1;
        }
        let _ = write!(out, "{offset:010} 00000 n\r\n");
        free += 1;
    }
    (len, out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdf_writer::writers::{Catalog, DocumentInfo};
    use pdf_writer::{Finish, Pdf, TextStr};

    const ROOT: Ref = Ref::new(1);
    const INFO: Ref = Ref::new(3);

    #[test]
    fn same_bytes_as_pdf_writer_in_memory() {
        let mut pdf = Pdf::new();
        pdf.document_info(INFO).title(TextStr("t"));
        pdf.catalog(ROOT).pages(Ref::new(2));
        pdf.stream(Ref::new(10), b"page one");
        pdf.stream(Ref::new(12), b"page two");
        pdf.stream(Ref::new(7), b"out of order");
        let expected = pdf.finish();

        let mut stream = PdfStream::new(Vec::new()).unwrap();
        let mut head = Objects::new();
        head.at(INFO)
            .indirect(INFO)
            .start::<DocumentInfo>()
            .title(TextStr("t"));
        head.at(ROOT)
            .indirect(ROOT)
            .start::<Catalog>()
            .pages(Ref::new(2))
            .finish();
        stream.write(head).unwrap();
        for (id, data) in [
            (10, &b"page one"[..]),
            (12, b"page two"),
            (7, b"out of order"),
        ] {
            let mut objs = Objects::new();
            objs.at(Ref::new(id)).stream(Ref::new(id), data);
            stream.write(objs).unwrap();
        }
        let got = stream.finish(ROOT, INFO).unwrap();
        assert_eq!(
            String::from_utf8_lossy(&got),
            String::from_utf8_lossy(&expected)
        );
        assert_eq!(got, expected);
    }
}

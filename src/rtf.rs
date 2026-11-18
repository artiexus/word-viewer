//! Bounded RTF output buffer.

/// Output cap; rendering stops once it is reached.
const MAX_RTF: usize = 32 << 20;
/// Most cells RichEdit shows in a table row.
const MAX_CELLS: usize = 63;
/// Width of a table without usable column widths, in twips (6.5").
const TABLE_WIDTH: i32 = 9360;
/// Widest table accepted from a document, in twips (22", Word's page limit).
const MAX_WIDTH: i32 = 31680;

pub struct Rtf {
    buf: String,
    pub full: bool,
}

impl Rtf {
    pub fn new() -> Rtf {
        let mut r = Rtf { buf: String::new(), full: false };
        // color 1: table borders
        r.put("{\\rtf1\\ansi\\deff0{\\fonttbl{\\f0 Segoe UI;}}{\\colortbl;\\red191\\green191\\blue191;}\\fs22\n");
        r
    }

    /// Append one table row. `cells` are RTF built by this type (see `put`);
    /// `edges` the right edge of each cell in twips, used if there's one per
    /// cell and they ascend, else the cells share the width equally.
    pub fn row(&mut self, cells: &[String], edges: &[i32]) {
        // RichEdit allows at most 63 cells in a row: join any beyond that.
        let n = cells.len().clamp(1, MAX_CELLS);
        let ok = edges.len() == cells.len() && n == cells.len() && edges.first().is_some_and(|&e| e > 0) && edges.windows(2).all(|w| w[0] < w[1]) && edges.last().is_some_and(|&e| e <= MAX_WIDTH);
        self.put("\\trowd\\trgaph80\\trleft0");
        for i in 0..n {
            let edge = edges.get(i).filter(|_| ok).copied().unwrap_or((i as i32 + 1) * (TABLE_WIDTH / n as i32).max(400));
            self.put(&format!("\\clbrdrt\\brdrs\\brdrw10\\brdrcf1\\clbrdrl\\brdrs\\brdrw10\\brdrcf1\\clbrdrb\\brdrs\\brdrw10\\brdrcf1\\clbrdrr\\brdrs\\brdrw10\\brdrcf1\\cellx{edge}"));
        }
        self.put("\n\\pard\\intbl\\sa0 ");
        for (i, c) in cells.iter().enumerate() {
            if i >= n {
                self.put("\\line "); // a cell past the limit, joined to the last
            }
            self.put(c);
            if i + 1 < n {
                self.put("\\cell ");
            }
        }
        self.put("\\cell\\row\n");
    }

    /// An empty buffer for part of a document (no RTF header), to be appended
    /// to the main one with `put` once it is complete.
    pub fn fragment() -> Rtf {
        Rtf { buf: String::new(), full: false }
    }

    /// Bytes written so far.
    pub fn size(&self) -> usize {
        self.buf.len()
    }

    /// Take what has been written, leaving the buffer empty.
    pub fn take(&mut self) -> String {
        std::mem::take(&mut self.buf)
    }

    /// Append trusted RTF markup (never document content), or RTF built by this
    /// type, whose document content is already escaped.
    pub fn put(&mut self, s: &str) {
        if self.full || self.buf.len() + s.len() > MAX_RTF {
            self.full = true;
            return;
        }
        self.buf.push_str(s);
    }

    /// Append one character of document text, escaped.
    pub fn ch(&mut self, c: u32) {
        match c {
            0x5C | 0x7B | 0x7D => {
                // \ { } are RTF syntax; escape them
                let s = [b'\\', c as u8];
                self.put(std::str::from_utf8(&s).unwrap_or(""));
            }
            0x20..=0x7E => {
                let s = [c as u8];
                self.put(std::str::from_utf8(&s).unwrap_or(""));
            }
            0..=0x1F => {}
            0x7F..=0xFFFF => self.put(&format!("\\u{}?", c as u16 as i16)),
            0x10000..=0x10FFFF => {
                let c = c - 0x10000;
                let hi = (0xD800 + (c >> 10)) as u16 as i16;
                let lo = (0xDC00 + (c & 0x3FF)) as u16 as i16;
                self.put(&format!("\\u{}?\\u{}?", hi, lo));
            }
            _ => self.put("\\u-3?"), // U+FFFD
        }
    }

    pub fn finish(mut self) -> String {
        self.buf.push('}');
        self.buf
    }
}

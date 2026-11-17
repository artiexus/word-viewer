//! word/document.xml (with word/styles.xml and word/numbering.xml) -> RTF.

use crate::numbers::{bullet, indent, number, symbol};
use crate::rtf::Rtf;
use std::collections::HashMap;

fn find(hay: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    hay.get(from..)?.windows(needle.len()).position(|w| w == needle).map(|i| from + i)
}

/// Value of attribute `key` (e.g. `w:val`) within a single tag.
fn attr<'a>(tag: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let mut k = key.to_vec();
    k.extend_from_slice(b"=\"");
    let start = find(tag, 0, &k)? + k.len();
    let len = tag[start..].iter().position(|&c| c == b'"')?;
    Some(&tag[start..start + len])
}

/// Attribute `key` as a number.
fn num(tag: &[u8], key: &[u8]) -> Option<i64> {
    std::str::from_utf8(attr(tag, key)?).ok()?.parse().ok()
}

/// A toggle like <w:b/> is on unless w:val says otherwise.
fn on(tag: &[u8]) -> bool {
    match attr(tag, b"w:val") {
        Some(v) => !(v.starts_with(b"0") || v == b"false" || v == b"none"),
        None => true,
    }
}

/// Decode XML text, entities included, calling `out` for each character.
fn unescape(raw: &[u8], mut out: impl FnMut(u32)) {
    let s = String::from_utf8_lossy(raw);
    let mut it = s.char_indices();
    while let Some((i, c)) = it.next() {
        if c == '&' {
            if let Some(semi) = s.as_bytes()[i..].iter().take(12).position(|&b| b == b';') {
                let ent = &s[i + 1..i + semi];
                let v = match ent {
                    "amp" => Some('&' as u32),
                    "lt" => Some('<' as u32),
                    "gt" => Some('>' as u32),
                    "quot" => Some('"' as u32),
                    "apos" => Some('\'' as u32),
                    _ => match ent.strip_prefix("#x").or_else(|| ent.strip_prefix("#X")) {
                        Some(h) => u32::from_str_radix(h, 16).ok(),
                        None => ent.strip_prefix('#').and_then(|d| d.parse().ok()),
                    },
                };
                if let Some(v) = v {
                    out(v);
                    for _ in 0..semi {
                        it.next();
                    }
                    continue;
                }
            }
        }
        out(c as u32);
    }
}

/// One XML tag: its name, everything between < and >, and whether it is a
/// closing tag or closes itself.
struct Tag<'a> {
    name: &'a [u8],
    tag: &'a [u8],
    close: bool,
    selfclose: bool,
}

/// The tags of `x`, from byte `p` on.
struct Tags<'a> {
    x: &'a [u8],
    p: usize,
}

impl<'a> Iterator for Tags<'a> {
    type Item = Tag<'a>;

    fn next(&mut self) -> Option<Tag<'a>> {
        let lt = find(self.x, self.p, b"<")?;
        let gt = find(self.x, lt, b">")?;
        let tag = &self.x[lt + 1..gt];
        let close = tag.first() == Some(&b'/');
        let body = if close { &tag[1..] } else { tag };
        let nl = body.iter().position(|&c| c == b'/' || c.is_ascii_whitespace()).unwrap_or(body.len());
        self.p = gt + 1;
        Some(Tag { name: &body[..nl], tag, close, selfclose: tag.last() == Some(&b'/') })
    }
}

/// A paragraph style: heading level (see HEAD), and list (numId, level).
#[derive(Clone, Default)]
struct Style {
    head: Option<usize>,
    num_id: Option<u32>,
    ilvl: Option<u8>,
    base: Option<Vec<u8>>,
}

/// Heading level for HEAD from a built-in style's name or an outline level.
fn heading_level(n: i64) -> usize {
    match n {
        1 => 1,
        2 => 2,
        _ => 3,
    }
}

/// Paragraph styles by id, each with what it inherits. Style ids are localized
/// (German Word's "Heading 1" is "berschrift1"), but built-in style names
/// ("heading 1", "title") and outline levels are not.
fn styles(x: &[u8]) -> HashMap<Vec<u8>, Style> {
    let mut raw: HashMap<Vec<u8>, Style> = HashMap::new();
    let mut cur: Option<(Vec<u8>, Style)> = None;
    for t in (Tags { x, p: 0 }) {
        if t.name == b"w:style" {
            if let Some((id, s)) = cur.take() {
                raw.insert(id, s);
            }
            if !t.close && !t.selfclose {
                cur = attr(t.tag, b"w:styleId").map(|id| (id.to_vec(), Style::default()));
            }
            continue;
        }
        let Some((_, s)) = cur.as_mut() else { continue };
        let v = attr(t.tag, b"w:val");
        match t.name {
            b"w:name" => {
                let name = v.unwrap_or_default().to_ascii_lowercase();
                if name == b"title" {
                    s.head = Some(4);
                } else if let Some(n) = name.strip_prefix(b"heading ").and_then(|n| std::str::from_utf8(n).ok()?.parse().ok()) {
                    s.head = (1..=9).contains(&n).then(|| heading_level(n));
                }
            }
            b"w:outlineLvl" if s.head.is_none() => s.head = num(t.tag, b"w:val").filter(|n| (0..9).contains(n)).map(|n| heading_level(n + 1)),
            b"w:basedOn" => s.base = v.map(|v| v.to_vec()),
            b"w:numId" => s.num_id = num(t.tag, b"w:val").map(|n| n as u32),
            b"w:ilvl" => s.ilvl = num(t.tag, b"w:val").map(|n| n.clamp(0, 8) as u8),
            _ => {}
        }
    }
    let mut out = HashMap::new();
    for (id, s) in &raw {
        let mut res = s.clone();
        let mut base = s.base.clone();
        for _ in 0..16 {
            let Some(b) = base.and_then(|b| raw.get(&b)) else { break };
            res.head = res.head.or(b.head);
            res.num_id = res.num_id.or(b.num_id);
            res.ilvl = res.ilvl.or(b.ilvl);
            base = b.base.clone();
        }
        out.insert(id.clone(), res);
    }
    out
}

/// One list level: first number, number format (as in .doc: see `number`),
/// and label template, in which %1 to %9 stand for the levels' numbers.
#[derive(Clone, Default)]
struct Lvl {
    start: i32,
    nfc: u8,
    text: Vec<u32>,
}

/// numbering.xml: list definitions (abstractNum) and the list instances
/// (num) paragraphs refer to, with the numbers counted so far.
#[derive(Default)]
struct Numbering {
    abs: HashMap<i64, Vec<Lvl>>,
    /// numId -> abstractNumId, and any start overrides per level.
    nums: HashMap<u32, (i64, [Option<i32>; 9])>,
    counts: HashMap<u32, [Option<i32>; 9]>,
}

impl Numbering {
    fn read(x: &[u8]) -> Numbering {
        let mut n = Numbering::default();
        let (mut abs, mut lvl, mut inst, mut over): (Option<i64>, Option<usize>, Option<u32>, Option<usize>) = (None, None, None, None);
        for t in (Tags { x, p: 0 }) {
            let level = || num(t.tag, b"w:ilvl").filter(|l| (0..9).contains(l)).map(|l| l as usize);
            match t.name {
                b"w:abstractNum" => {
                    abs = if t.close { None } else { num(t.tag, b"w:abstractNumId") };
                    if let Some(a) = abs {
                        n.abs.insert(a, vec![Lvl::default(); 9]);
                    }
                }
                b"w:num" => {
                    inst = if t.close { None } else { num(t.tag, b"w:numId").map(|i| i as u32) };
                    if let Some(i) = inst {
                        n.nums.insert(i, (-1, [None; 9]));
                    }
                }
                b"w:lvl" => lvl = if t.close { None } else { level() },
                b"w:lvlOverride" => over = if t.close { None } else { level() },
                b"w:abstractNumId" => {
                    if let (Some(i), Some(a)) = (inst, num(t.tag, b"w:val")) {
                        n.nums.entry(i).or_insert((a, [None; 9])).0 = a;
                    }
                }
                b"w:startOverride" => {
                    if let (Some(i), Some(o), Some(v)) = (inst, over, num(t.tag, b"w:val")) {
                        n.nums.entry(i).or_insert((-1, [None; 9])).1[o] = Some(v.clamp(0, i32::MAX as i64) as i32);
                    }
                }
                _ => {
                    let (Some(a), Some(l)) = (abs, lvl) else { continue };
                    let Some(lv) = n.abs.get_mut(&a).and_then(|v| v.get_mut(l)) else { continue };
                    match t.name {
                        b"w:start" => lv.start = num(t.tag, b"w:val").map_or(0, |v| v.clamp(0, i32::MAX as i64) as i32),
                        b"w:numFmt" => {
                            lv.nfc = match attr(t.tag, b"w:val").unwrap_or_default() {
                                b"upperRoman" => 1,
                                b"lowerRoman" => 2,
                                b"upperLetter" => 3,
                                b"lowerLetter" => 4,
                                b"decimalZero" => 22,
                                b"bullet" => 23,
                                b"none" => 255,
                                _ => 0,
                            }
                        }
                        b"w:lvlText" => {
                            lv.text.clear();
                            unescape(attr(t.tag, b"w:val").unwrap_or_default(), |c| lv.text.push(c));
                        }
                        _ => {}
                    }
                }
            }
        }
        n
    }

    /// Count a list paragraph and return its label and whether it is a
    /// bullet, or None if its list isn't defined.
    fn label(&mut self, id: u32, ilvl: u8) -> Option<(Vec<u32>, bool)> {
        let (abs, starts) = self.nums.get(&id)?;
        let lvls = self.abs.get(abs)?;
        let start = |k: usize| starts[k].unwrap_or(lvls[k].start);
        let d = ilvl.min(8) as usize;
        let c = self.counts.entry(id).or_insert([None; 9]);
        c[d] = Some(c[d].map_or(start(d), |n| n.saturating_add(1)));
        c[d + 1..].fill(None); // deeper levels restart
        let lvl = &lvls[d];
        if lvl.nfc == 255 {
            return Some((Vec::new(), false));
        }
        let mut out = Vec::new();
        let mut it = lvl.text.iter().peekable();
        while let Some(&ch) = it.next() {
            match it.peek() {
                Some(&&k @ 0x31..=0x39) if ch == '%' as u32 => {
                    it.next();
                    let k = k as usize - 0x31;
                    out.extend(number(c[k].unwrap_or(start(k)), lvls[k].nfc).chars().map(|c| c as u32));
                }
                _ => out.push(u16::try_from(ch).map_or(ch, if lvl.nfc == 23 { bullet } else { symbol })),
            }
        }
        Some((out, lvl.nfc == 23))
    }
}

/// Above this, a table row is written out as a paragraph instead, so a
/// pathological table can't hold unbounded memory.
const FLUSH: usize = 1 << 20;

pub fn render(x: &[u8], styles_xml: &[u8], numbering_xml: &[u8]) -> String {
    const HEAD: [&str; 5] = ["", "\\b\\fs40 ", "\\b\\fs32 ", "\\b\\fs28 ", "\\b\\fs52 "];
    let styles = styles(styles_xml);
    let mut numbering = Numbering::read(numbering_xml);
    let mut r = Rtf::new();
    let (mut bold, mut ital, mut under, mut vanish, mut head) = (false, false, false, false, 0);
    let (mut skip, mut inppr) = (0, false);
    let (mut pstyle, mut num_id, mut ilvl): (Option<&[u8]>, Option<u32>, Option<u8>) = (None, None, None);
    // Tables: text in a table goes to `cell`; a row's cells collect in `cells`.
    let (mut depth, mut cell, mut cells, mut edges, mut grid) = (0, Rtf::fragment(), Vec::<String>::new(), Vec::new(), Vec::<i32>::new());
    let (mut col, mut span, mut row_bytes) = (0, 1, 0);
    let mut after_table = false; // space the next paragraph from the table above
    let mut tags = Tags { x, p: 0 };
    while !r.full {
        let Some(Tag { name, tag, close, selfclose }) = tags.next() else { break };
        let out = if depth > 0 { &mut cell } else { &mut r };
        match name {
            // Not shown: the fallback copy of mc:Choice content, the old place
            // of moved text, and formatting before a tracked change.
            b"mc:Fallback" | b"w:moveFrom" | b"w:pPrChange" | b"w:rPrChange" if !selfclose => {
                skip = if close { skip - (skip > 0) as usize } else { skip + 1 };
            }
            _ if skip > 0 => {}
            b"w:p" if close => out.put(if depth > 0 { "\\line " } else { "\\par\n" }),
            b"w:p" => {
                (bold, ital, under, head, pstyle, num_id, ilvl) = (false, false, false, 0, None, None, None);
                if depth == 0 {
                    out.put("\\pard\\sa160 ");
                    if std::mem::take(&mut after_table) {
                        out.put("\\sb120 ");
                    }
                }
            }
            b"w:pStyle" if !close => {
                pstyle = attr(tag, b"w:val");
                head = match pstyle.and_then(|v| styles.get(v)) {
                    Some(s) => s.head.unwrap_or(0),
                    // no styles.xml: go by English style ids
                    None => match pstyle.unwrap_or_default() {
                        v if v.starts_with(b"Title") => 4,
                        v => v.strip_prefix(b"Heading").map_or(0, |rest| match rest.first() {
                            Some(&d @ b'1'..=b'3') => (d - b'0') as usize,
                            _ => 3,
                        }),
                    },
                };
            }
            b"w:numId" if inppr && !close => num_id = num(tag, b"w:val").map(|n| n as u32),
            b"w:ilvl" if inppr && !close => ilvl = num(tag, b"w:val").map(|n| n.clamp(0, 8) as u8),
            b"w:pPr" => {
                inppr = !close && !selfclose; // its <w:tab> children are tab stops, not tabs
                let st = pstyle.and_then(|v| styles.get(v));
                let id = num_id.or(st.and_then(|s| s.num_id)).unwrap_or(0);
                if close && id != 0 {
                    let lv = ilvl.or(st.and_then(|s| s.ilvl)).unwrap_or(0);
                    match numbering.label(id, lv) {
                        // a numbered heading shows its number; a bulleted one, nothing
                        Some((label, bullet)) if head > 0 => {
                            if !bullet && !label.is_empty() {
                                out.put("{");
                                out.put(HEAD[head]);
                                label.into_iter().for_each(|c| out.ch(c));
                                out.put(" }");
                            }
                        }
                        Some((label, _)) if depth > 0 => {
                            label.into_iter().for_each(|c| out.ch(c));
                            out.put(" ");
                        }
                        Some((label, _)) => {
                            out.put(&indent(lv, label.len()));
                            label.into_iter().for_each(|c| out.ch(c));
                            out.put("\\tab ");
                        }
                        None if head == 0 => out.put(&format!("{}\\bullet\\tab ", indent(lv, 1))),
                        None => {}
                    }
                }
            }
            b"w:ind" if !close && depth == 0 => {
                if let Some(fi) = num(tag, b"w:firstLine") {
                    out.put(&format!("\\fi{} ", fi.clamp(-14400, 14400)));
                }
            }
            b"w:r" if !close => (bold, ital, under, vanish) = (false, false, false, false),
            b"w:vanish" if !close => vanish = on(tag), // hidden text: Word doesn't show or print it
            b"w:b" if !close => bold = on(tag),
            b"w:i" if !close => ital = on(tag),
            b"w:u" if !close => under = on(tag),
            b"w:tab" if !close && !inppr => out.put("\\tab "),
            b"w:br" | b"w:cr" if !close => out.put("\\line "),
            b"w:tbl" if !selfclose => {
                if close {
                    depth -= (depth > 0) as usize;
                    after_table = depth == 0;
                } else {
                    depth += 1;
                    if depth == 1 {
                        grid.clear();
                    }
                }
            }
            // the column widths, in twips
            b"w:gridCol" if depth == 1 && !close => grid.push(num(tag, b"w:w").unwrap_or(0).clamp(0, 31680) as i32),
            b"w:tr" if depth == 1 && !selfclose => {
                if close {
                    if row_bytes <= FLUSH {
                        r.row(&cells, &edges);
                    } else {
                        r.put("\\pard\\sa60 ");
                        r.put(&cells.join("\\tab\\u9474?  "));
                        r.put("\\par\n");
                    }
                }
                (col, row_bytes) = (0, 0);
                cells.clear();
                edges.clear();
            }
            b"w:gridSpan" if depth == 1 && !close => span = num(tag, b"w:val").unwrap_or(1).clamp(1, 63) as usize,
            b"w:tc" if depth == 1 && !selfclose => {
                if close {
                    let mut c = cell.take();
                    if c.ends_with("\\line ") {
                        c.truncate(c.len() - 6); // after the cell's last paragraph
                    }
                    // the cell's right edge: the widths of the columns it spans so far
                    col += span;
                    edges.push(grid.get(..col).map_or(0, |g| g.iter().sum()));
                    row_bytes += c.len().max(16);
                    cells.push(c);
                } else {
                    cell.take();
                }
                span = 1;
            }
            b"w:tc" if depth > 1 && close => out.put("\\tab "), // a nested table's cells
            b"w:t" if !close && !selfclose => {
                let Some(te) = find(x, tags.p, b"</w:t>") else { break };
                if !vanish {
                    out.put("{");
                    out.put(HEAD[head]);
                    if bold {
                        out.put("\\b ");
                    }
                    if ital {
                        out.put("\\i ");
                    }
                    if under {
                        out.put("\\ul ");
                    }
                    unescape(&x[tags.p..te], |c| out.ch(c));
                    out.put("}");
                }
                tags.p = te + 6;
            }
            _ => {}
        }
        if depth > 0 && cell.size() + row_bytes > FLUSH {
            // a giant row: write what there is as a paragraph
            r.put("\\pard\\sa60 ");
            r.put(&cells.join("\\tab\\u9474?  "));
            r.put(&cell.take());
            r.put("\\par\n");
            cells.clear();
            edges.clear();
            row_bytes = 0;
        }
    }
    r.finish()
}

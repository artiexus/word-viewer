//! Malicious and malformed inputs must be rejected quickly, never panic or blow up.
use std::time::{Duration, Instant};

const SAMPLE: &[u8] = include_bytes!("fixtures/sample.docx");

/// A minimal zip with one stored member; sizes can lie on purpose.
fn zip(name: &str, data: &[u8], claimed_size: u32, method: u16) -> Vec<u8> {
    let n = name.as_bytes();
    let mut z = Vec::new();
    z.extend_from_slice(&0x04034b50u32.to_le_bytes());
    z.extend_from_slice(&[20, 0, 0, 0]);
    z.extend_from_slice(&method.to_le_bytes());
    z.extend_from_slice(&[0; 8]); // time, date, crc
    z.extend_from_slice(&(data.len() as u32).to_le_bytes());
    z.extend_from_slice(&claimed_size.to_le_bytes());
    z.extend_from_slice(&(n.len() as u16).to_le_bytes());
    z.extend_from_slice(&[0, 0]);
    z.extend_from_slice(n);
    z.extend_from_slice(data);
    let cd = z.len() as u32;
    z.extend_from_slice(&0x02014b50u32.to_le_bytes());
    z.extend_from_slice(&[20, 0, 20, 0, 0, 0]);
    z.extend_from_slice(&method.to_le_bytes());
    z.extend_from_slice(&[0; 8]);
    z.extend_from_slice(&(data.len() as u32).to_le_bytes());
    z.extend_from_slice(&claimed_size.to_le_bytes());
    z.extend_from_slice(&(n.len() as u16).to_le_bytes());
    z.extend_from_slice(&[0; 12]); // extra, comment, disk, attrs
    z.extend_from_slice(&0u32.to_le_bytes()); // local header offset
    z.extend_from_slice(n);
    let cdlen = z.len() as u32 - cd;
    z.extend_from_slice(&0x06054b50u32.to_le_bytes());
    z.extend_from_slice(&[0, 0, 0, 0, 1, 0, 1, 0]);
    z.extend_from_slice(&cdlen.to_le_bytes());
    z.extend_from_slice(&cd.to_le_bytes());
    z.extend_from_slice(&[0, 0]);
    z
}

fn quick<T>(f: impl FnOnce() -> T) -> T {
    let t = Instant::now();
    let r = f();
    assert!(t.elapsed() < Duration::from_secs(5), "took {:?}", t.elapsed());
    r
}

#[test]
fn sample_renders() {
    let rtf = word_viewer::to_rtf(SAMPLE).expect("sample should render");
    assert!(rtf.contains("Test Document") && rtf.contains("caf\\u233?"));
}

#[test]
fn every_truncation_is_rejected_or_renders() {
    for n in 0..SAMPLE.len() {
        let _ = word_viewer::to_rtf(&SAMPLE[..n]);
    }
}

#[test]
fn every_single_byte_corruption_is_survivable() {
    for i in 0..SAMPLE.len() {
        let mut d = SAMPLE.to_vec();
        d[i] ^= 0xFF;
        let _ = word_viewer::to_rtf(&d);
    }
}

#[test]
fn zip_claiming_4gb_is_rejected() {
    let xml = b"<w:document><w:body/></w:document>";
    assert!(word_viewer::to_rtf(&zip("word/document.xml", xml, u32::MAX, 8)).is_none());
    assert!(word_viewer::to_rtf(&zip("word/document.xml", xml, u32::MAX, 0)).is_none());
}

#[test]
fn rtf_injection_is_escaped() {
    let xml = br"<w:p><w:r><w:t>}{\object\objemb{\*\objdata 0102}}{\field{\*\fldinst HYPERLINK x}}</w:t></w:r></w:p>";
    let rtf = word_viewer::to_rtf(&zip("word/document.xml", xml, xml.len() as u32, 0)).unwrap();
    assert!(!rtf.contains(r"{\object") && !rtf.contains(r"{\field"));
    assert!(rtf.contains(r"\}\{\\object"));
}

#[test]
fn entity_flood_is_fast() {
    let mut xml = b"<w:p><w:r><w:t>".to_vec();
    xml.extend(std::iter::repeat_n(b'&', 20 << 20));
    xml.extend_from_slice(b"</w:t></w:r></w:p>");
    quick(|| word_viewer::to_rtf(&zip("word/document.xml", &xml, xml.len() as u32, 0)));
}

#[test]
fn tag_flood_is_fast_and_output_is_capped() {
    let xml: Vec<u8> = b"<w:tc><w:b/>".repeat(5 << 20);
    let rtf = quick(|| word_viewer::to_rtf(&zip("word/document.xml", &xml, xml.len() as u32, 0))).unwrap();
    assert!(rtf.len() <= (32 << 20) + 1);
}

#[test]
fn oversized_input_is_rejected() {
    assert!(word_viewer::to_rtf(&vec![0u8; word_viewer::MAX_FILE + 1]).is_none());
}

/// A compound file whose sector chains point at themselves.
#[test]
fn cyclic_compound_file_is_rejected() {
    let mut z = vec![0u8; 512 * 3];
    z[..8].copy_from_slice(b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1");
    z[0x1E] = 9; // 512-byte sectors
    z[0x2C] = 1; // one FAT sector...
    z[0x4C..0x50].copy_from_slice(&0u32.to_le_bytes()); // ...which is sector 0
    z[0x30..0x34].copy_from_slice(&1u32.to_le_bytes()); // directory at sector 1
    z[0x3C..0x40].copy_from_slice(&0xFFFF_FFFEu32.to_le_bytes());
    z[0x44..0x48].copy_from_slice(&0xFFFF_FFFEu32.to_le_bytes());
    // FAT: sector 0 -> 0, sector 1 -> 1 (cycles)
    z[512..516].copy_from_slice(&0u32.to_le_bytes());
    z[516..520].copy_from_slice(&1u32.to_le_bytes());
    assert!(quick(|| word_viewer::to_rtf(&z)).is_none());
}

#[test]
fn docx_hidden_text_is_not_shown() {
    let xml = br"<w:p><w:r><w:t>Shown </w:t></w:r><w:r><w:rPr><w:vanish/></w:rPr><w:t>SECRET</w:t></w:r><w:r><w:t>again</w:t></w:r></w:p>";
    let rtf = word_viewer::to_rtf(&zip("word/document.xml", xml, xml.len() as u32, 0)).unwrap();
    assert!(rtf.contains("Shown") && rtf.contains("again") && !rtf.contains("SECRET"));
}

// ---- legacy Word (.doc) ----

/// Saved by OpenOffice as Word 97: a title, headings, bold/italic/underline, a
/// bulleted list, a two-level numbered list, a hyperlink field, a table with
/// an empty cell, hidden text, and a deletion and an insertion made with
/// change tracking on.
const DOC: &[u8] = include_bytes!("fixtures/sample.doc");

#[test]
fn doc_renders_with_formatting() {
    let rtf = word_viewer::to_rtf(DOC).expect("sample.doc should render");
    for want in [
        r"{\b\fs52 Garden Notes}",
        r"{\b\fs40 First Section}",
        r"{\b\fs32 \i Shopping List}", // Heading 2 is italic in OpenOffice
        r"{\b bold}",
        r"{\i italic}",
        r"{\ul underlined}",
        "See the website for more.", // the field's result, not its code
    ] {
        assert!(rtf.contains(want), "missing {want} in {rtf}");
    }
}

#[test]
fn doc_lists_are_numbered_and_bulleted() {
    let paras = word_viewer::read(DOC, "sample.doc").unwrap();
    // headings are in an outline-numbering list too, but show no number
    for item in ["\u{2022} Seeds", "\u{2022} Compost", "1. Dig", "2. Plant", "1. Water daily", "2. Weed weekly", "3. Harvest", "Steps"] {
        assert!(paras.iter().any(|p| p == item), "missing {item:?} in {paras:?}");
    }
}

#[test]
fn doc_tables_are_tables() {
    let rtf = word_viewer::to_rtf(DOC).unwrap();
    // three columns with the widths stored in the file
    assert_eq!(rtf.matches(r"\row").count(), 3);
    assert!(rtf.contains(r"\cellx3324") && rtf.contains(r"\cellx9972"), "{rtf}");
}

#[test]
fn doc_deleted_and_hidden_text_is_not_shown() {
    let rtf = word_viewer::to_rtf(DOC).unwrap();
    assert!(rtf.contains("Visible start. Visible middle. Visible end.") && rtf.contains("INSERTED-WORDS"));
    assert!(!rtf.contains("DELETED-WORDS") && !rtf.contains("HIDDEN-TEXT"));
}

#[test]
fn doc_table_rows_survive_an_empty_cell() {
    let paras = word_viewer::read(DOC, "sample.doc").unwrap();
    for row in ["Plant Count Notes", "Tomato 12", "Basil 30 Pots", "After the table."] {
        assert!(paras.iter().any(|p| p == row), "missing row {row:?} in {paras:?}");
    }
}

#[test]
fn doc_survives_truncation_and_corruption() {
    for n in (0..DOC.len()).step_by(7) {
        let _ = word_viewer::to_rtf(&DOC[..n]);
    }
    for i in 0..DOC.len() {
        let mut d = DOC.to_vec();
        d[i] ^= 0xFF;
        let _ = word_viewer::to_rtf(&d);
    }
}

// ---- .docx features ----

/// Built by hand: German style ids (Word localizes them), a two-level
/// numbered list, a style that carries numbering (starting at IV), hidden
/// text, a tracked formatting change, moved text, and a table with column
/// widths, a merged cell, an empty cell and a two-paragraph cell.
const FEATURES: &[u8] = include_bytes!("fixtures/features.docx");

#[test]
fn docx_headings_lists_and_tables() {
    let rtf = word_viewer::to_rtf(FEATURES).unwrap();
    for want in [r"{\b\fs52 Gartenbericht}", r"{\b\fs40 Erster Abschnitt}", r"{\b\fs32 Kapitel Zwei}", r"\cellx2000", r"\cellx5000", r"\cellx9000"] {
        assert!(rtf.contains(want), "missing {want} in {rtf}");
    }
    let paras = word_viewer::read(FEATURES, "features.docx").unwrap();
    for p in ["1. Dig", "2. Plant", "2.a) Water daily", "2.b) Weed weekly", "3. Harvest", "IV. Styled item one", "V. Styled item two", "Visible text.", "NOW-PLAIN", "Stays.", "Plant Count Notes", "Tomato 12", "Spans two Line one Line two", "After the table."] {
        assert!(paras.iter().any(|x| x == p), "missing {p:?} in {paras:?}");
    }
    for gone in ["HIDDEN-TEXT", "MOVED-AWAY"] {
        assert!(!rtf.contains(gone), "{gone} shown");
    }
    assert!(!rtf.contains(r"{\b NOW-PLAIN"), "formatting from before a tracked change shown");
}

#[test]
fn docx_features_survive_truncation_and_corruption() {
    for n in 0..FEATURES.len() {
        let _ = word_viewer::to_rtf(&FEATURES[..n]);
    }
    for i in 0..FEATURES.len() {
        let mut d = FEATURES.to_vec();
        d[i] ^= 0xFF;
        let _ = word_viewer::to_rtf(&d);
    }
}

#[test]
fn table_and_list_floods_are_capped() {
    // a million empty cells in one row
    let mut xml = b"<w:tbl><w:tr>".to_vec();
    xml.extend(b"<w:tc></w:tc>".repeat(1_000_000));
    xml.extend(b"</w:tr></w:tbl>");
    let rtf = quick(|| word_viewer::to_rtf(&zip("word/document.xml", &xml, xml.len() as u32, 0))).unwrap();
    assert!(rtf.len() <= (32 << 20) + 1);
    // 200,000 list items at an out-of-range level of an undefined list
    let xml = br#"<w:p><w:pPr><w:numPr><w:ilvl w:val="99"/><w:numId w:val="7"/></w:numPr></w:pPr><w:r><w:t>x</w:t></w:r></w:p>"#.repeat(200_000);
    quick(|| word_viewer::to_rtf(&zip("word/document.xml", &xml, xml.len() as u32, 0))).unwrap();
}

// ---- every other format (PDF, RTF, ODT, EPUB, HTML, text) ----

fn read_quick(data: &[u8], name: &str) -> Result<Vec<String>, word_viewer::ReadError> {
    quick(|| word_viewer::read(data, name))
}

const FIXTURES: [(&str, &[u8]); 6] = [
    ("sample.pdf", include_bytes!("fixtures/sample.pdf")),
    ("sample.rtf", include_bytes!("fixtures/sample.rtf")),
    ("sample.odt", include_bytes!("fixtures/sample.odt")),
    ("sample.epub", include_bytes!("fixtures/sample.epub")),
    ("sample.html", include_bytes!("fixtures/sample.html")),
    ("sample.txt", include_bytes!("fixtures/sample.txt")),
];

#[test]
fn every_format_reads() {
    for (name, data) in FIXTURES {
        let p = word_viewer::read(data, name).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert!(p.join(" ").contains("quick") || p.join(" ").contains("The quick"), "{name}: {p:?}");
    }
}

#[test]
fn every_format_survives_truncation_and_corruption() {
    for (name, data) in FIXTURES {
        for n in 0..data.len() {
            let _ = word_viewer::read(&data[..n], name);
        }
        for i in 0..data.len() {
            let mut d = data.to_vec();
            d[i] ^= 0xFF;
            let _ = word_viewer::read(&d, name);
        }
    }
}

#[test]
fn pdf_decompression_bomb_is_capped() {
    // 300 MB of zeros behind one stream; the decode budget stops it
    let r = read_quick(include_bytes!("fixtures/bomb.pdf"), "bomb.pdf");
    assert!(r.is_err());
}

fn pdf(objs: &[&str]) -> Vec<u8> {
    let mut p = String::from("%PDF-1.4\n");
    for (i, o) in objs.iter().enumerate() {
        p.push_str(&format!("{} 0 obj\n{}\nendobj\n", i + 1, o));
    }
    p.push_str("trailer\n<< /Root 1 0 R >>\n%%EOF\n");
    p.into_bytes()
}

#[test]
fn pdf_structural_attacks() {
    let text = "<< /Length 30 >>\nstream\nBT /F1 12 Tf (Hello) Tj ET\nendstream";
    let font = "<< /Type /Font /Subtype /Type1 /Encoding /WinAnsiEncoding >>";
    let cases: Vec<Vec<u8>> = vec![
        // a page tree that contains itself
        pdf(&["<< /Type /Catalog /Pages 2 0 R >>", "<< /Type /Pages /Kids [2 0 R 2 0 R] >>"]),
        // nesting far past any sane depth
        format!("%PDF-1.4\n1 0 obj\n{}\nendobj\n", "[".repeat(200_000)).into_bytes(),
        format!("%PDF-1.4\n1 0 obj\n{}\nendobj\n", "<< /A ".repeat(100_000)).into_bytes(),
        // a stream length beyond the file and beyond usize
        pdf(&["<< /Type /Catalog /Pages 2 0 R >>", "<< /Type /Pages /Kids [3 0 R] >>", "<< /Type /Page /Contents 4 0 R >>", "<< /Length 99999999999999999999 >>\nstream\nBT ET"]),
        // an object stream claiming a billion objects
        pdf(&["<< /Type /ObjStm /N 1000000000 /First 5 /Length 9 >>\nstream\n1 0 2 0\nendstream"]),
        // fonts with gigantic width ranges and a CMap range over every code
        pdf(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] >>",
            "<< /Type /Page /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
            text,
            "<< /Type /Font /Subtype /Type0 /DescendantFonts [6 0 R] /ToUnicode 7 0 R >>",
            "<< /W [0 4294967295 500 0 [1 2 3]] /DW 1000 >>",
            "<< /Length 80 >>\nstream\nbegincodespacerange <0000> <FFFF> endcodespacerange beginbfrange <0000> <FFFFFFFF> <0041> endbfrange\nendstream",
        ]),
        // numbers before nested arrays: reference lookahead must not re-parse them
        // (this took exponential time before; found by fuzzing)
        format!("%PDF-1.4\n1 0 obj\n{}{}\nendobj\n", "[1 2 ".repeat(40), "]".repeat(40)).into_bytes(),
        // references that point at each other forever
        pdf(&["<< /Type /Catalog /Pages 2 0 R >>", "3 0 R", "2 0 R"]),
        // a million tiny pages
        {
            let kids: String = (0..1_000_000).map(|_| "3 0 R ").collect();
            pdf(&["<< /Type /Catalog /Pages 2 0 R >>", &format!("<< /Type /Pages /Kids [{kids}] >>"), "<< /Type /Page /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>", text, font])
        },
    ];
    for c in cases {
        let _ = read_quick(&c, "attack.pdf");
    }
}

#[test]
fn other_format_attacks() {
    let _ = read_quick(&[b"{\\rtf1 ".as_slice(), &b"{".repeat(1_000_000)].concat(), "a.rtf");
    let _ = read_quick(b"{\\rtf1 \\bin99999999999999999999 x}", "a.rtf");
    let _ = read_quick(&[b"<html>".as_slice(), &b"<".repeat(5_000_000)].concat(), "a.html");
    let _ = read_quick(&[b"<html><script>".as_slice(), &b"x".repeat(5_000_000)].concat(), "a.html");
    let _ = read_quick(&"&".repeat(5_000_000).into_bytes(), "a.html");
    // binary junk is not "text"
    let junk: Vec<u8> = (0..100_000u32).map(|i| (i.wrapping_mul(2654435761) >> 24) as u8).collect();
    assert!(read_quick(&junk, "junk.txt").is_err());
}

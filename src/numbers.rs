//! List numbers and bullets, shared by .doc and .docx.

/// `n` in number format `nfc`: 1/2 upper/lower Roman, 3/4 upper/lower letter,
/// 22 two digits; decimal otherwise.
pub fn number(n: i32, nfc: u8) -> String {
    match nfc {
        1 | 2 if (1..4000).contains(&n) => {
            let mut s = String::new();
            let mut n = n;
            for (v, r) in [(1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"), (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")] {
                while n >= v {
                    s.push_str(r);
                    n -= v;
                }
            }
            if nfc == 2 {
                s.make_ascii_lowercase();
            }
            s
        }
        // a, b, ... z, aa, bb, ...
        3 | 4 if (1..=26 * 40).contains(&n) => {
            let c = (b'A' + ((n - 1) % 26) as u8) as char;
            let s = c.to_string().repeat(((n - 1) / 26 + 1) as usize);
            if nfc == 4 {
                s.to_ascii_lowercase()
            } else {
                s
            }
        }
        22 => format!("{n:02}"),
        _ => n.to_string(),
    }
}

/// Bullets in symbol fonts (Symbol, Wingdings) are stored in the private use
/// area; show the usual ones as their Unicode look-alikes.
pub fn symbol(ch: u16) -> u32 {
    match ch {
        0xF000..=0xF0FF => match ch & 0xFF {
            0xA7 | 0x6E => 0x25AA, // small square
            0xD8 => 0x27A2,        // arrowhead
            0xFC => 0x2713,        // check mark
            0x76 => 0x2756,        // diamond
            _ => 0x2022,           // bullet
        },
        _ => ch as u32,
    }
}

/// A bullet level's character: as `symbol`, plus the Latin-1 characters that
/// Wingdings bullets are often stored as (shown in other fonts, they'd read
/// as letters).
pub fn bullet(ch: u16) -> u32 {
    match ch {
        0xA7 => 0x25AA, // small square
        0xD8 => 0x27A2, // arrowhead
        0xFC => 0x2713, // check mark
        _ => symbol(ch),
    }
}

/// RTF indent for a list item at `level` (0 to 8) whose label is `len`
/// characters: a hanging indent wide enough that the text after the label's
/// tab lines up.
pub fn indent(level: u8, len: usize) -> String {
    let hang = if len > 3 { 720 } else { 360 };
    format!("\\li{}\\fi-{hang} ", 360 * level.min(8) as u32 + hang)
}

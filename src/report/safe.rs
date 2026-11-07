//! Display-safe text (DEC-010 part 2): one function turns hostile bytes into
//! text that is safe to print and to embed.
//!
//! Valid UTF-8 passes through unchanged except for the escaped characters;
//! each byte of an invalid UTF-8 sequence is written `\xNN`. The result holds
//! no control character, no bidi control, no line or paragraph separator, no
//! raw `"` and no raw `\`. Because the backslash is always escaped, the
//! mapping is injective: the output can be read back to the exact input bytes.

/// The characters escaped as `\u{NNNN}` besides C1: the bidirectional controls,
/// the line and paragraph separators and the invisible format characters
/// (soft hyphen, zero-width characters, word joiner, BOM, deprecated format
/// controls, interlinear annotation, Unicode tag characters).
fn is_bidi_or_separator(c: char) -> bool {
    matches!(
        c,
        '\u{061C}'
            | '\u{200E}'
            | '\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2066}'..='\u{2069}'
            | '\u{2028}'
            | '\u{2029}'
            // Invisible and format characters that hide text from a reader.
            | '\u{00AD}'
            | '\u{200B}'..='\u{200D}'
            | '\u{2060}'
            | '\u{FEFF}'
            | '\u{206A}'..='\u{206F}'
            | '\u{FFF9}'..='\u{FFFB}'
            | '\u{E0000}'..='\u{E007F}'
    )
}

/// `value` as `digits` lowercase hex digits, most significant first.
fn push_hex(out: &mut String, value: u32, digits: u32) {
    let mut left = digits;
    while left > 0 {
        left = left.saturating_sub(1);
        let nibble = value.checked_shr(left.saturating_mul(4)).unwrap_or(0) % 16;
        out.push(char::from_digit(nibble, 16).unwrap_or('0'));
    }
}

fn push_escaped_byte(out: &mut String, byte: u8) {
    out.push_str("\\x");
    push_hex(out, u32::from(byte), 2);
}

fn push_char(out: &mut String, c: char) {
    match c {
        '\\' => out.push_str("\\\\"),
        '"' => out.push_str("\\\""),
        '\n' => out.push_str("\\n"),
        '\r' => out.push_str("\\r"),
        '\t' => out.push_str("\\t"),
        // The remaining C0 controls and DEL (all ASCII, so the cast is exact).
        '\u{0000}'..='\u{001F}' | '\u{007F}' => push_escaped_byte(out, u32::from(c) as u8),
        c if ('\u{0080}'..='\u{009F}').contains(&c) || is_bidi_or_separator(c) => {
            out.push_str("\\u{");
            // Four digits up to U+FFFF, more for the tag characters.
            push_hex(out, u32::from(c), if u32::from(c) > 0xFFFF { 5 } else { 4 });
            out.push('}');
        }
        c => out.push(c),
    }
}

/// Turn arbitrary bytes into display-safe text (DEC-010 part 2).
///
/// Total, linear in the input, and the output is at most four times the input
/// length (`\xNN` per invalid byte, `\u{NNNN}` for a two-byte C1 character).
pub fn safe_text(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    let mut rest = bytes;
    loop {
        match core::str::from_utf8(rest) {
            Ok(s) => {
                s.chars().for_each(|c| push_char(&mut out, c));
                return out;
            }
            Err(e) => {
                let valid = e.valid_up_to();
                if let Some(good) = rest.get(..valid).and_then(|g| core::str::from_utf8(g).ok()) {
                    good.chars().for_each(|c| push_char(&mut out, c));
                }
                // `None` means a sequence cut short by the end of the input:
                // every remaining byte is invalid.
                let bad = e.error_len().unwrap_or(rest.len().saturating_sub(valid));
                let end = valid.saturating_add(bad);
                for byte in rest.get(valid..end).unwrap_or_default() {
                    push_escaped_byte(&mut out, *byte);
                }
                match rest.get(end..) {
                    Some(tail) if !tail.is_empty() => rest = tail,
                    _ => return out,
                }
            }
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::arithmetic_side_effects
)]
mod tests {
    use super::*;

    #[test]
    fn safe_text_escapes_controls_c1_bidi_separators_quote_and_backslash() {
        // C0: the three named, the rest as \xNN.
        assert_eq!(safe_text(b"a\nb\rc\td"), "a\\nb\\rc\\td");
        assert_eq!(safe_text(&[0x00, 0x01, 0x1B, 0x1F]), "\\x00\\x01\\x1b\\x1f");
        assert_eq!(safe_text(&[0x7F]), "\\x7f");
        // Quote and backslash.
        assert_eq!(safe_text(b"say \"hi\"\\"), "say \\\"hi\\\"\\\\");
        // C1 as \u{NNNN}.
        for cp in 0x80u32..=0x9F {
            let s = char::from_u32(cp).unwrap().to_string();
            assert_eq!(safe_text(s.as_bytes()), format!("\\u{{{cp:04x}}}"));
        }
        // Bidi controls and the separators.
        let escaped = [
            0x061Cu32, 0x200E, 0x200F, 0x202A, 0x202B, 0x202C, 0x202D, 0x202E, 0x2066, 0x2067,
            0x2068, 0x2069, 0x2028, 0x2029,
            // Invisible and format characters, with the edges of each range.
            0x00AD, 0x200B, 0x200C, 0x200D, 0x2060, 0xFEFF, 0x206A, 0x206B, 0x206C, 0x206D, 0x206E,
            0x206F, 0xFFF9, 0xFFFA, 0xFFFB, 0xE0000, 0xE0001, 0xE0020, 0xE007E, 0xE007F,
        ];
        for cp in escaped {
            let s = char::from_u32(cp).unwrap().to_string();
            assert_eq!(
                safe_text(s.as_bytes()),
                format!("\\u{{{cp:04x}}}"),
                "{cp:#x}"
            );
        }
        let mixed = "a\u{202E}b\u{0085}c\u{2028}\u{7F}\"\\";
        let out = safe_text(mixed.as_bytes());
        assert!(out.chars().all(|c| !c.is_control()), "{out:?}");
        assert!(!out.contains('\u{202E}') && !out.contains('\u{2028}'));
        for b in 0u8..0x20 {
            assert!(safe_text(&[b]).chars().all(|c| !c.is_control()));
        }
    }

    #[test]
    fn safe_text_writes_invalid_utf8_bytes_as_hex_escapes() {
        assert_eq!(safe_text(&[0xFF]), "\\xff");
        assert_eq!(safe_text(&[b'a', 0x80, b'b']), "a\\x80b");
        // A truncated three-byte sequence at the end: both bytes.
        assert_eq!(safe_text(&[0xE2, 0x82]), "\\xe2\\x82");
        // A truncated sequence in the middle.
        assert_eq!(safe_text(&[0xE2, 0x82, b'a']), "\\xe2\\x82a");
        // An overlong NUL and a surrogate encoding are invalid.
        assert_eq!(safe_text(&[0xC0, 0x80]), "\\xc0\\x80");
        assert_eq!(safe_text(&[0xED, 0xA0, 0x80]), "\\xed\\xa0\\x80");
        // U+FFFD in the input is a valid character and is kept; an invalid
        // byte is not turned into it (two inputs never give one output).
        assert_eq!(safe_text("\u{FFFD}".as_bytes()), "\u{FFFD}");
        assert_ne!(safe_text(&[0xFF]), safe_text("\u{FFFD}".as_bytes()));
    }

    #[test]
    fn safe_text_keeps_valid_printable_unicode() {
        for s in [
            "photos/full-house.jpg",
            "caf\u{E9}",
            "\u{65E5}\u{672C}\u{8A9E}",
            "\u{1F600} emoji",
            "\u{FFFD}",
            "\u{00A0}nbsp",
            // Neighbours of the escaped ranges stay as they are.
            "\u{00AC}\u{00AE}\u{200A}\u{2010}\u{2061}\u{2062}\u{2063}\u{2064}",
            "\u{205F}\u{2065}\u{FEFE}\u{FF00}\u{FFF8}\u{FFFC}\u{FFFD}",
            "\u{DFFFF}\u{E0080}\u{E0100}\u{E01EF}",
            "",
        ] {
            assert_eq!(safe_text(s.as_bytes()), s);
        }
    }

    #[test]
    fn safe_text_is_injective_on_a_sampled_set() {
        // Every byte string over a small alphabet that mixes the interesting
        // classes, up to length 4: no two different inputs share an output.
        let alphabet: [&[u8]; 11] = [
            b"a",
            b"\\",
            b"\"",
            b"x",
            b"\n",
            &[0xFF],
            &[0x80],
            "\u{202E}".as_bytes(),
            "\u{FFFD}".as_bytes(),
            "\u{200B}".as_bytes(),
            "\u{E0041}".as_bytes(),
        ];
        let mut seen: std::collections::HashMap<String, Vec<u8>> = std::collections::HashMap::new();
        let mut level: Vec<Vec<u8>> = vec![Vec::new()];
        for _ in 0..4 {
            let mut next = Vec::new();
            for base in &level {
                for a in alphabet {
                    let mut v = base.clone();
                    v.extend_from_slice(a);
                    next.push(v);
                }
            }
            for input in &next {
                let out = safe_text(input);
                if let Some(prev) = seen.insert(out.clone(), input.clone()) {
                    assert_eq!(&prev, input, "two inputs give {out:?}");
                }
            }
            level = next;
        }
    }

    #[test]
    fn safe_text_output_bound_holds() {
        // Four bytes of output per input byte is the worst case: an invalid
        // byte, or a two-byte C1 character.
        let all_bytes: Vec<u8> = (0..=255u8).collect();
        let worst = [
            vec![0xFFu8; 1000],
            "\u{0085}".repeat(500).into_bytes(),
            "\u{202E}".repeat(333).into_bytes(),
            "\u{E0041}".repeat(250).into_bytes(),
            "\u{200B}".repeat(333).into_bytes(),
            vec![0u8; 1000],
            all_bytes,
        ];
        for input in worst {
            let out = safe_text(&input);
            assert!(
                out.len() <= input.len() * 4,
                "{} > 4 x {}",
                out.len(),
                input.len()
            );
        }
        assert_eq!(safe_text(&[0xFF; 1000]).len(), 4000);
        assert_eq!(safe_text(b""), "");
    }
}

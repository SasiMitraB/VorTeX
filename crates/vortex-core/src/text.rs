//! Column conversions between Rust strings and JavaScript (UTF-16) offsets.
//!
//! Core modules count columns in `char`s; the editor counts UTF-16 code
//! units. They differ only for characters outside the BMP (e.g. `𝛼`).

/// UTF-16 length of `s`.
pub fn utf16_len(s: &str) -> u32 {
    s.encode_utf16().count() as u32
}

/// Byte index in `line` of UTF-16 offset `col` (clamped to the line).
pub fn byte_at_utf16(line: &str, col: u32) -> usize {
    let mut units = 0u32;
    for (i, c) in line.char_indices() {
        if units >= col {
            return i;
        }
        units += c.len_utf16() as u32;
    }
    line.len()
}

/// `char` column → UTF-16 column on `line`.
pub fn char_col_to_utf16(line: &str, col: usize) -> u32 {
    line.chars().take(col).map(|c| c.len_utf16() as u32).sum()
}

/// UTF-16 column → `char` column on `line`.
pub fn utf16_to_char_col(line: &str, col: u32) -> usize {
    line[..byte_at_utf16(line, col)].chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn astral_characters_count_twice_in_utf16() {
        let line = "a𝛼b";
        assert_eq!(utf16_len(line), 4);
        assert_eq!(char_col_to_utf16(line, 2), 3);
        assert_eq!(utf16_to_char_col(line, 3), 2);
        assert_eq!(byte_at_utf16(line, 3), 5);
        assert_eq!(byte_at_utf16(line, 99), line.len());
    }
}

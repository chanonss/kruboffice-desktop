//! Thai word counting. Thai is written without spaces between words, so counting
//! whitespace-separated tokens makes a whole Thai sentence one word. A dictionary word breaker
//! (ICU4X, the same data the layout uses to wrap Thai lines) finds the words inside a token.

use icu_segmenter::WordSegmenter;
use icu_segmenter::options::WordBreakInvariantOptions;
use std::sync::OnceLock;

/// A Thai letter, vowel, tone mark or digit (U+0E01–U+0E5B).
pub(crate) fn is_thai(c: char) -> bool {
    matches!(u32::from(c), 0x0E01..=0x0E5B)
}

/// Words in one whitespace-free `token` that has Thai in it: the word-like segments the
/// dictionary word breaker finds that have a letter or digit. Never less than 1, so a token
/// still counts as one word if the breaker finds nothing in it.
pub(crate) fn count(token: &str) -> usize {
    static SEGMENTER: OnceLock<icu_segmenter::WordSegmenterBorrowed<'static>> = OnceLock::new();
    let seg = SEGMENTER.get_or_init(|| WordSegmenter::new_dictionary(WordBreakInvariantOptions::default()));
    let mut words = 0usize;
    let mut start = 0usize;
    for (end, kind) in seg.segment_str(token).iter_with_word_type() {
        let piece = token.get(start..end).unwrap_or("");
        start = end;
        // ๆ (mai yamok, U+0E46) repeats the word before it; it isn't a word of its own.
        if kind.is_word_like() && piece.chars().any(|c| c.is_alphanumeric() && c != '\u{0E46}') {
            words = words.saturating_add(1);
        }
    }
    words.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_thai_words_without_spaces() {
        // ภาษา | ไทย | ไม่มี | การ | เว้น | วรรค (the dictionary keeps ไม่มี whole)
        assert_eq!(count("ภาษาไทยไม่มีการเว้นวรรค"), 6);
        // ประเทศไทย | เป็น | ประเทศ | ที่ | สวยงาม
        assert_eq!(count("ประเทศไทยเป็นประเทศที่สวยงาม"), 5);
        assert_eq!(count("สวัสดี"), 1);
    }

    #[test]
    fn mixed_and_marks() {
        // Latin glued to Thai counts both parts.
        assert_eq!(count("Wordไทย"), 2);
        // Mai yamok repeats a word; it isn't one.
        assert_eq!(count("เด็กๆ"), 1);
        // Thai digits are a number, like 42.
        assert_eq!(count("๒๕๖๙"), 1);
    }
}

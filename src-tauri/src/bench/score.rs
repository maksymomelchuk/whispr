//! Transcript scoring: WER and CER against a reference.
//!
//! Normalization strips everything that is a formatting choice rather than a
//! recognition error — case, punctuation, and runs of whitespace — so that
//! "We've" and "weve" or "uk" Cyrillic case variants don't count as mistakes.
//! Numbers are deliberately NOT canonicalized: "1,250" vs "twelve hundred
//! fifty" are left as-is, which is why number-heavy clips are scored for the
//! transcript eyeball rather than treated as recognition errors.

pub fn normalized_words(text: &str) -> Vec<String> {
    let mut flattened = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            flattened.extend(ch.to_lowercase());
        } else if !is_apostrophe(ch) {
            // Apostrophes are dropped, not split on, so "We've" stays one token.
            flattened.push(' ');
        }
    }
    flattened.split_whitespace().map(str::to_string).collect()
}

fn is_apostrophe(ch: char) -> bool {
    matches!(ch, '\'' | '\u{2019}' | '\u{2018}' | '\u{02BC}')
}

pub fn word_error_rate(reference: &str, hypothesis: &str) -> f64 {
    let reference_words = normalized_words(reference);
    let hypothesis_words = normalized_words(hypothesis);
    error_rate(&reference_words, &hypothesis_words)
}

pub fn character_error_rate(reference: &str, hypothesis: &str) -> f64 {
    let reference_chars: Vec<char> = normalized_words(reference).join(" ").chars().collect();
    let hypothesis_chars: Vec<char> = normalized_words(hypothesis).join(" ").chars().collect();
    error_rate(&reference_chars, &hypothesis_chars)
}

fn error_rate<T: PartialEq>(reference: &[T], hypothesis: &[T]) -> f64 {
    if reference.is_empty() {
        return if hypothesis.is_empty() { 0.0 } else { 1.0 };
    }
    levenshtein(reference, hypothesis) as f64 / reference.len() as f64
}

fn levenshtein<T: PartialEq>(left: &[T], right: &[T]) -> usize {
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    let mut current = vec![0usize; right.len() + 1];
    for (i, left_item) in left.iter().enumerate() {
        current[0] = i + 1;
        for (j, right_item) in right.iter().enumerate() {
            let substitution_cost = if left_item == right_item { 0 } else { 1 };
            current[j + 1] = (previous[j + 1] + 1)
                .min(current[j] + 1)
                .min(previous[j] + substitution_cost);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_text_scores_zero() {
        assert_eq!(word_error_rate("hello world", "hello world"), 0.0);
        assert_eq!(character_error_rate("hello world", "hello world"), 0.0);
    }

    #[test]
    fn casing_and_punctuation_are_not_errors() {
        assert_eq!(word_error_rate("We've done it.", "weve done it"), 0.0);
    }

    #[test]
    fn one_wrong_word_in_four_is_quarter_wer() {
        assert_eq!(
            word_error_rate("the quick brown fox", "the quick brown dog"),
            0.25
        );
    }

    #[test]
    fn empty_hypothesis_against_nonempty_reference_is_total_error() {
        assert_eq!(word_error_rate("hello world", ""), 1.0);
    }

    #[test]
    fn cyrillic_case_is_normalized() {
        assert_eq!(word_error_rate("Привіт Світ", "привіт світ"), 0.0);
    }

    #[test]
    fn insertion_counts_against_reference_length() {
        assert_eq!(word_error_rate("one two", "one two three"), 0.5);
    }
}

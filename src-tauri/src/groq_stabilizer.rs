//! Longest-stable-prefix stabilizer for the Groq polling preview.
//!
//! Groq has no streaming endpoint, so the live preview is produced by a
//! polling worker that re-transcribes the trailing audio window every few
//! seconds. Each fresh poll can rewrite earlier words (Whisper is allowed
//! to revise its own output as it sees more context), which would make the
//! overlay flicker if emitted raw. This module stabilizes the emitted text
//! by tracking the longest word-prefix that has remained consistent across
//! the observed polls and only revising the tail.
//!
//! Tokenization is a plain ASCII/UTF-8 whitespace split. That is enough
//! for v1: Whisper emits transcripts with conventional spacing between
//! words, so word boundaries align across polls. The known limitation is
//! that punctuation sticks to adjacent words ("hello," vs "hello") — if a
//! later poll adds a comma after the last stable word, that word will look
//! "different" and the stable prefix will shrink by one. We accept that
//! flicker in exchange for not having to ship a real word tokenizer here.

#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct Stabilizer {
    stable_prefix: Vec<String>,
}

#[allow(dead_code)]
impl Stabilizer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ingest a new poll's raw transcript. Updates the internal stable
    /// prefix and returns the partial that should be emitted to the
    /// overlay for this poll.
    pub fn ingest(&mut self, poll_result: &str) -> String {
        let new_words: Vec<String> = poll_result
            .split_whitespace()
            .map(str::to_string)
            .collect();

        let common = self
            .stable_prefix
            .iter()
            .zip(new_words.iter())
            .take_while(|(a, b)| a == b)
            .count();

        // If the previous stable prefix matched the head of the new poll
        // in full, every new word is now confirmed against the prior
        // observation — adopt the new poll as the next stable prefix.
        // Otherwise the polls disagreed mid-prefix; keep only the agreed
        // portion.
        if common == self.stable_prefix.len() {
            self.stable_prefix = new_words.clone();
        } else {
            self.stable_prefix.truncate(common);
        }

        // Since stable_prefix is now a prefix of new_words, emitting
        // stable_prefix + remaining new_words is equivalent to rendering
        // the new poll's word list.
        new_words.join(" ")
    }

    #[cfg(test)]
    pub fn stable_prefix(&self) -> &[String] {
        &self.stable_prefix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(s: &str) -> Vec<String> {
        s.split_whitespace().map(str::to_string).collect()
    }

    #[test]
    fn first_poll_is_emitted_verbatim_and_seeds_stable_prefix() {
        let mut s = Stabilizer::new();
        let out = s.ingest("the quick brown fox");
        assert_eq!(out, "the quick brown fox");
        assert_eq!(s.stable_prefix(), words("the quick brown fox").as_slice());
    }

    #[test]
    fn identical_repeat_leaves_stable_prefix_untouched() {
        let mut s = Stabilizer::new();
        s.ingest("the quick brown fox");
        let out = s.ingest("the quick brown fox");
        assert_eq!(out, "the quick brown fox");
        assert_eq!(s.stable_prefix(), words("the quick brown fox").as_slice());
    }

    #[test]
    fn appended_word_grows_the_stable_prefix() {
        let mut s = Stabilizer::new();
        s.ingest("the quick brown fox");
        let out = s.ingest("the quick brown fox jumps");
        assert_eq!(out, "the quick brown fox jumps");
        assert_eq!(
            s.stable_prefix(),
            words("the quick brown fox jumps").as_slice()
        );
    }

    #[test]
    fn mid_word_change_shrinks_stable_prefix_to_unchanged_head() {
        let mut s = Stabilizer::new();
        s.ingest("the quick brown fox");
        let out = s.ingest("the quick red fox");
        assert_eq!(out, "the quick red fox");
        assert_eq!(s.stable_prefix(), words("the quick").as_slice());
    }

    #[test]
    fn dropped_tail_shrinks_stable_prefix_to_surviving_common_prefix() {
        let mut s = Stabilizer::new();
        s.ingest("the quick brown fox");
        let out = s.ingest("the quick");
        assert_eq!(out, "the quick");
        assert_eq!(s.stable_prefix(), words("the quick").as_slice());
    }
}

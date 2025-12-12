// Not copy to prevent logic errors
#[derive(Clone)]
pub struct ParseCursor<'a> {
    data: &'a str,
    cursor_byte_len: usize,
}

#[derive(Debug)]
pub struct Failed;

use stable_string_patterns_method::{Searchable, StrPatternExt};

impl<'a> ParseCursor<'a> {
    pub fn new(data: &'a str) -> Self {
        Self {
            data,
            cursor_byte_len: 0,
        }
    }

    pub fn data(&self) -> &'a str {
        self.data
    }

    #[cfg(not(feature = "use-unsafe"))]
    pub fn cursor(&self) -> &'a str {
        &self.data[..self.cursor_byte_len]
    }

    #[cfg(feature = "use-unsafe")]
    pub fn cursor(&self) -> &'a str {
        debug_assert!(self.data.is_char_boundary(self.cursor_byte_len));
        unsafe { self.data.get_unchecked(..self.cursor_byte_len) }
    }

    pub fn len(&self) -> usize {
        self.cursor().len()
    }

    #[cfg(not(feature = "use-unsafe"))]
    pub fn remainder(&self) -> &'a str {
        &self.data[self.cursor_byte_len..]
    }

    #[cfg(feature = "use-unsafe")]
    pub fn remainder(&self) -> &'a str {
        debug_assert!(self.data.is_char_boundary(self.cursor_byte_len));
        unsafe { self.data.get_unchecked(self.cursor_byte_len..) }
    }

    fn grow(&mut self, grow_by: usize) {
        self.cursor_byte_len += grow_by;
    }

    fn set_new_rem(&mut self, new_rem: &str) -> usize {
        debug_assert!(self.remainder().ends_with(new_rem));
        let old_len = self.len();
        let new_rem_len = new_rem.len();
        let new_len = self.data.len() - new_rem_len;
        self.cursor_byte_len = new_len;
        new_len - old_len
    }

    #[cfg(test)]
    fn check_invariants(&self) {
        assert!(self.data.is_char_boundary(self.cursor_byte_len));
    }

    pub fn reset(&mut self) {
        self.cursor_byte_len = 0;
    }

    pub fn consume(&mut self) -> &'a str {
        let res = self.cursor();
        let rem = self.remainder();
        *self = Self::new(rem);
        res
    }

    pub fn advance_until(&mut self, s: impl Searchable) -> Result<(), Failed> {
        let grow_by = self.remainder().find_(s).ok_or(Failed)?;
        self.grow(grow_by);
        Ok(())
    }

    pub fn advance_until_last(&mut self, s: impl Searchable) -> Result<(), Failed> {
        let grow_by = self.remainder().rfind_(s).ok_or(Failed)?;
        self.grow(grow_by);
        Ok(())
    }

    pub fn advance_once(&mut self, s: impl Searchable) -> Result<(), Failed> {
        let new_rem = self.remainder().strip_prefix_(s).ok_or(Failed)?;
        self.set_new_rem(new_rem);
        Ok(())
    }

    pub fn advance_while(&mut self, s: impl Searchable) -> usize {
        let new_rem = self.remainder().trim_start_matches_(s);
        self.set_new_rem(new_rem)
    }

    pub fn advance_to_become_one(&mut self, s: impl Searchable) -> Result<usize, Failed> {
        let mut candidate = self.clone();
        candidate.reset();
        candidate.advance_once(s)?;
        if candidate.len() < self.len() {
            return Err(Failed);
        }
        let grow = candidate.len() - self.len();
        *self = candidate;
        Ok(grow)
    }

    pub fn advance_to_match_repeated(&mut self, s: impl Searchable) -> Result<usize, Failed> {
        let mut candidate = self.clone();
        candidate.reset();
        candidate.advance_while(s);
        if candidate.len() < self.len() {
            return Err(Failed);
        }
        let grow = candidate.len() - self.len();
        *self = candidate;
        Ok(grow)
    }

    pub fn skip_until(&mut self, s: impl Searchable) -> Result<(), Failed> {
        self.advance_until(s)?;
        self.consume();
        Ok(())
    }

    pub fn skip_until_last(&mut self, s: impl Searchable) -> Result<(), Failed> {
        self.advance_until_last(s)?;
        self.consume();
        Ok(())
    }

    pub fn skip_once(&mut self, s: impl Searchable) -> Result<(), Failed> {
        self.advance_once(s)?;
        self.consume();
        Ok(())
    }

    pub fn skip_while(&mut self, s: impl Searchable) {
        self.advance_while(s);
        self.consume();
    }
}

#[cfg(test)]
mod tests {
    use super::ParseCursor;

    static S: &str = "Les mélèzes en fleurs.";

    #[test]
    fn test_advance_char() {
        let mut s = ParseCursor::new(S);
        s.advance_until('n').unwrap();

        s.check_invariants();
        assert_eq!(s.cursor(), "Les mélèzes e");
    }

    #[test]
    fn test_advance_str() {
        let mut s = ParseCursor::new(S);
        s.advance_until("en").unwrap();

        s.check_invariants();
        assert_eq!(s.cursor(), "Les mélèzes ");
    }

    #[test]
    fn test_advance_fn() {
        let mut s = ParseCursor::new(S);
        s.advance_until(|c| c == 'f').unwrap();

        s.check_invariants();
        assert_eq!(s.cursor(), "Les mélèzes en ");
    }

    #[test]
    fn test_advance_while_char() {
        let mut s = ParseCursor::new("aaabbbccc");
        s.advance_while('a');

        s.check_invariants();
        assert_eq!(s.cursor(), "aaa");
        assert_eq!(s.remainder(), "bbbccc");
    }

    #[test]
    fn test_advance_while_char_no_match() {
        let mut s = ParseCursor::new("bbbccc");
        s.advance_while('a');

        s.check_invariants();
        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), "bbbccc");
    }

    #[test]
    fn test_advance_while_char_all_match() {
        let mut s = ParseCursor::new("aaaa");
        s.advance_while('a');

        s.check_invariants();
        assert_eq!(s.cursor(), "aaaa");
        assert_eq!(s.remainder(), "");
    }

    #[test]
    fn test_advance_while_str() {
        let mut s = ParseCursor::new("Les mélèzes en fleurs.");
        s.advance_while("Les ");

        s.check_invariants();
        assert_eq!(s.cursor(), "Les ");
        assert_eq!(s.remainder(), "mélèzes en fleurs.");
    }

    #[test]
    fn test_advance_while_fn_whitespace() {
        let mut s = ParseCursor::new("   \t\n  hello world");
        s.advance_while(|c: char| c.is_whitespace());

        s.check_invariants();
        assert_eq!(s.cursor(), "   \t\n  ");
        assert_eq!(s.remainder(), "hello world");
    }

    #[test]
    fn test_advance_while_fn_alphabetic() {
        let mut s = ParseCursor::new("abcDEF123");
        s.advance_while(|c: char| c.is_alphabetic());

        s.check_invariants();
        assert_eq!(s.cursor(), "abcDEF");
        assert_eq!(s.remainder(), "123");
    }

    #[test]
    fn test_advance_while_fn_digits() {
        let mut s = ParseCursor::new("12345abc");
        s.advance_while(|c: char| c.is_numeric());

        s.check_invariants();
        assert_eq!(s.cursor(), "12345");
        assert_eq!(s.remainder(), "abc");
    }

    #[test]
    fn test_advance_while_unicode() {
        let mut s = ParseCursor::new("éééabc");
        s.advance_while('é');

        s.check_invariants();
        assert_eq!(s.cursor(), "ééé");
        assert_eq!(s.remainder(), "abc");
    }

    #[test]
    fn test_advance_while_multiple_calls() {
        let mut s = ParseCursor::new("aaabbbccc");

        s.advance_while('a');
        s.check_invariants();
        assert_eq!(s.cursor(), "aaa");
        assert_eq!(s.remainder(), "bbbccc");

        s.advance_while('b');
        s.check_invariants();
        assert_eq!(s.cursor(), "aaabbb");
        assert_eq!(s.remainder(), "ccc");

        s.advance_while('c');
        s.check_invariants();
        assert_eq!(s.cursor(), "aaabbbccc");
        assert_eq!(s.remainder(), "");
    }

    #[test]
    fn test_advance_while_after_advance_until() {
        let mut s = ParseCursor::new("   hello   world");

        // Skip initial whitespace
        s.advance_while(|c: char| c.is_whitespace());
        assert_eq!(s.cursor(), "   ");

        // Advance until space
        s.advance_until(' ').unwrap();
        assert_eq!(s.cursor(), "   hello");

        // Skip spaces again
        s.advance_while(' ');
        assert_eq!(s.cursor(), "   hello   ");
        assert_eq!(s.remainder(), "world");
    }

    #[test]
    fn test_advance_while_empty_remainder() {
        let mut s = ParseCursor::new("abc");
        s.advance_while(|c: char| c.is_alphabetic());

        assert_eq!(s.cursor(), "abc");
        assert_eq!(s.remainder(), "");

        // Advance while on empty remainder should do nothing
        s.advance_while('x');
        s.check_invariants();
        assert_eq!(s.cursor(), "abc");
        assert_eq!(s.remainder(), "");
    }

    #[test]
    fn test_advance_while_char_len_tracking() {
        let mut s = ParseCursor::new("éééabc");

        // char_len should be recalculated after advance_while
        s.advance_while('é');
        assert_eq!(s.len(), 6); // 3 chars, 6 bytes

        s.advance_while(|c: char| c.is_alphabetic());
    }

    // ========== Tests for advance_once ==========

    #[test]
    fn test_advance_once_char() {
        let mut s = ParseCursor::new("abc");
        s.advance_once('a').unwrap();

        s.check_invariants();
        assert_eq!(s.cursor(), "a");
        assert_eq!(s.remainder(), "bc");
    }

    #[test]
    fn test_advance_once_char_unicode() {
        let mut s = ParseCursor::new("éabc");
        s.advance_once('é').unwrap();

        s.check_invariants();
        assert_eq!(s.cursor(), "é");
        assert_eq!(s.remainder(), "abc");
    }

    #[test]
    fn test_advance_once_str() {
        let mut s = ParseCursor::new("hello world");
        s.advance_once("hello").unwrap();

        s.check_invariants();
        assert_eq!(s.cursor(), "hello");
        assert_eq!(s.remainder(), " world");
    }

    #[test]
    fn test_advance_once_str_multichar() {
        let mut s = ParseCursor::new("Les mélèzes");
        s.advance_once("Les ").unwrap();

        s.check_invariants();
        assert_eq!(s.cursor(), "Les ");
        assert_eq!(s.remainder(), "mélèzes");
    }

    #[test]
    fn test_advance_once_fn() {
        let mut s = ParseCursor::new("abc123");
        s.advance_once(|c: char| c.is_alphabetic()).unwrap();

        s.check_invariants();
        assert_eq!(s.cursor(), "a");
        assert_eq!(s.remainder(), "bc123");
    }

    #[test]
    fn test_advance_once_char_mismatch() {
        let mut s = ParseCursor::new("abc");
        let result = s.advance_once('x');

        assert!(result.is_err());
        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), "abc");
    }

    #[test]
    fn test_advance_once_str_mismatch() {
        let mut s = ParseCursor::new("hello world");
        let result = s.advance_once("goodbye");

        assert!(result.is_err());
        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), "hello world");
    }

    #[test]
    fn test_advance_once_fn_mismatch() {
        let mut s = ParseCursor::new("123abc");
        let result = s.advance_once(|c: char| c.is_alphabetic());

        assert!(result.is_err());
        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), "123abc");
    }

    #[test]
    fn test_advance_once_empty_string() {
        let mut s = ParseCursor::new("");
        let result = s.advance_once('a');

        assert!(result.is_err());
        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), "");
    }

    #[test]
    fn test_advance_once_multiple_calls() {
        let mut s = ParseCursor::new("abc");

        s.advance_once('a').unwrap();
        assert_eq!(s.cursor(), "a");

        s.advance_once('b').unwrap();
        assert_eq!(s.cursor(), "ab");

        s.advance_once('c').unwrap();
        assert_eq!(s.cursor(), "abc");
        assert_eq!(s.remainder(), "");
    }

    // ========== Tests for skip_until ==========

    #[test]
    fn test_skip_until_char() {
        let mut s = ParseCursor::new("hello world");
        s.skip_until('w').unwrap();

        s.check_invariants();
        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), "world");
    }

    #[test]
    fn test_skip_until_str() {
        let mut s = ParseCursor::new("hello world foo bar");
        s.skip_until("foo").unwrap();

        s.check_invariants();
        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), "foo bar");
    }

    #[test]
    fn test_skip_until_fn() {
        let mut s = ParseCursor::new("abc123def");
        s.skip_until(|c: char| c.is_numeric()).unwrap();

        s.check_invariants();
        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), "123def");
    }

    #[test]
    fn test_skip_until_not_found() {
        let mut s = ParseCursor::new("hello world");
        let result = s.skip_until('z');

        assert!(result.is_err());
    }

    #[test]
    fn test_skip_until_unicode() {
        let mut s = ParseCursor::new("Les mélèzes en fleurs");
        s.skip_until('è').unwrap();

        s.check_invariants();
        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), "èzes en fleurs");
    }

    // ========== Tests for skip_once ==========

    #[test]
    fn test_skip_once_char() {
        let mut s = ParseCursor::new("abc");
        s.skip_once('a').unwrap();

        s.check_invariants();
        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), "bc");
    }

    #[test]
    fn test_skip_once_str() {
        let mut s = ParseCursor::new("hello world");
        s.skip_once("hello").unwrap();

        s.check_invariants();
        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), " world");
    }

    #[test]
    fn test_skip_once_fn() {
        let mut s = ParseCursor::new("abc123");
        s.skip_once(|c: char| c.is_alphabetic()).unwrap();

        s.check_invariants();
        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), "bc123");
    }

    #[test]
    fn test_skip_once_mismatch() {
        let mut s = ParseCursor::new("abc");
        let result = s.skip_once('x');

        assert!(result.is_err());
        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), "abc");
    }

    #[test]
    fn test_skip_once_multiple_calls() {
        let mut s = ParseCursor::new("abc123");

        s.skip_once('a').unwrap();
        assert_eq!(s.remainder(), "bc123");

        s.skip_once('b').unwrap();
        assert_eq!(s.remainder(), "c123");

        s.skip_once('c').unwrap();
        assert_eq!(s.remainder(), "123");
    }

    // ========== Tests for skip_while ==========

    #[test]
    fn test_skip_while_char() {
        let mut s = ParseCursor::new("aaabbb");
        s.skip_while('a');

        s.check_invariants();
        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), "bbb");
    }

    #[test]
    fn test_skip_while_str() {
        let mut s = ParseCursor::new("Les Les mélèzes");
        s.skip_while("Les ");

        s.check_invariants();
        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), "mélèzes");
    }

    #[test]
    fn test_skip_while_fn_whitespace() {
        let mut s = ParseCursor::new("   \t\nhello");
        s.skip_while(|c: char| c.is_whitespace());

        s.check_invariants();
        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), "hello");
    }

    #[test]
    fn test_skip_while_no_match() {
        let mut s = ParseCursor::new("abc");
        s.skip_while('x');

        s.check_invariants();
        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), "abc");
    }

    #[test]
    fn test_skip_while_all_match() {
        let mut s = ParseCursor::new("aaaa");
        s.skip_while('a');

        s.check_invariants();
        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), "");
    }

    // ========== Tests for consume ==========

    #[test]
    fn test_consume_basic() {
        let mut s = ParseCursor::new("hello world");
        s.advance_until(' ').unwrap();

        let consumed = s.consume();
        assert_eq!(consumed, "hello");
        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), " world");
    }

    #[test]
    fn test_consume_empty_cursor() {
        let mut s = ParseCursor::new("hello");

        let consumed = s.consume();
        assert_eq!(consumed, "");
        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), "hello");
    }

    #[test]
    fn test_consume_full_string() {
        let mut s = ParseCursor::new("hello");
        s.advance_while(|c: char| c.is_alphabetic());

        let consumed = s.consume();
        assert_eq!(consumed, "hello");
        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), "");
    }

    #[test]
    fn test_consume_multiple_times() {
        let mut s = ParseCursor::new("one two three");

        s.advance_until(' ').unwrap();
        let first = s.consume();
        assert_eq!(first, "one");

        s.skip_once(' ').unwrap();
        s.advance_until(' ').unwrap();
        let second = s.consume();
        assert_eq!(second, "two");

        s.skip_once(' ').unwrap();
        s.advance_while(|c: char| c.is_alphabetic());
        let third = s.consume();
        assert_eq!(third, "three");

        assert_eq!(s.remainder(), "");
    }

    #[test]
    fn test_consume_with_unicode() {
        let mut s = ParseCursor::new("Les mélèzes");
        s.advance_until(' ').unwrap();

        let consumed = s.consume();
        assert_eq!(consumed, "Les");
        assert_eq!(s.remainder(), " mélèzes");
    }

    // ========== Tests for reset ==========

    #[test]
    fn test_reset_basic() {
        let mut s = ParseCursor::new("hello world");
        s.advance_until(' ').unwrap();

        assert_eq!(s.cursor(), "hello");

        s.reset();
        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), "hello world");
    }

    #[test]
    fn test_reset_after_consume() {
        let mut s = ParseCursor::new("hello world");
        s.advance_until(' ').unwrap();
        s.consume();

        assert_eq!(s.data(), " world");

        s.reset();
        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), " world");
    }

    #[test]
    fn test_reset_empty() {
        let mut s = ParseCursor::new("hello");
        s.reset();

        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), "hello");
    }

    #[test]
    fn test_reset_at_end() {
        let mut s = ParseCursor::new("hello");
        s.advance_while(|c: char| c.is_alphabetic());

        assert_eq!(s.cursor(), "hello");

        s.reset();
        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), "hello");
    }

    // ========== Tests for data ==========

    #[test]
    fn test_data_method() {
        let s = ParseCursor::new("hello world");
        assert_eq!(s.data(), "hello world");
    }

    #[test]
    fn test_data_unchanged_after_advance() {
        let mut s = ParseCursor::new("hello world");
        s.advance_until(' ').unwrap();

        assert_eq!(s.data(), "hello world");
    }

    #[test]
    fn test_data_changed_after_consume() {
        let mut s = ParseCursor::new("hello world");
        s.advance_until(' ').unwrap();
        s.consume();

        assert_eq!(s.data(), " world");
    }

    // ========== Integration tests ==========

    #[test]
    fn test_parse_key_value_pairs() {
        let mut s = ParseCursor::new("key1=value1;key2=value2;key3=value3");

        // Parse first pair
        s.advance_until('=').unwrap();
        let key1 = s.consume();
        assert_eq!(key1, "key1");

        s.skip_once('=').unwrap();
        s.advance_until(';').unwrap();
        let val1 = s.consume();
        assert_eq!(val1, "value1");

        s.skip_once(';').unwrap();

        // Parse second pair
        s.advance_until('=').unwrap();
        let key2 = s.consume();
        assert_eq!(key2, "key2");

        s.skip_once('=').unwrap();
        s.advance_until(';').unwrap();
        let val2 = s.consume();
        assert_eq!(val2, "value2");

        s.skip_once(';').unwrap();

        // Parse third pair
        s.advance_until('=').unwrap();
        let key3 = s.consume();
        assert_eq!(key3, "key3");

        s.skip_once('=').unwrap();
        s.advance_while(|c: char| c.is_alphanumeric());
        let val3 = s.consume();
        assert_eq!(val3, "value3");
    }

    #[test]
    fn test_parse_whitespace_separated_tokens() {
        let mut s = ParseCursor::new("  token1   token2   token3  ");
        let mut tokens = Vec::new();

        while !s.remainder().is_empty() {
            s.skip_while(|c: char| c.is_whitespace());
            if s.remainder().is_empty() {
                break;
            }
            s.advance_while(|c: char| !c.is_whitespace());
            tokens.push(s.consume());
        }

        assert_eq!(tokens, vec!["token1", "token2", "token3"]);
    }

    #[test]
    fn test_reset_and_reparse() {
        let mut s = ParseCursor::new("test data");

        s.advance_until(' ').unwrap();
        assert_eq!(s.cursor(), "test");

        s.reset();
        s.advance_while(|c: char| c.is_alphabetic());
        assert_eq!(s.cursor(), "test");
    }

    #[test]
    fn test_failed_advance_until_at_end() {
        let mut s = ParseCursor::new("hello");
        let result = s.advance_until('z');

        assert!(result.is_err());
        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), "hello");
    }

    #[test]
    fn test_advance_until_empty_string() {
        let mut s = ParseCursor::new("");
        let result = s.advance_until('a');

        assert!(result.is_err());
        assert_eq!(s.cursor(), "");
        assert_eq!(s.remainder(), "");
    }

    #[test]
    fn test_complex_unicode_parsing() {
        let mut s = ParseCursor::new("Les mélèzes 🌲 en fleurs 🌸");

        s.advance_until('🌲').unwrap();
        let part1 = s.consume();
        assert_eq!(part1, "Les mélèzes ");

        s.skip_once('🌲').unwrap();
        s.skip_while(' ');

        s.advance_until('🌸').unwrap();
        let part2 = s.consume();
        assert_eq!(part2, "en fleurs ");
    }
}

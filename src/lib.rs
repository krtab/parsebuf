// Not copy to prevent logic errors
#[derive(Clone, Debug)]
pub struct ParseCursor<'a> {
    data: &'a str,
    cursor_range: Range<usize>,
}

#[derive(Debug)]
pub struct Failed;

use std::ops::Range;

use stable_string_patterns_method::{IntoSearchable, Searchable, StrPatternExt};

#[derive(Debug, Clone, Copy)]
enum Direction {
    Forward,
    Backward,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PatternLoc {
    FirstExcluded,
    FirstIncluded,
    BeginningMany,
    BeginningOnce,
    LastExcluded,
    EndOfLast,
    StartOfSuffixMany,
}

fn find_directional_offset(
    haystack: &str,
    pattern: impl Searchable,
    loc: PatternLoc,
    direction: Direction,
) -> Option<usize> {
    let from_start_offset_to_end_offset = |offset_from_beg| haystack.len() - offset_from_beg;
    let from_end_offset_to_start_offset = |offset_from_end| haystack.len() - offset_from_end;
    let offset_of_sub_end = |(offset_of_sub, sub): (usize, &str)| offset_of_sub + sub.len();
    match (loc, direction) {
        (PatternLoc::FirstExcluded, Direction::Forward) => haystack.find_(pattern),
        (PatternLoc::FirstExcluded, Direction::Backward) => haystack
            .rmatch_indices_(pattern)
            .next()
            .map(offset_of_sub_end)
            .map(from_start_offset_to_end_offset),
        (PatternLoc::FirstIncluded, Direction::Forward) => haystack
            .match_indices_(pattern)
            .next()
            .map(offset_of_sub_end),
        (PatternLoc::FirstIncluded, Direction::Backward) => haystack
            .rfind_(pattern)
            .map(from_start_offset_to_end_offset),
        (PatternLoc::BeginningMany, Direction::Forward) => {
            let rem = haystack.trim_start_matches_(pattern);
            Some(from_end_offset_to_start_offset(rem.len()))
        }
        (PatternLoc::BeginningMany, Direction::Backward) => {
            let rem = haystack.trim_end_matches_(pattern);
            Some(from_start_offset_to_end_offset(rem.len()))
        }
        (PatternLoc::BeginningOnce, Direction::Forward) => {
            let rem = haystack.strip_prefix_(pattern)?;
            Some(from_end_offset_to_start_offset(rem.len()))
        }
        (PatternLoc::BeginningOnce, Direction::Backward) => {
            let rem = haystack.strip_suffix_(pattern)?;
            Some(from_start_offset_to_end_offset(rem.len()))
        }
        (PatternLoc::LastExcluded, Direction::Forward) => haystack.rfind_(pattern),
        (PatternLoc::LastExcluded, Direction::Backward) => haystack
            .rmatch_indices_(pattern)
            .next()
            .map(offset_of_sub_end)
            .map(from_start_offset_to_end_offset),
        (PatternLoc::EndOfLast, Direction::Forward) => haystack
            .rmatch_indices_(pattern)
            .next()
            .map(offset_of_sub_end),
        (PatternLoc::EndOfLast, Direction::Backward) => {
            haystack.find_(pattern).map(from_start_offset_to_end_offset)
        }
        (PatternLoc::StartOfSuffixMany, Direction::Forward) => {
            Some(haystack.trim_end_matches_(pattern).len())
        }
        (PatternLoc::StartOfSuffixMany, Direction::Backward) => {
            Some(haystack.trim_start_matches_(pattern).len())
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum InwardStrategy {
    CursorOnly,
    WholeData,
}

pub enum FallBack {
    ToTheEnd,
    StayAtBeginning,
}

impl<'a> ParseCursor<'a> {
    pub fn new_empty_start(data: &'a str) -> Self {
        Self {
            data,
            cursor_range: Range { start: 0, end: 0 },
        }
    }

    pub fn new_empty_end(data: &'a str) -> Self {
        Self {
            data,
            cursor_range: Range {
                start: data.len(),
                end: data.len(),
            },
        }
    }

    pub fn new_full(data: &'a str) -> Self {
        Self {
            data,
            cursor_range: Range {
                start: 0,
                end: data.len(),
            },
        }
    }

    pub fn front_to_back(&mut self) {
        self.cursor_range.end = self.cursor_range.start;
    }

    pub fn back_to_front(&mut self) {
        self.cursor_range.start = self.cursor_range.end;
    }

    pub fn data(&self) -> &'a str {
        self.data
    }

    fn cursor_range(&self) -> Range<usize> {
        self.cursor_range.clone()
    }

    #[cfg(not(feature = "use-unsafe"))]
    pub fn cursor(&self) -> &'a str {
        &self.data[self.cursor_range()]
    }

    #[cfg(feature = "use-unsafe")]
    pub fn cursor(&self) -> &'a str {
        #[cfg(debug_assertions)]
        self.check_invariants();
        unsafe { self.data.get_unchecked(self.cursor_range()) }
    }

    #[cfg(any(test, feature = "use-unsafe"))]
    fn check_invariants(&self) {
        assert!(self.cursor_range.start <= self.cursor_range.end);
        assert!(self.data.get(self.cursor_range.clone()).is_some());
    }

    #[cfg(not(feature = "use-unsafe"))]
    pub fn back_rem(&self) -> &'a str {
        &self.data[..self.cursor_range.start]
    }

    #[cfg(feature = "use-unsafe")]
    pub fn back_rem(&self) -> &'a str {
        #[cfg(debug_assertions)]
        self.check_invariants();
        unsafe { self.data.get_unchecked(..self.cursor_range.start) }
    }

    #[cfg(not(feature = "use-unsafe"))]
    pub fn front_rem(&self) -> &'a str {
        &self.data[self.cursor_range.end..]
    }

    #[cfg(feature = "use-unsafe")]
    pub fn front_rem(&self) -> &'a str {
        #[cfg(debug_assertions)]
        self.check_invariants();
        unsafe { self.data.get_unchecked(self.cursor_range.end..) }
    }

    #[cfg(not(feature = "use-unsafe"))]
    pub fn all_but_front_rem(&self) -> &'a str {
        &self.data[..self.cursor_range.end]
    }

    #[cfg(feature = "use-unsafe")]
    pub fn all_but_front_rem(&self) -> &'a str {
        #[cfg(debug_assertions)]
        self.check_invariants();
        unsafe { self.data.get_unchecked(..self.cursor_range.end) }
    }

    #[cfg(not(feature = "use-unsafe"))]
    pub fn all_but_back_rem(&self) -> &'a str {
        &self.data[self.cursor_range.start..]
    }

    #[cfg(feature = "use-unsafe")]
    pub fn all_but_back_rem(&self) -> &'a str {
        #[cfg(debug_assertions)]
        self.check_invariants();
        unsafe { self.data.get_unchecked(self.cursor_range.start..) }
    }

    pub fn split(&self) -> (&'a str, &'a str, &'a str) {
        (self.back_rem(), self.cursor(), self.front_rem())
    }

    pub fn extract(&self) -> (&'a str, Self, &'a str) {
        (
            self.back_rem(),
            Self::new_full(self.cursor()),
            self.front_rem(),
        )
    }

    pub fn snap(&self) -> Self {
        self.extract().1
    }

    fn move_front_forward(&mut self, by: usize) {
        self.cursor_range.end += by;
    }

    fn move_front_backward(&mut self, by: usize, inward_strategy: InwardStrategy) {
        if self.cursor().len() >= by {
            self.cursor_range.end -= by;
        } else {
            #[cfg(debug_assertions)]
            if let InwardStrategy::CursorOnly = inward_strategy {
                panic!("Cannot move past the other end of the cursor!")
            }
            self.cursor_range.end -= by;
            self.back_to_front();
        }
    }

    fn move_back_backward(&mut self, by: usize) {
        self.cursor_range.start -= by;
    }

    fn move_back_forward(&mut self, by: usize, inward_strategy: InwardStrategy) {
        if self.cursor().len() >= by {
            self.cursor_range.start += by;
        } else {
            #[cfg(debug_assertions)]
            if let InwardStrategy::CursorOnly = inward_strategy {
                panic!("Cannot move past the other end of the cursor!")
            }
            self.cursor_range.start += by;
            self.front_to_back();
        }
    }

    pub fn front_forward_by(&mut self, by: usize) -> &mut Self {
        assert!(self.front_rem().is_char_boundary(by));
        self.move_front_forward(by);
        self
    }

    pub fn front_forward(
        &mut self,
        pattern: impl IntoSearchable,
        loc: PatternLoc,
    ) -> Result<&mut Self, Failed> {
        let by = find_directional_offset(
            self.front_rem(),
            pattern.into_searchable(),
            loc,
            Direction::Forward,
        )
        .ok_or(Failed)?;
        self.move_front_forward(by);
        Ok(self)
    }

    pub fn front_forward_or(
        &mut self,
        pattern: impl IntoSearchable,
        loc: PatternLoc,
        fallback: FallBack,
    ) -> &mut Self {
        if self.front_forward(pattern, loc).is_err() {
            match fallback {
                FallBack::ToTheEnd => self.move_front_forward(self.front_rem().len()),
                FallBack::StayAtBeginning => (),
            }
        }
        self
    }

    pub fn back_backward(
        &mut self,
        pattern: impl IntoSearchable,
        loc: PatternLoc,
    ) -> Result<&mut Self, Failed> {
        let by = find_directional_offset(
            self.back_rem(),
            pattern.into_searchable(),
            loc,
            Direction::Backward,
        )
        .ok_or(Failed)?;
        self.move_back_backward(by);
        Ok(self)
    }

    pub fn front_backward(
        &mut self,
        pattern: impl IntoSearchable,
        loc: PatternLoc,
        inward_strategy: InwardStrategy,
    ) -> Result<&mut Self, Failed> {
        let view = match inward_strategy {
            InwardStrategy::CursorOnly => self.cursor(),
            InwardStrategy::WholeData => self.all_but_front_rem(),
        };
        let by = find_directional_offset(view, pattern.into_searchable(), loc, Direction::Backward)
            .ok_or(Failed)?;
        self.move_front_backward(by, inward_strategy);
        Ok(self)
    }

    fn back_forward_view(&self, inward_strategy: InwardStrategy) -> &str {
        match inward_strategy {
            InwardStrategy::CursorOnly => self.cursor(),
            InwardStrategy::WholeData => self.all_but_back_rem(),
        }
    }

    pub fn back_forward(
        &mut self,
        pattern: impl IntoSearchable,
        loc: PatternLoc,
        inward_strategy: InwardStrategy,
    ) -> Result<&mut Self, Failed> {
        let view = self.back_forward_view(inward_strategy);
        let by = find_directional_offset(view, pattern.into_searchable(), loc, Direction::Forward)
            .ok_or(Failed)?;
        self.move_back_forward(by, inward_strategy);
        Ok(self)
    }

    pub fn back_forward_by(&mut self, by: usize, inward_strategy: InwardStrategy) -> &mut Self {
        assert!(self.back_forward_view(inward_strategy).is_char_boundary(by));
        self.move_back_forward(by, inward_strategy);
        self
    }

    pub fn back_forward_or(
        &mut self,
        pattern: impl IntoSearchable,
        loc: PatternLoc,
        inward_strategy: InwardStrategy,
        fallback: FallBack,
    ) -> &mut Self {
        if self.back_forward(pattern, loc, inward_strategy).is_err() {
            match fallback {
                FallBack::ToTheEnd => self.move_back_forward(
                    self.back_forward_view(inward_strategy).len(),
                    inward_strategy,
                ),
                FallBack::StayAtBeginning => (),
            }
        }
        self
    }

    pub fn step(self, mut f: impl FnMut(&mut Self) -> Result<&mut Self,Failed>) -> impl Iterator<Item = &'a str> {
        let mut state = self;
        std::iter::from_fn(move || {
            state.back_to_front();
            f(&mut state).ok()?;
            Some(state.cursor())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_directional_offset_first_excluded_forward() {
        // Find first occurrence, return offset to start of match
        assert_eq!(
            find_directional_offset(
                "hello world",
                "world",
                PatternLoc::FirstExcluded,
                Direction::Forward
            ),
            Some(6)
        );
        assert_eq!(
            find_directional_offset(
                "abcabc",
                "abc",
                PatternLoc::FirstExcluded,
                Direction::Forward
            ),
            Some(0)
        );
        assert_eq!(
            find_directional_offset(
                "hello world",
                "xyz",
                PatternLoc::FirstExcluded,
                Direction::Forward
            ),
            None
        );
        assert_eq!(
            find_directional_offset(
                "foo bar baz",
                " ",
                PatternLoc::FirstExcluded,
                Direction::Forward
            ),
            Some(3)
        );
    }

    #[test]
    fn test_find_directional_offset_first_excluded_backward() {
        // Find last occurrence from the end, return offset excluding the match from end
        assert_eq!(
            find_directional_offset(
                "hello world",
                "o",
                PatternLoc::FirstExcluded,
                Direction::Backward
            ),
            Some(3)
        );
        assert_eq!(
            find_directional_offset(
                "abcabc",
                "abc",
                PatternLoc::FirstExcluded,
                Direction::Backward
            ),
            Some(0)
        );
        assert_eq!(
            find_directional_offset(
                "hello world",
                "xyz",
                PatternLoc::FirstExcluded,
                Direction::Backward
            ),
            None
        );
        assert_eq!(
            find_directional_offset(
                "foo bar baz",
                " ",
                PatternLoc::FirstExcluded,
                Direction::Backward
            ),
            Some(3)
        );
    }

    #[test]
    fn test_find_directional_offset_first_included_forward() {
        // Find first occurrence, return offset to end of match
        assert_eq!(
            find_directional_offset(
                "hello world",
                "world",
                PatternLoc::FirstIncluded,
                Direction::Forward
            ),
            Some(11)
        );
        assert_eq!(
            find_directional_offset(
                "abcabc",
                "abc",
                PatternLoc::FirstIncluded,
                Direction::Forward
            ),
            Some(3)
        );
        assert_eq!(
            find_directional_offset(
                "hello world",
                "xyz",
                PatternLoc::FirstIncluded,
                Direction::Forward
            ),
            None
        );
        assert_eq!(
            find_directional_offset(
                "foo bar baz",
                " ",
                PatternLoc::FirstIncluded,
                Direction::Forward
            ),
            Some(4)
        );
    }

    #[test]
    fn test_find_directional_offset_first_included_backward() {
        // Find last occurrence from the end, return offset including the match from end
        assert_eq!(
            find_directional_offset(
                "hello world",
                "o",
                PatternLoc::FirstIncluded,
                Direction::Backward
            ),
            Some(4)
        );
        assert_eq!(
            find_directional_offset(
                "abcabc",
                "abc",
                PatternLoc::FirstIncluded,
                Direction::Backward
            ),
            Some(3)
        );
        assert_eq!(
            find_directional_offset(
                "hello world",
                "xyz",
                PatternLoc::FirstIncluded,
                Direction::Backward
            ),
            None
        );
        assert_eq!(
            find_directional_offset(
                "foo bar baz!",
                " ",
                PatternLoc::FirstIncluded,
                Direction::Backward
            ),
            Some(5)
        );
    }

    #[test]
    fn test_find_directional_offset_beginning_many_forward() {
        // Trim from start, return offset after trimming
        assert_eq!(
            find_directional_offset(
                "   hello",
                " ",
                PatternLoc::BeginningMany,
                Direction::Forward
            ),
            Some(3)
        );
        assert_eq!(
            find_directional_offset("aaabbb", "a", PatternLoc::BeginningMany, Direction::Forward),
            Some(3)
        );
        assert_eq!(
            find_directional_offset("hello", "x", PatternLoc::BeginningMany, Direction::Forward),
            Some(0)
        );
        assert_eq!(
            find_directional_offset("aaaa", "a", PatternLoc::BeginningMany, Direction::Forward),
            Some(4)
        );
    }

    #[test]
    fn test_find_directional_offset_beginning_many_backward() {
        // Trim from end, return offset from end after trimming
        assert_eq!(
            find_directional_offset(
                "hello   ",
                " ",
                PatternLoc::BeginningMany,
                Direction::Backward
            ),
            Some(3)
        );
        assert_eq!(
            find_directional_offset(
                "aaabbb",
                "b",
                PatternLoc::BeginningMany,
                Direction::Backward
            ),
            Some(3)
        );
        assert_eq!(
            find_directional_offset("hello", "x", PatternLoc::BeginningMany, Direction::Backward),
            Some(0)
        );
        assert_eq!(
            find_directional_offset("bbbb", "b", PatternLoc::BeginningMany, Direction::Backward),
            Some(4)
        );
    }

    #[test]
    fn test_find_directional_offset_beginning_once_forward() {
        // Strip prefix once, return offset to end of match
        assert_eq!(
            find_directional_offset(
                "abcdef",
                "abc",
                PatternLoc::BeginningOnce,
                Direction::Forward
            ),
            Some(3) // strips "abc", rem="def" (len 3), offset: 6-3=3
        );
        assert_eq!(
            find_directional_offset(
                "hello world",
                "hello",
                PatternLoc::BeginningOnce,
                Direction::Forward
            ),
            Some(5) // strips "hello", rem=" world" (len 6), offset: 11-6=5
        );
        assert_eq!(
            find_directional_offset(
                "hello world",
                "world",
                PatternLoc::BeginningOnce,
                Direction::Forward
            ),
            None // "world" is not a prefix
        );
        assert_eq!(
            find_directional_offset(
                "test",
                "test",
                PatternLoc::BeginningOnce,
                Direction::Forward
            ),
            Some(4) // exact match, rem="" (len 0), offset: 4-0=4
        );
        assert_eq!(
            find_directional_offset(
                "abcabc",
                "abc",
                PatternLoc::BeginningOnce,
                Direction::Forward
            ),
            Some(3) // only strips first occurrence at beginning
        );
        assert_eq!(
            find_directional_offset("", "x", PatternLoc::BeginningOnce, Direction::Forward),
            None // empty string has no prefix
        );
    }

    #[test]
    fn test_find_directional_offset_beginning_once_backward() {
        // Strip suffix once, return offset from end to start of match
        assert_eq!(
            find_directional_offset(
                "abcdef",
                "def",
                PatternLoc::BeginningOnce,
                Direction::Backward
            ),
            Some(3) // strips "def", rem="abc" (len 3), offset from end: 6-3=3
        );
        assert_eq!(
            find_directional_offset(
                "hello world",
                "world",
                PatternLoc::BeginningOnce,
                Direction::Backward
            ),
            Some(6) // strips "world", rem="hello " (len 6), offset from end: 11-6=5
        );
        assert_eq!(
            find_directional_offset(
                "hello world",
                "hello",
                PatternLoc::BeginningOnce,
                Direction::Backward
            ),
            None // "hello" is not a suffix
        );
        assert_eq!(
            find_directional_offset(
                "test",
                "test",
                PatternLoc::BeginningOnce,
                Direction::Backward
            ),
            Some(4) // exact match, rem="" (len 0), offset: 4-0=4
        );
        assert_eq!(
            find_directional_offset(
                "abcabc",
                "abc",
                PatternLoc::BeginningOnce,
                Direction::Backward
            ),
            Some(3) // only strips last occurrence at end, rem="abc" (len 3)
        );
        assert_eq!(
            find_directional_offset("", "x", PatternLoc::BeginningOnce, Direction::Backward),
            None // empty string has no suffix
        );
    }

    #[test]
    fn test_find_directional_offset_last_excluded_forward() {
        // Find last occurrence, return offset to start of match
        assert_eq!(
            find_directional_offset(
                "hello world",
                "o",
                PatternLoc::LastExcluded,
                Direction::Forward
            ),
            Some(7)
        );
        assert_eq!(
            find_directional_offset(
                "abcabc",
                "abc",
                PatternLoc::LastExcluded,
                Direction::Forward
            ),
            Some(3)
        );
        assert_eq!(
            find_directional_offset(
                "hello world",
                "xyz",
                PatternLoc::LastExcluded,
                Direction::Forward
            ),
            None
        );
        assert_eq!(
            find_directional_offset(
                "foo bar baz",
                " ",
                PatternLoc::LastExcluded,
                Direction::Forward
            ),
            Some(7)
        );
    }

    #[test]
    fn test_find_directional_offset_last_excluded_backward() {
        // Same as FirstExcluded Backward based on the code
        assert_eq!(
            find_directional_offset(
                "hello world",
                "o",
                PatternLoc::LastExcluded,
                Direction::Backward
            ),
            Some(3) // Same logic as FirstExcluded Backward
        );
        assert_eq!(
            find_directional_offset(
                "abcabc",
                "abc",
                PatternLoc::LastExcluded,
                Direction::Backward
            ),
            Some(0) // Same logic as FirstExcluded Backward
        );
        assert_eq!(
            find_directional_offset(
                "hello world",
                "xyz",
                PatternLoc::LastExcluded,
                Direction::Backward
            ),
            None
        );
    }

    #[test]
    fn test_find_directional_offset_end_of_last_forward() {
        // Find last occurrence, return offset to end of match
        assert_eq!(
            find_directional_offset(
                "hello world",
                "o",
                PatternLoc::EndOfLast,
                Direction::Forward
            ),
            Some(8)
        );
        assert_eq!(
            find_directional_offset("abcabc", "abc", PatternLoc::EndOfLast, Direction::Forward),
            Some(6)
        );
        assert_eq!(
            find_directional_offset(
                "hello world",
                "xyz",
                PatternLoc::EndOfLast,
                Direction::Forward
            ),
            None
        );
        assert_eq!(
            find_directional_offset(
                "foo bar baz",
                " ",
                PatternLoc::EndOfLast,
                Direction::Forward
            ),
            Some(8)
        );
    }

    #[test]
    fn test_find_directional_offset_end_of_last_backward() {
        // Find first occurrence, return offset from end
        assert_eq!(
            find_directional_offset(
                "hello world",
                "world",
                PatternLoc::EndOfLast,
                Direction::Backward
            ),
            Some(5)
        );
        assert_eq!(
            find_directional_offset("abcabc", "abc", PatternLoc::EndOfLast, Direction::Backward),
            Some(6)
        );
        assert_eq!(
            find_directional_offset(
                "hello world",
                "xyz",
                PatternLoc::EndOfLast,
                Direction::Backward
            ),
            None
        );
        assert_eq!(
            find_directional_offset(
                "foo bar baz",
                " ",
                PatternLoc::EndOfLast,
                Direction::Backward
            ),
            Some(8)
        );
    }

    #[test]
    fn test_find_directional_offset_start_of_suffix_many_forward() {
        // Trim from end, return offset where suffix starts
        assert_eq!(
            find_directional_offset(
                "hello   ",
                " ",
                PatternLoc::StartOfSuffixMany,
                Direction::Forward
            ),
            Some(5)
        );
        assert_eq!(
            find_directional_offset(
                "aaabbb",
                "b",
                PatternLoc::StartOfSuffixMany,
                Direction::Forward
            ),
            Some(3)
        );
        assert_eq!(
            find_directional_offset(
                "hello",
                "x",
                PatternLoc::StartOfSuffixMany,
                Direction::Forward
            ),
            Some(5)
        );
        assert_eq!(
            find_directional_offset(
                "bbbb",
                "b",
                PatternLoc::StartOfSuffixMany,
                Direction::Forward
            ),
            Some(0)
        );
    }

    #[test]
    fn test_find_directional_offset_start_of_suffix_many_backward() {
        // Trim from start, return length of trimmed string
        assert_eq!(
            find_directional_offset(
                "   hello",
                " ",
                PatternLoc::StartOfSuffixMany,
                Direction::Backward
            ),
            Some(5) // trim_start_matches "   " leaves "hello" (length 5)
        );
        assert_eq!(
            find_directional_offset(
                "aaabbb",
                "a",
                PatternLoc::StartOfSuffixMany,
                Direction::Backward
            ),
            Some(3) // trim_start_matches "aaa" leaves "bbb" (length 3)
        );
        assert_eq!(
            find_directional_offset(
                "hello",
                "x",
                PatternLoc::StartOfSuffixMany,
                Direction::Backward
            ),
            Some(5) // no match, returns full length
        );
        assert_eq!(
            find_directional_offset(
                "aaaa",
                "a",
                PatternLoc::StartOfSuffixMany,
                Direction::Backward
            ),
            Some(0) // all trimmed, empty string (length 0)
        );
    }

    #[test]
    fn test_find_directional_offset_empty_string() {
        assert_eq!(
            find_directional_offset("", "x", PatternLoc::FirstExcluded, Direction::Forward),
            None
        );
        assert_eq!(
            find_directional_offset("", " ", PatternLoc::BeginningMany, Direction::Forward),
            Some(0)
        );
        assert_eq!(
            find_directional_offset("", " ", PatternLoc::StartOfSuffixMany, Direction::Forward),
            Some(0)
        );
    }

    #[test]
    fn test_find_directional_offset_single_char() {
        assert_eq!(
            find_directional_offset("a", "a", PatternLoc::FirstExcluded, Direction::Forward),
            Some(0)
        );
        assert_eq!(
            find_directional_offset("a", "a", PatternLoc::FirstIncluded, Direction::Forward),
            Some(1)
        );
        assert_eq!(
            find_directional_offset("a", "b", PatternLoc::FirstExcluded, Direction::Forward),
            None
        );
    }
}

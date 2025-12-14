// Not copy to prevent logic errors
#[derive(Clone)]
pub struct ParseCursor<'a> {
    data: &'a str,
    cursor_range: Range<usize>,
}

#[derive(Debug)]
pub struct Failed;

use std::ops::Range;

use stable_string_patterns_method::{DoubleEndedSearchable, Searchable, StrPatternExt};

#[derive(Debug, Clone, Copy)]
enum Direction {
    Forward,
    Backward,
}

pub enum PartternLoc {
    FirstExcluded,
    FirstIncluded,
    BeginningMany,
    LastExcluded,
    EndOfLast,
    StartOfSuffixMany,
}

fn find_directional_offset(
    haystack: &str,
    pattern: impl Searchable,
    loc: PartternLoc,
    direction: Direction,
) -> Option<usize> {
    let offset_from_end = |offset_from_beg| haystack.len() - offset_from_beg;
    let offset_from_start = |offset_from_end| haystack.len() - offset_from_end;
    let offset_of_sub_end = |(offset_of_sub, sub): (usize, &str)| offset_of_sub + sub.len();
    match (loc, direction) {
        (PartternLoc::FirstExcluded, Direction::Forward) => haystack.find_(pattern),
        (PartternLoc::FirstExcluded, Direction::Backward) => haystack
            .rmatch_indices_(pattern)
            .next()
            .map(offset_of_sub_end)
            .map(offset_from_end),
        (PartternLoc::FirstIncluded, Direction::Forward) => haystack
            .match_indices_(pattern)
            .next()
            .map(offset_of_sub_end),
        (PartternLoc::FirstIncluded, Direction::Backward) => {
            haystack.rfind_(pattern).map(offset_from_end)
        }
        (PartternLoc::BeginningMany, Direction::Forward) => {
            let rem = haystack.trim_start_matches_(pattern);
            Some(offset_from_start(rem.len()))
        }
        (PartternLoc::BeginningMany, Direction::Backward) => {
            let rem = haystack.trim_end_matches_(pattern);
            Some(offset_from_end(rem.len()))
        }
        (PartternLoc::LastExcluded, Direction::Forward) => haystack.rfind_(pattern),
        (PartternLoc::LastExcluded, Direction::Backward) => haystack
            .rmatch_indices_(pattern)
            .next()
            .map(offset_of_sub_end)
            .map(offset_from_end),
        (PartternLoc::EndOfLast, Direction::Forward) => haystack
            .rmatch_indices_(pattern)
            .next()
            .map(offset_of_sub_end),
        (PartternLoc::EndOfLast, Direction::Backward) => {
            haystack.find_(pattern).map(offset_from_end)
        }
        (PartternLoc::StartOfSuffixMany, Direction::Forward) => {
            Some(haystack.trim_end_matches_(pattern).len())
        }
        (PartternLoc::StartOfSuffixMany, Direction::Backward) => {
            Some(haystack.trim_start_matches_(pattern).len())
        }
    }
}

#[derive(Clone, Copy)]
pub enum InwardStrategy {
    CursorOnly,
    WholeData,
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

    pub fn reset_to_back(&mut self) {
        self.cursor_range.end = self.cursor_range.start;
    }

    pub fn reset_to_front(&mut self) {
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
        debug_assert!(self.check_invariants());
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
        debug_assert!(self.check_invariants());
        unsafe { self.data.get_unchecked(..self.cursor_range.start) }
    }

    #[cfg(not(feature = "use-unsafe"))]
    pub fn front_rem(&self) -> &'a str {
        &self.data[self.cursor_range.end..]
    }

    #[cfg(feature = "use-unsafe")]
    pub fn front_rem(&self) -> &'a str {
        debug_assert!(self.check_invariants());
        unsafe { self.data.get_unchecked(self.cursor_range.end..) }
    }

    #[cfg(not(feature = "use-unsafe"))]
    pub fn all_but_front_rem(&self) -> &'a str {
        &self.data[..self.cursor_range.end]
    }

    #[cfg(feature = "use-unsafe")]
    pub fn all_but_front_rem(&self) -> &'a str {
        debug_assert!(self.check_invariants());
        unsafe { self.data.get_unchecked(..self.cursor_range.end) }
    }

    #[cfg(not(feature = "use-unsafe"))]
    pub fn all_but_back_rem(&self) -> &'a str {
        &self.data[self.cursor_range.start..]
    }

    #[cfg(feature = "use-unsafe")]
    pub fn all_but_back_rem(&self) -> &'a str {
        debug_assert!(self.check_invariants());
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

    // #[cfg(feature = "use-unsafe")]
    // pub fn remainder(&self) -> &'a str {
    //     debug_assert!(self.data.is_char_boundary(self.cursor_byte_len));
    //     unsafe { self.data.get_unchecked(self.cursor_byte_len..) }
    // }

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
            self.reset_to_front();
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
            self.cursor_range.end += by;
            self.reset_to_back();
        }
    }

    pub fn front_forwardl(
        &mut self,
        pattern: impl Searchable,
        loc: PartternLoc,
    ) -> Result<usize, Failed> {
        let by = find_directional_offset(self.front_rem(), pattern, loc, Direction::Forward)
            .ok_or(Failed)?;
        self.move_front_forward(by);
        Ok(by)
    }

    pub fn back_backward(
        &mut self,
        pattern: impl Searchable,
        loc: PartternLoc,
    ) -> Result<usize, Failed> {
        let by = find_directional_offset(self.back_rem(), pattern, loc, Direction::Backward)
            .ok_or(Failed)?;
        self.move_back_backward(by);
        Ok(by)
    }

    pub fn front_backward(
        &mut self,
        pattern: impl Searchable,
        loc: PartternLoc,
        inward_strategy: InwardStrategy,
    ) -> Result<usize, Failed> {
        let view = match inward_strategy {
            InwardStrategy::CursorOnly => self.cursor(),
            InwardStrategy::WholeData => self.all_but_front_rem(),
        };
        let by = find_directional_offset(view, pattern, loc, Direction::Backward).ok_or(Failed)?;
        self.move_front_backward(by, inward_strategy);
        Ok(by)
    }

    pub fn back_forward(
        &mut self,
        pattern: impl Searchable,
        loc: PartternLoc,
        inward_strategy: InwardStrategy,
    ) -> Result<usize, Failed> {
        let view = match inward_strategy {
            InwardStrategy::CursorOnly => self.cursor(),
            InwardStrategy::WholeData => self.all_but_back_rem(),
        };
        let by = find_directional_offset(view, pattern, loc, Direction::Forward).ok_or(Failed)?;
        self.move_back_forward(by, inward_strategy);
        Ok(by)
    }
}

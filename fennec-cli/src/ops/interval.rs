use std::ops::Sub;

/// Half-open interval.
///
/// TODO: could become a wrapper around [`std::range::Range`].
/// TODO: some usages may likely be replaced with [`std::range::Range`] directly.
#[must_use]
#[derive(Copy, Clone, PartialEq, Eq, derive_more::Debug)]
#[debug("{start:?}..{end:?}")]
pub struct Interval<Index> {
    start: Index,
    end: Index,
}

impl<Index> From<core::ops::Range<Index>> for Interval<Index> {
    fn from(range: core::ops::Range<Index>) -> Self {
        Self { start: range.start, end: range.end }
    }
}

impl<Index> Interval<Index> {
    pub fn new(start: Index, end: Index) -> Self
    where
        Index: PartialOrd,
    {
        assert!(start <= end);
        Self { start, end }
    }

    #[must_use]
    pub const fn start(self) -> Index
    where
        Index: Copy,
    {
        self.start
    }

    #[must_use]
    pub const fn end(self) -> Index
    where
        Index: Copy,
    {
        self.end
    }

    /// Returns [`true`] if the interval ends earlier than the other interval starts.
    #[must_use]
    pub fn is_earlier_than(self, other: Self) -> bool
    where
        Index: Copy + PartialOrd,
    {
        self.end <= other.start
    }

    /// Returns [`true`] if the interval fully contains the other interval.
    #[must_use]
    #[expect(clippy::needless_pass_by_value)]
    pub fn contains(self, other: Self) -> bool
    where
        Index: PartialOrd,
    {
        (self.start <= other.start) && (other.end <= self.end)
    }

    /// Restrict the interval start to the specified index.
    pub fn clamp_start_to(mut self, index: Index) -> Self
    where
        Index: Copy + PartialOrd,
    {
        if index > self.end {
            self.start = self.end;
        } else if index > self.start {
            self.start = index;
        }
        self
    }

    /// Interval duration.
    pub fn duration(self) -> <Index as Sub>::Output
    where
        Index: Sub,
    {
        self.end - self.start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_clamp_start() {
        let interval = Interval { start: 1, end: 10 };

        // Target before the interval does not change the interval:
        assert_eq!(interval.clamp_start_to(0).start, 1);

        // Target within the interval clamps to the target:
        assert_eq!(interval.clamp_start_to(5).start, 5);

        // Target after the interval clamps to the end:
        assert_eq!(interval.clamp_start_to(12).start, 10);
    }
}

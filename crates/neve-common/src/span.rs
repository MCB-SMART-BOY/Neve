//! Source code span and position tracking.

use std::fmt;

/// A byte position in source code.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct BytePos(pub u32);

impl BytePos {
    pub const ZERO: BytePos = BytePos(0);

    pub fn offset(self, offset: u32) -> BytePos {
        BytePos(self.0 + offset)
    }
}

impl fmt::Debug for BytePos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BytePos({})", self.0)
    }
}

impl From<usize> for BytePos {
    fn from(pos: usize) -> Self {
        BytePos(pos as u32)
    }
}

impl From<BytePos> for usize {
    fn from(pos: BytePos) -> Self {
        pos.0 as usize
    }
}

/// A span representing a range in source code.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub start: BytePos,
    pub end: BytePos,
}

impl Span {
    pub const DUMMY: Span = Span {
        start: BytePos::ZERO,
        end: BytePos::ZERO,
    };

    pub fn new(start: BytePos, end: BytePos) -> Self {
        Span { start, end }
    }

    pub fn from_usize(start: usize, end: usize) -> Self {
        Span {
            start: BytePos::from(start),
            end: BytePos::from(end),
        }
    }

    /// Create a span that covers both `self` and `other`.
    pub fn merge(self, other: Span) -> Span {
        Span {
            start: std::cmp::min(self.start, other.start),
            end: std::cmp::max(self.end, other.end),
        }
    }

    /// Returns the length of this span in bytes.
    pub fn len(&self) -> usize {
        (self.end.0 - self.start.0) as usize
    }

    /// Returns true if this span has zero length.
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Returns the byte range for this span.
    pub fn range(&self) -> std::ops::Range<usize> {
        usize::from(self.start)..usize::from(self.end)
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start.0, self.end.0)
    }
}


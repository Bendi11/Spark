
/// Type used to index a source string, assumes text files do not exceed 4GB individually
pub type SourceIndex = u32;

/// Contiguous span of text referenced by start and end position in a source file
#[derive(Debug, Clone, Copy, PartialEq, Eq,)]
pub struct SourceSpan {
    start: SourceIndex,
    end: SourceIndex,
}

impl SourceSpan {
    /// Create a new source span from inclusive lower bound and exclusive upper bound
    pub const fn new(start: SourceIndex, end: SourceIndex) -> Self {
        debug_assert!(start <= end);

        Self {
            start,
            end,
        }
    }
}

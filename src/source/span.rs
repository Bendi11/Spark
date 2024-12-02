
/// Contiguous span of text referenced by start and end position in a source file
#[derive(Debug, Clone, Copy, PartialEq, Eq,)]
pub struct SourceSpan {
    start: u32,
    end: u32,
}

use std::str::CharIndices;

pub mod tok;

/// Lexer producing a stream of tokens with span data from a source file
pub struct Lexer<'src> {
    text: CharIndices<'src>,
}

impl<'src> Lexer<'src> {
    /// Create a new token stream from the given source text
    pub fn new(text: &'src str) -> Self {
        Self {
            text: text.char_indices(),
        }
    }
}

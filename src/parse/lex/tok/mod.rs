use text::TextLiteral;

use crate::{shared::operator::Operator, source::span::SourceSpan};

pub mod text;
pub mod number;

pub struct Token {
    pub span: SourceSpan,
    pub kind: TokenKind,
}

pub enum TokenKind {
    CharLiteral(TextLiteral),
    StringLiteral(TextLiteral),
    NumberLiteral(SourceSpan),
    Identifier,
    Operator(Operator),
}

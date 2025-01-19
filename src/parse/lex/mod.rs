use std::{iter::Peekable, str::CharIndices};

pub mod tok;
pub mod chars;

use tok::{number::NumberLiteral, text::TextLiteral};
pub use tok::{Token, TokenKind};

use crate::{shared::operator::{arith::ArithmeticOperator, Operator}, source::span::{SourceIndex, SourceSpan}};

/// Lexer producing a stream of tokens with span data from a source file
pub struct Lexer<'src> {
    text: Peekable<CharIndices<'src>>,
    idx: SourceIndex,
}

impl<'src> Lexer<'src> {
    /// Create a new token stream from the given source text
    pub fn new(text: &'src str) -> Self {
        Self {
            text: text.char_indices().peekable(),
            idx: 0,
        }
    }
    
    /// Consume a character from the input stream
    fn char(&mut self) -> Option<(SourceIndex, char)> {
        match self.text.next() {
            Some((idx, ch)) => {
                self.idx += 1;
                Some((idx as SourceIndex, ch))
            },
            None => None,
        }
    }

    fn peek_char(&mut self) -> Option<char> {
        self.text.peek().map(|(_, c)| c).copied()
    }
    
    /// Consume characters starting from the first content character of a text literal until the
    /// given unescaped terminator is reached
    fn text_literal(&mut self, terminator: char) -> (TextLiteral, SourceIndex) {
        let start = self.idx;
        let mut end = self.idx;

        loop {
            let Some(next) = self.char() else {
                break (
                    TextLiteral { contents: SourceSpan::new(start, self.idx) },
                    self.idx,
                )
            };

            match next.1 {
                '\\' => {
                    self.char();
                    end = self.idx;
                },
                c if c == terminator => break (
                    TextLiteral { contents: SourceSpan::new(start, end) },
                    self.idx
                ),
                _ => {
                    end += 1;
                }
            }
        }
    }

    fn number_literal(&mut self) -> Option<NumberLiteral> {
        
    }
    
    /// Consume characters from the input stream and produce the next token
    fn token(&mut self) -> Option<Token> {
        let first = loop {
            let next = self.char()?;
            if !next.1.is_ascii_whitespace() {
                break next
            }
        };

        Some(match first.1 {
            d @ ('\'' | '"') => {
                let (contents, end) = self.text_literal(d);
                
                Token {
                    span: SourceSpan::new(first.0, end),
                    kind: match d {
                        '\'' => TokenKind::CharLiteral(contents),
                        '"' => TokenKind::StringLiteral(contents),
                    }
                }
            },
            '-' => match self.peek_char() {
                Some(c) if c.is_ascii_digit() || c == '.' => {
                    
                },
                _ => Token {
                    span: SourceSpan::new(first.0, first.0),
                    kind: TokenKind::Operator(Operator::Arithmetic(ArithmeticOperator::Sub))
                }
            },
            '+' | '*' | '/' => Token {
                span: SourceSpan::new(first.0, first.0),
                kind: TokenKind::Operator(Operator::Arithmetic(match first.1 {
                    '+' => ArithmeticOperator::Add,
                    '*' => ArithmeticOperator::Mul,
                    '/' => ArithmeticOperator::Div,
                }))
            },
            d if d.is_ascii_digit() => {
            
            },
            c if chars::is_ident_first(c) => {

            },
        })
    }
}

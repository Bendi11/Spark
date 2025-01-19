

/// Check if the given character is valid for a continuation of an identifier, **not** the first
/// digit. See []
pub const fn is_ident_all(c: char) -> bool {
    c.is_ascii_alphabetic() || c.is_ascii_digit() || c == '_'
}

/// Check if the given character is valid as the first character in an identifier
pub const fn is_ident_first(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

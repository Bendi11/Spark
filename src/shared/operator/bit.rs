use std::fmt;



pub enum BitOperator {
    And,
    Or,
    Not,
    Xor,
    Shl,
    Shr,
}

impl fmt::Display for BitOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::And => "&",
                Self::Or => "|",
                Self::Not => "~",
                Self::Xor => "^",
                Self::Shl => ">>",
                Self::Shr => "<<"
            }
        )
    }
}

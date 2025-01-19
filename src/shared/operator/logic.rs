use std::fmt;



pub enum LogicOperator {
    And,
    Or,
    Not,
}

impl fmt::Display for LogicOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::And => "&&",
                Self::Or => "||",
                Self::Not => "!"
            }
        )
    }
}

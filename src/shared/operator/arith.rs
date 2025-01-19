use std::fmt;


pub enum ArithmeticOperator {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

impl fmt::Display for ArithmeticOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Add => '+',
                Self::Sub => '-',
                Self::Mul => '*',
                Self::Div => '/',
                Self::Mod => '%',
            }
        )
    }
}

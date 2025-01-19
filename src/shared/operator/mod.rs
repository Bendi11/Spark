use arith::ArithmeticOperator;
use bit::BitOperator;
use logic::LogicOperator;

pub mod bit;
pub mod logic;
pub mod arith;

pub enum Operator {
    Bit(BitOperator),
    Logic(LogicOperator),
    Arithmetic(ArithmeticOperator),
}

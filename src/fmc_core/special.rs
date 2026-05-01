use std::fmt::Display;

#[derive(Clone, PartialEq, Debug)]
pub enum Special {
    LogicOr,
    LogicAnd,
    LogicNot,
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessThanEqual,
    GreaterThanEqual,
    Addition,
    Subraction,
    Multiplication,
    Division,
    Modulo,
    IntCast,
}

impl Display for Special {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Special::LogicOr => write!(f, "||"),
            Special::LogicAnd => write!(f, "&&"),
            Special::LogicNot => write!(f, "!"),
            Special::Equal => write!(f, "=="),
            Special::NotEqual => write!(f, "!="),
            Special::LessThan => write!(f, "<"),
            Special::GreaterThan => write!(f, ">"),
            Special::LessThanEqual => write!(f, "<="),
            Special::GreaterThanEqual => write!(f, ">="),
            Special::Addition => write!(f, "+"),
            Special::Subraction => write!(f, "-"),
            Special::Multiplication => write!(f, "*"),
            Special::Division => write!(f, "/"),
            Special::Modulo => write!(f, "%"),
            Special::IntCast => write!(f, "int"),
        }
    }
}

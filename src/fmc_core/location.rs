use std::fmt::Display;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
/// Indicates the different stacks or streams for each effect
pub enum Location {
    Main,
    Input,
    Output,
    Rnd,
    RndBool,
    RndFloat,
    /// Cells can only have a depth of 1, enforced by semantics of the calculus rather than the machine
    Cell(String),
    Stack(String),
    Local(String),
}

impl Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Location::Main => write!(f, "λ"),
            Location::Input => write!(f, "in"),
            Location::Output => write!(f, "out"),
            Location::Rnd => write!(f, "rnd"),
            Location::RndBool => write!(f, "rnd_b"),
            Location::RndFloat => write!(f, "rnd_f"),
            Location::Cell(index) => write!(f, "{}", index),
            Location::Stack(index) => write!(f, "{}", index),
            Location::Local(index) => write!(f, "{}", index),
        }
    }
}

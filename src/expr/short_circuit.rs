use std::fmt::Display;

pub enum ShortCircuit<X> {
    Known(bool),
    Unknown(X),
}

impl<X> From<bool> for ShortCircuit<X> {
    fn from(value: bool) -> Self {
        ShortCircuit::Known(value)
    }
}

impl<X: Display> Display for ShortCircuit<X> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShortCircuit::Known(x) => write!(f, "known: {}", x),
            ShortCircuit::Unknown(x) => write!(f, "unknown: {}", x),
        }
    }
}

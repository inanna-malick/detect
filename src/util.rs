// req'd to derive some stuff
#[derive(Debug, Eq, PartialEq, Clone)]
pub(crate) enum Done {}

pub(crate) fn never<A>(_: &Done) -> A {
    unreachable!("never")
}

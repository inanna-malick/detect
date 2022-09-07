pub(crate) enum Done {}

pub(crate) fn never<A>(_: &Done) -> A {
    unreachable!("never")
}

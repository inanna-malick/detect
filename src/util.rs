/// uninhabited type, used to signify that something does not exist
/// provided typeclass instances never invoked but provided for
/// convenience
#[derive(Debug)]
pub(crate) enum Done {}

impl std::fmt::Display for Done {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unreachable!()
    }
}

#[derive(Debug)]
pub(crate) enum Done {}

impl std::fmt::Display for Done {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unreachable!()
    }
}
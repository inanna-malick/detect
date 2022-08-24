use std::io;
use futures::{future::BoxFuture};

pub trait FileLike {
    fn size(&self) -> usize;
    fn filetype(&self) -> FileType;
    // NOTE: may need to change type slightly to impl or w/e
    fn contents(&self) -> BoxFuture<io::Result<String>>;
}

#[derive(Eq, PartialEq)]
pub enum FileType {
    Binary,
    Exec,
    Symlink,
    Text,
}
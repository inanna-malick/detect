use futures::future::BoxFuture;
use std::io;

pub trait FileLike {
    fn contents(&self) -> BoxFuture<io::Result<String>>;
}

pub trait MetadataLike {
    fn size(&self) -> u64;
    fn filetype(&self) -> FileType;
}

#[derive(Eq, PartialEq)]
pub enum FileType {
    Binary,
    Exec,
    Symlink,
    File,
    Dir,
    Socket,
    Unknown, // idk if this needs to be here?
}

use std::io;
use std::result;
use failure::Error;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug, Fail)]
pub enum FlipperError {

    #[fail(display = "encountered an io error: {}", inner)]
    Io {
        inner: io::Error,
    },

    #[fail(display = "invocation error")]
    Invoke,

    #[fail(display = "module loading error")]
    Load,

    #[fail(display = "push error")]
    Push,

    #[fail(display = "pull error")]
    Pull,

    #[fail(display = "malloc error")]
    Malloc,

    #[fail(display = "free error")]
    Free,
}